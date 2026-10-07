//! Read-only Git object snapshots. Never checks out, resets, stashes, switches,
//! registers worktrees, or runs checkout filters/hooks.
use doctor_core::CommandStatus;
use doctor_runner::{execute, CapturedCommand, CommandSpec, RunnerError};
use std::{
    error::Error,
    ffi::OsString,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tempfile::TempDir;

const MAX_FILES: usize = 10_000;
const MAX_BLOB: u64 = 8 * 1024 * 1024;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;

#[derive(Debug)]
pub enum GitError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Runner(RunnerError),
    Command(Box<CapturedCommand>),
    InvalidRef(String),
    InvalidOutput(&'static str),
    UnsafeEnvironment(String),
    UnsupportedEntry(String),
    Limit(&'static str),
    Snapshot {
        path: PathBuf,
        source: Box<GitError>,
    },
    Cleanup {
        path: PathBuf,
        source: io::Error,
        primary: Option<Box<GitError>>,
    },
}
impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot access {}: {source}", path.display()),
            Self::Runner(e) => write!(f, "{e}"),
            Self::Command(c) => write!(f, "Git command failed ({:?}): {}", c.record.status, c.record.stderr.trim()),
            Self::InvalidRef(r) => write!(f, "invalid or non-commit Git ref: {r:?}"),
            Self::InvalidOutput(m) => write!(f, "invalid Git output: {m}"),
            Self::UnsafeEnvironment(name) => write!(f, "unset {name} to compare the explicitly selected repository"),
            Self::UnsupportedEntry(p) => write!(f, "unsupported Git tree entry: {p}; symlinks, submodules and unsafe paths are not materialized"),
            Self::Limit(m) => write!(f, "Git snapshot limit exceeded: {m}"),
            Self::Snapshot { path, source } => write!(f, "snapshot {} failed and was cleaned: {source}", path.display()),
            Self::Cleanup { path, source, primary } => {
                write!(f, "cannot clean temporary snapshot {}: {source}", path.display())?;
                if let Some(e) = primary { write!(f, "; original error: {e}")?; }
                Ok(())
            }
        }
    }
}
impl Error for GitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::Cleanup { source, .. } => Some(source),
            Self::Runner(e) => Some(e),
            Self::Snapshot { source, .. } => Some(source),
            _ => None,
        }
    }
}
fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn io_error(path: &Path, source: io::Error) -> GitError {
    GitError::Io {
        path: path.into(),
        source,
    }
}

pub struct GitRepository {
    root: PathBuf,
    timeout: Duration,
}
impl GitRepository {
    pub fn open(path: &Path, timeout: Duration) -> Result<Self, GitError> {
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "GIT_NAMESPACE",
        ] {
            if std::env::var_os(name).is_some() {
                return Err(GitError::UnsafeEnvironment(name.into()));
            }
        }
        let root = fs::canonicalize(path).map_err(|e| io_error(path, e))?;
        let mut repository = Self { root, timeout };
        let output = repository.command(&["rev-parse".into(), "--show-toplevel".into()])?;
        let bytes = output
            .stdout_bytes
            .strip_suffix(b"\n")
            .unwrap_or(&output.stdout_bytes);
        let path = std::str::from_utf8(bytes)
            .map_err(|_| GitError::InvalidOutput("repository path is not UTF-8"))?;
        repository.root = fs::canonicalize(path).map_err(|e| io_error(Path::new(path), e))?;
        // Partial clones can invoke remote helpers while reading missing objects.
        match repository.command(&[
            "config".into(),
            "--get-regexp".into(),
            "^(extensions\\.partialclone|remote\\..*\\.promisor)$".into(),
        ]) {
            Ok(_) => {
                return Err(GitError::UnsupportedEntry(
                    "partial/promisor repository configuration".into(),
                ))
            }
            Err(GitError::Command(c)) if c.record.status == (CommandStatus::Exited { code: 1 }) => {
            }
            Err(e) => return Err(e),
        }
        Ok(repository)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn command(&self, args: &[OsString]) -> Result<CapturedCommand, GitError> {
        // Verified Git plumbing docs: raw cat-file blobs do not invoke filters.
        // Optional-lock suppression prevents status from refreshing the index.
        let mut safe_args: Vec<OsString> = [
            "--no-pager",
            "--no-optional-locks",
            "--no-replace-objects",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        safe_args.extend_from_slice(args);
        let capture = execute(&CommandSpec {
            program: "git".into(),
            args: safe_args,
            working_directory: self.root.clone(),
            timeout: self.timeout,
        })
        .map_err(GitError::Runner)?;
        if capture.record.status != (CommandStatus::Exited { code: 0 }) {
            return Err(GitError::Command(Box::new(capture)));
        }
        Ok(capture)
    }
    pub fn is_dirty(&self) -> Result<bool, GitError> {
        // Compare raw bytes, never Git status or attribute-driven clean filters.
        let index = self.command(&["ls-files".into(), "--stage".into(), "-z".into()])?;
        let untracked = self.command(&[
            "ls-files".into(),
            "--others".into(),
            "--exclude-standard".into(),
            "-z".into(),
        ])?;
        if !untracked.stdout_bytes.is_empty() {
            return Ok(true);
        }
        let head = match self.resolve("HEAD") {
            Ok(head) => head,
            Err(GitError::InvalidRef(_)) => return Ok(true),
            Err(e) => return Err(e),
        };
        let tree = self.command(&[
            "ls-tree".into(),
            "-r".into(),
            "-z".into(),
            "--full-tree".into(),
            head.into(),
        ])?;
        let mut committed = std::collections::BTreeMap::new();
        for entry in tree
            .stdout_bytes
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
        {
            let (header, path) = entry
                .split_once_tab()
                .ok_or(GitError::InvalidOutput("tree separator"))?;
            let header =
                std::str::from_utf8(header).map_err(|_| GitError::InvalidOutput("tree header"))?;
            let fields: Vec<_> = header.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(GitError::InvalidOutput("tree fields"));
            }
            committed.insert(path.to_vec(), (fields[0].to_owned(), fields[2].to_owned()));
        }
        let mut indexed = std::collections::BTreeMap::new();
        for entry in index
            .stdout_bytes
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
        {
            let (header, bytes) = entry
                .split_once_tab()
                .ok_or(GitError::InvalidOutput("index separator"))?;
            let header =
                std::str::from_utf8(header).map_err(|_| GitError::InvalidOutput("index header"))?;
            let fields: Vec<_> = header.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(GitError::InvalidOutput("index fields"));
            }
            if fields[2] != "0" {
                return Ok(true);
            }
            indexed.insert(bytes.to_vec(), (fields[0].to_owned(), fields[1].to_owned()));
            let relative = Path::new(
                std::str::from_utf8(bytes)
                    .map_err(|_| GitError::InvalidOutput("index path is not UTF-8"))?,
            );
            if relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(GitError::InvalidOutput("unsafe index path"));
            }
            let mut path = self.root.clone();
            for component in relative.components() {
                path.push(component);
                match fs::symlink_metadata(&path) {
                    Ok(meta) if meta.file_type().is_symlink() => return Ok(true),
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
                    Err(e) => return Err(io_error(&path, e)),
                }
            }
            let meta = fs::symlink_metadata(&path).map_err(|e| io_error(&path, e))?;
            if !meta.is_file() || !matches!(fields[0], "100644" | "100755") {
                return Ok(true);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if (meta.permissions().mode() & 0o111 != 0) != (fields[0] == "100755") {
                    return Ok(true);
                }
            }
            let hash = self.command(&[
                "hash-object".into(),
                "--no-filters".into(),
                "--".into(),
                path.into_os_string(),
            ])?;
            if std::str::from_utf8(&hash.stdout_bytes)
                .map_err(|_| GitError::InvalidOutput("raw hash"))?
                .trim()
                != fields[1]
            {
                return Ok(true);
            }
        }
        Ok(indexed != committed)
    }
    pub fn resolve(&self, reference: &str) -> Result<String, GitError> {
        if reference.is_empty()
            || reference.len() > 4096
            || reference.starts_with('-')
            || reference.chars().any(char::is_control)
        {
            return Err(GitError::InvalidRef(reference.into()));
        }
        // ^{commit} requires a single commit (tags may peel); end-of-options
        // keeps revision text from becoming a Git option. No shell is involved.
        let output = match self.command(&[
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{reference}^{{commit}}").into(),
        ]) {
            Err(GitError::Command(c))
                if matches!(c.record.status, CommandStatus::Exited { .. }) =>
            {
                return Err(GitError::InvalidRef(reference.into()))
            }
            result => result?,
        };
        let value = std::str::from_utf8(&output.stdout_bytes)
            .map_err(|_| GitError::InvalidOutput("commit OID"))?
            .trim();
        if !oid(value) {
            return Err(GitError::InvalidOutput("expected one full commit OID"));
        }
        Ok(value.into())
    }
    pub fn snapshots(&self, from: &str, to: &str) -> Result<Snapshots, GitError> {
        // Validate both refs before allocating any temporary resource.
        let from_commit = self.resolve(from)?;
        let to_commit = self.resolve(to)?;
        let temporary_root = std::env::temp_dir();
        let canonical_temp =
            fs::canonicalize(&temporary_root).map_err(|e| io_error(&temporary_root, e))?;
        if canonical_temp.starts_with(&self.root) {
            return Err(GitError::UnsafeEnvironment(
                "TMPDIR/TEMP (temporary directory is inside the active worktree)".into(),
            ));
        }
        let directory = tempfile::Builder::new()
            .prefix("sortralis-git-")
            .tempdir()
            .map_err(|e| io_error(&std::env::temp_dir(), e))?;
        let from_path = directory.path().join("from");
        let to_path = directory.path().join("to");
        let result = self
            .materialize(&from_commit, &from_path)
            .and_then(|()| self.materialize(&to_commit, &to_path));
        if let Err(primary) = result {
            let path = directory.path().to_path_buf();
            return match directory.close() {
                Ok(()) => Err(GitError::Snapshot {
                    path,
                    source: Box::new(primary),
                }),
                Err(source) => Err(GitError::Cleanup {
                    path,
                    source,
                    primary: Some(Box::new(primary)),
                }),
            };
        }
        Ok(Snapshots {
            directory,
            from_path,
            to_path,
            from_commit,
            to_commit,
        })
    }
    fn materialize(&self, commit: &str, destination: &Path) -> Result<(), GitError> {
        let output = self.command(&[
            "ls-tree".into(),
            "-r".into(),
            "-l".into(),
            "-z".into(),
            "--full-tree".into(),
            commit.into(),
        ])?;
        fs::create_dir(destination).map_err(|e| io_error(destination, e))?;
        let mut total = 0u64;
        for (count, entry) in output
            .stdout_bytes
            .split(|b| *b == 0)
            .filter(|e| !e.is_empty())
            .enumerate()
        {
            if count >= MAX_FILES {
                return Err(GitError::Limit("10,000 entries per ref"));
            }
            let (header, path) = entry
                .split_once_tab()
                .ok_or(GitError::InvalidOutput("tree entry lacks path separator"))?;
            let path = std::str::from_utf8(path)
                .map_err(|_| GitError::InvalidOutput("tree path is not UTF-8"))?;
            let relative = Path::new(path);
            if relative.as_os_str().is_empty()
                || relative.components().any(|c| {
                    !matches!(c, Component::Normal(_)) || c.as_os_str().eq_ignore_ascii_case(".git")
                })
            {
                return Err(GitError::UnsupportedEntry(path.into()));
            }
            // Snapshot Cargo config must not select a target rustc/wrapper/alias.
            // Source-only diff omits Cargo and rustup tool-selection configuration.
            if relative.components().any(|c| {
                c.as_os_str() == ".cargo"
                    || c.as_os_str() == "rust-toolchain"
                    || c.as_os_str() == "rust-toolchain.toml"
            }) {
                continue;
            }
            let header =
                std::str::from_utf8(header).map_err(|_| GitError::InvalidOutput("tree header"))?;
            let fields: Vec<_> = header.split_whitespace().collect();
            if fields.len() != 4
                || !matches!(fields[0], "100644" | "100755")
                || fields[1] != "blob"
                || !oid(fields[2])
            {
                return Err(GitError::UnsupportedEntry(path.into()));
            }
            let size: u64 = fields[3]
                .parse()
                .map_err(|_| GitError::InvalidOutput("blob size"))?;
            if size > MAX_BLOB {
                return Err(GitError::Limit("8 MiB per blob"));
            }
            total = total
                .checked_add(size)
                .ok_or(GitError::Limit("size overflow"))?;
            if total > MAX_TOTAL {
                return Err(GitError::Limit("64 MiB per ref"));
            }
            let blob = self.command(&["cat-file".into(), "blob".into(), fields[2].into()])?;
            if blob.stdout_bytes.len() as u64 != size {
                return Err(GitError::InvalidOutput("blob size changed"));
            }
            let file = destination.join(relative);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
            }
            fs::write(&file, &blob.stdout_bytes).map_err(|e| io_error(&file, e))?;
        }
        Ok(())
    }
}
// Keep parsing binary -z output separate from UTF-8 paths.
trait SplitTab {
    fn split_once_tab(&self) -> Option<(&[u8], &[u8])>;
}
impl SplitTab for [u8] {
    fn split_once_tab(&self) -> Option<(&[u8], &[u8])> {
        let index = self.iter().position(|b| *b == b'\t')?;
        Some((&self[..index], &self[index + 1..]))
    }
}
pub struct Snapshots {
    directory: TempDir,
    from_path: PathBuf,
    to_path: PathBuf,
    pub from_commit: String,
    pub to_commit: String,
}
impl Snapshots {
    pub fn root(&self) -> &Path {
        self.directory.path()
    }
    pub fn from_path(&self) -> &Path {
        &self.from_path
    }
    pub fn to_path(&self) -> &Path {
        &self.to_path
    }
    /// Explicit close reports OS cleanup failures. Drop is a best-effort fallback
    /// on unwinding. SIGKILL/power loss may leave this unregistered OS temp folder.
    pub fn close(self) -> Result<(), GitError> {
        let path = self.directory.path().to_path_buf();
        self.directory.close().map_err(|source| GitError::Cleanup {
            path,
            source,
            primary: None,
        })
    }
}
