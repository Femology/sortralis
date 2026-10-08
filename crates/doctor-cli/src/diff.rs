//! Ref snapshots use Git objects; all analysis is outside the active worktree.
use crate::check::sdk_context;
use doctor_cargo::{analyze, CargoError};
use doctor_core::{git_diff::*, Config, ConfigError, DetectedPackage, Evidence, CONFIG_FILE_NAME};
use doctor_git::{GitError, GitRepository};
use doctor_source::{
    analyze_directory, inventory_interface_directory, inventory_storage_directory, SourceError,
    SourceOptions, TargetContext,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug)]
pub enum DiffError {
    Git(GitError),
    Cargo(CargoError),
    Source(SourceError),
    Config(ConfigError),
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Manifest {
        path: PathBuf,
        source: toml::de::Error,
    },
    Lock {
        path: PathBuf,
        source: toml::de::Error,
    },
    OutsideSnapshot(PathBuf),
    AnalysisSnapshot {
        path: PathBuf,
        source: Box<DiffError>,
    },
    Cleanup {
        source: GitError,
        primary: Option<Box<DiffError>>,
    },
}
impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Git(e) => write!(f, "{e}"),
            Self::Cargo(e) => write!(f, "{e}"),
            Self::Source(e) => write!(f, "{e}"),
            Self::Config(e) => write!(f, "{e}"),
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Manifest { path, source } => {
                write!(f, "invalid manifest {}: {source}", path.display())
            }
            Self::Lock { path, source } => {
                write!(f, "invalid lockfile {}: {source}", path.display())
            }
            Self::OutsideSnapshot(p) => {
                write!(f, "Cargo path escapes committed snapshot: {}", p.display())
            }
            Self::AnalysisSnapshot { path, source } => write!(
                f,
                "snapshot {} was cleaned after analysis failed: {source}",
                path.display()
            ),
            Self::Cleanup { source, primary } => {
                write!(f, "{source}")?;
                if let Some(e) = primary {
                    write!(f, "; analysis error: {e}")?;
                }
                Ok(())
            }
        }
    }
}
impl Error for DiffError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Git(e) => Some(e),
            Self::Cargo(e) => Some(e),
            Self::Source(e) => Some(e),
            Self::Config(e) => Some(e),
            Self::Io { source, .. } => Some(source),
            Self::Lock { source, .. } | Self::Manifest { source, .. } => Some(source),
            Self::Cleanup { source, .. } => Some(source),
            Self::AnalysisSnapshot { source, .. } => Some(source),
            _ => None,
        }
    }
}
impl DiffError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::AnalysisSnapshot { source, .. } => source.exit_code(),
            Self::Git(GitError::Snapshot { source, .. }) => git_exit_code(source),
            Self::Git(GitError::InvalidOutput(_)) => 4,
            Self::Git(GitError::Command(_) | GitError::Runner(_))
            | Self::Cargo(CargoError::MetadataFailed(_) | CargoError::Runner(_))
            | Self::Cleanup { .. } => 3,
            Self::Cargo(CargoError::InvalidMetadata { .. }) => 4,
            _ => 2,
        }
    }
}
fn git_exit_code(error: &GitError) -> u8 {
    match error {
        GitError::Snapshot { source, .. } => git_exit_code(source),
        GitError::InvalidOutput(_) => 4,
        GitError::Command(_) | GitError::Runner(_) | GitError::Cleanup { .. } => 3,
        _ => 2,
    }
}
fn relative(root: &Path, path: &Path) -> Result<PathBuf, DiffError> {
    let canonical = fs::canonicalize(path).map_err(|e| io_error(path, e))?;
    canonical
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| DiffError::OutsideSnapshot(path.into()))
}
fn io_error(path: &Path, source: io::Error) -> DiffError {
    DiffError::Io {
        path: path.into(),
        source,
    }
}
fn evidence_paths(evidence: &mut [Evidence], prefix: &Path) {
    for item in evidence {
        if let Some(path) = &mut item.path {
            *path = prefix.join(&*path);
        }
    }
}
fn manifests(directory: &Path, result: &mut Vec<PathBuf>) -> Result<(), DiffError> {
    if directory.join("Cargo.toml").is_file() {
        result.push(directory.join("Cargo.toml"));
        return Ok(());
    }
    for entry in fs::read_dir(directory).map_err(|e| io_error(directory, e))? {
        let entry = entry.map_err(|e| io_error(directory, e))?;
        let path = entry.path();
        if entry.file_type().map_err(|e| io_error(&path, e))?.is_dir()
            && ![
                "target",
                "vendor",
                "generated",
                ".git",
                ".cargo",
                "node_modules",
            ]
            .iter()
            .any(|name| entry.file_name() == *name)
        {
            manifests(&path, result)?;
        }
    }
    Ok(())
}
// Cargo's documented path dependencies, target paths and workspace membership
// must stay inside the snapshot before metadata is allowed to inspect them.
fn validate_manifest_paths(root: &Path, directory: &Path) -> Result<(), DiffError> {
    for entry in fs::read_dir(directory).map_err(|e| io_error(directory, e))? {
        let entry = entry.map_err(|e| io_error(directory, e))?;
        let path = entry.path();
        if entry.file_type().map_err(|e| io_error(&path, e))?.is_dir() {
            validate_manifest_paths(root, &path)?;
        } else if entry.file_name() == "Cargo.toml" {
            let text = fs::read_to_string(&path).map_err(|e| io_error(&path, e))?;
            let value: toml::Value =
                toml::from_str(&text).map_err(|source| DiffError::Manifest {
                    path: path.clone(),
                    source,
                })?;
            validate_paths(root, directory, &value)?;
        }
    }
    Ok(())
}
fn validate_paths(root: &Path, base: &Path, value: &toml::Value) -> Result<(), DiffError> {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                if key == "metadata" {
                    continue;
                }
                if matches!(
                    key.as_str(),
                    "path" | "workspace" | "members" | "default-members"
                ) {
                    let values: Vec<_> = match value {
                        toml::Value::String(s) => vec![s.as_str()],
                        toml::Value::Array(a) => a.iter().filter_map(toml::Value::as_str).collect(),
                        _ => vec![],
                    };
                    for path in values {
                        let relative = Path::new(path);
                        let mut normalized = base.to_path_buf();
                        for c in relative.components() {
                            match c {
                                std::path::Component::Normal(c) => normalized.push(c),
                                std::path::Component::CurDir => {}
                                std::path::Component::ParentDir if normalized != root => {
                                    normalized.pop();
                                }
                                _ => return Err(DiffError::OutsideSnapshot(base.join(relative))),
                            }
                        }
                        if !normalized.starts_with(root) {
                            return Err(DiffError::OutsideSnapshot(normalized));
                        }
                    }
                }
                validate_paths(root, base, value)?;
            }
        }
        toml::Value::Array(array) => {
            for item in array {
                validate_paths(root, base, item)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn prefixed_identity(prefix: &Path, identity: &str) -> String {
    if prefix.as_os_str().is_empty() {
        identity.into()
    } else {
        format!("{}/{}", prefix.display(), identity)
    }
}
#[derive(Deserialize)]
struct Lockfile {
    #[serde(default)]
    package: Vec<LockedPackage>,
}
#[derive(Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
}
fn locked_versions(root: &Path) -> Result<Vec<String>, DiffError> {
    let path = root.join("Cargo.lock");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(io_error(&path, e)),
    };
    // Cargo.lock's package/name/version entries are verified against Cargo's
    // official lockfile documentation and the repository's actual lockfile.
    let lock: Lockfile =
        toml::from_str(&text).map_err(|source| DiffError::Lock { path, source })?;
    let mut versions: Vec<_> = lock
        .package
        .into_iter()
        .filter(|p| p.name == "soroban-sdk")
        .map(|p| p.version)
        .collect();
    versions.sort();
    versions.dedup();
    Ok(versions)
}
fn excluded(path: &Path, exclude: &Path) -> bool {
    if exclude.components().count() == 1 {
        path.components()
            .any(|p| p.as_os_str() == exclude.as_os_str())
    } else {
        path.starts_with(exclude)
    }
}
fn ref_analysis(root: &Path, commit: &str) -> Result<RefAnalysis, DiffError> {
    // Cargo reports canonical absolute paths. Canonicalize the snapshot boundary
    // too so platform path aliases (for example macOS /var -> /private/var)
    // cannot produce a false OutsideSnapshot result.
    let canonical_root = fs::canonicalize(root).map_err(|e| io_error(root, e))?;
    let root = canonical_root.as_path();
    validate_manifest_paths(root, root)?;
    let config = Config::load_optional(root.join(CONFIG_FILE_NAME)).map_err(DiffError::Config)?;
    let root_options = SourceOptions {
        exclude: config.exclude.iter().map(PathBuf::from).collect(),
        ..SourceOptions::default()
    };
    root_options.validate().map_err(DiffError::Source)?;
    let mut paths = vec![];
    manifests(root, &mut paths)?;
    paths.sort();
    let mut packages = BTreeMap::<PathBuf, DetectedPackage>::new();
    let mut contracts = BTreeMap::new();
    for manifest in paths {
        if root_options
            .exclude
            .iter()
            .any(|e| relative(root, &manifest).is_ok_and(|p| excluded(&p, e)))
        {
            continue;
        }
        let cargo = analyze(&manifest, Duration::from_secs(30)).map_err(DiffError::Cargo)?;
        relative(root, &cargo.workspace_root)?;
        let locked = locked_versions(&cargo.workspace_root)?;
        for mut package in cargo.packages {
            package.manifest_path = relative(root, &package.manifest_path)?;
            packages.insert(package.manifest_path.clone(), package);
        }
        for candidate in cargo.contracts {
            let mut package = candidate.package;
            package.manifest_path = relative(root, &package.manifest_path)?;
            if contracts.contains_key(&package.manifest_path) {
                continue;
            }
            let sdk_requirements: Vec<_> = candidate
                .sdk_dependencies
                .iter()
                .map(|d| d.requirement.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let contexts: Vec<_> = sdk_requirements.iter().map(|r| sdk_context(r)).collect();
            let context = if contexts.iter().all(|c| *c == TargetContext::Sdk28) {
                TargetContext::Sdk28
            } else if contexts.iter().all(|c| *c == TargetContext::OtherSdk) {
                TargetContext::OtherSdk
            } else {
                TargetContext::Unknown
            };
            let mut model = PackageSnapshot {
                package,
                sdk_requirements,
                locked_sdk_versions: locked.clone(),
                rule_context: format!("{context:?}"),
                source: Default::default(),
                storage: Default::default(),
                findings: vec![],
            };
            let mut roots = candidate.source_roots;
            roots.sort();
            let roots: Vec<_> = roots
                .iter()
                .filter(|path| {
                    !roots
                        .iter()
                        .any(|parent| parent != *path && path.starts_with(parent))
                })
                .cloned()
                .collect();
            for source_root in roots {
                let canonical =
                    fs::canonicalize(&source_root).map_err(|e| io_error(&source_root, e))?;
                let prefix = relative(root, &canonical)?;
                if root_options.exclude.iter().any(|e| excluded(&prefix, e)) {
                    continue;
                }
                let options = SourceOptions {
                    sdk_crate_names: candidate
                        .sdk_dependencies
                        .iter()
                        .map(|d| {
                            d.alias
                                .as_deref()
                                .unwrap_or("soroban_sdk")
                                .replace('-', "_")
                        })
                        .collect(),
                    exclude: root_options
                        .exclude
                        .iter()
                        .filter_map(|e| {
                            if e.components().count() == 1 {
                                Some(e.clone())
                            } else {
                                e.strip_prefix(&prefix).ok().map(Path::to_path_buf)
                            }
                        })
                        .collect(),
                };
                let mut source = inventory_interface_directory(&canonical, &options)
                    .map_err(DiffError::Source)?;
                for f in &mut source.functions {
                    evidence_paths(std::slice::from_mut(&mut f.evidence), &prefix);
                }
                for t in &mut source.types {
                    t.identity = prefixed_identity(&prefix, &t.identity);
                    evidence_paths(std::slice::from_mut(&mut t.evidence), &prefix);
                }
                for e in &mut source.events {
                    e.identity = prefixed_identity(&prefix, &e.identity);
                    evidence_paths(std::slice::from_mut(&mut e.evidence), &prefix);
                }
                evidence_paths(&mut source.uncertainties, &prefix);
                model.source.functions.append(&mut source.functions);
                model.source.types.append(&mut source.types);
                model.source.events.append(&mut source.events);
                model.source.uncertainties.append(&mut source.uncertainties);
                let mut storage =
                    inventory_storage_directory(&canonical, &options).map_err(DiffError::Source)?;
                let mut names = BTreeMap::new();
                for t in &mut storage.contract_types {
                    let old = t.identity.clone();
                    t.identity = prefixed_identity(&prefix, &old);
                    names.insert(old, t.identity.clone());
                    evidence_paths(std::slice::from_mut(&mut t.evidence), &prefix);
                }
                for entry in &mut storage.entries {
                    if let Some(id) = &mut entry.key.contract_type {
                        if let Some(new) = names.get(id) {
                            entry.key.identity = entry.key.identity.replacen(id.as_str(), new, 1);
                            *id = new.clone();
                        }
                    } else if entry.key.dynamic {
                        entry.key.identity = format!("{}:{}", prefix.display(), entry.key.identity);
                    }
                    evidence_paths(&mut entry.evidence, &prefix);
                }
                evidence_paths(&mut storage.uncertainties, &prefix);
                model.storage.entries.append(&mut storage.entries);
                model
                    .storage
                    .contract_types
                    .append(&mut storage.contract_types);
                model
                    .storage
                    .uncertainties
                    .append(&mut storage.uncertainties);
                let mut findings =
                    analyze_directory(&canonical, context, &options).map_err(DiffError::Source)?;
                for finding in &mut findings {
                    for e in &mut finding.evidence {
                        if let Some(path) = &mut e.path {
                            *path = relative(root, path)?;
                        }
                    }
                }
                model.findings.append(&mut findings);
            }
            model.findings.sort_by(|a, b| a.id.cmp(&b.id));
            contracts.insert(model.package.manifest_path.clone(), model);
        }
    }
    Ok(RefAnalysis {
        commit: commit.into(),
        cargo_packages: packages.into_values().collect(),
        contracts: contracts.into_values().collect(),
    })
}
pub fn git_diff(repository: &Path, from: &str, to: &str) -> Result<GitDiff, DiffError> {
    let repo = GitRepository::open(repository, Duration::from_secs(15)).map_err(DiffError::Git)?;
    let dirty = repo.is_dirty().map_err(DiffError::Git)?;
    let snapshots = repo.snapshots(from, to).map_err(DiffError::Git)?;
    let result = (|| {
        let before = ref_analysis(snapshots.from_path(), &snapshots.from_commit)?;
        let after = ref_analysis(snapshots.to_path(), &snapshots.to_commit)?;
        Ok(compare_refs(from, to, dirty, before, after))
    })();
    let path = snapshots.root().to_path_buf();
    match snapshots.close() {
        Ok(()) => result.map_err(|source| DiffError::AnalysisSnapshot {
            path,
            source: Box::new(source),
        }),
        Err(source) => Err(DiffError::Cleanup {
            source,
            primary: result.err().map(Box::new),
        }),
    }
}
