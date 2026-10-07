//! The only construction boundary for Stellar CLI operations. Syntax and local
//! fetch behavior verified against CLI 27.0.0; see repository EVIDENCE.md.
use crate::{CapturedCommand, CommandRunner, CommandSpec, RunnerError};
use doctor_core::CommandStatus;
use std::{error::Error, ffi::OsString, fmt, io, path::Path, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoOperation {
    Interface,
    Meta,
    EnvMeta,
    Build,
    Hash,
}
impl InfoOperation {
    fn argument(self) -> &'static str {
        match self {
            Self::Interface => "interface",
            Self::Meta => "meta",
            Self::EnvMeta => "env-meta",
            Self::Build => "build",
            Self::Hash => "hash",
        }
    }
}
#[derive(Debug)]
pub enum StellarError {
    Runner(RunnerError),
    Missing(Box<CapturedCommand>),
    Failed(Box<CapturedCommand>),
    NetworkForbidden,
}
impl fmt::Display for StellarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runner(e) => write!(f, "{e}"),
            Self::Missing(c) => write!(f, "Stellar CLI executable {:?} is not installed; install the Stellar CLI to inspect local Wasm", c.request.program),
            Self::Failed(c) => write!(f, "Stellar command {:?} {:?} failed ({:?}): stdout={:?}; stderr={:?}", c.request.program, c.request.args, c.record.status, c.record.stdout, c.record.stderr),
            Self::NetworkForbidden => f.write_str("contract info build is skipped: verified CLI fetches remote attestations; local inspection does not authorize network access"),
        }
    }
}
impl Error for StellarError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runner(e) => Some(e),
            _ => None,
        }
    }
}
pub struct StellarCli<'a, R: CommandRunner> {
    runner: &'a R,
    executable: OsString,
    timeout: Duration,
}
impl<'a, R: CommandRunner> StellarCli<'a, R> {
    pub fn new(runner: &'a R, timeout: Duration) -> Self {
        Self {
            runner,
            executable: "stellar".into(),
            timeout,
        }
    }
    /// Explicit executable injection for tests/controlled installations; never a
    /// shell command string or a target repository configuration value.
    pub fn with_executable(mut self, executable: OsString) -> Self {
        self.executable = executable;
        self
    }
    pub fn version_request(directory: &Path, timeout: Duration) -> CommandSpec {
        CommandSpec {
            program: "stellar".into(),
            args: vec!["--version".into()],
            working_directory: directory.into(),
            timeout,
        }
    }
    pub fn build_request(directory: &Path, timeout: Duration) -> CommandSpec {
        CommandSpec {
            program: "stellar".into(),
            args: vec!["contract".into(), "build".into()],
            working_directory: directory.into(),
            timeout,
        }
    }
    /// Verified CLI 27 copies package-name.replace('-', '_') + .wasm into out_dir.
    pub fn build_to_request(
        directory: &Path,
        timeout: Duration,
        package: &str,
        out_dir: &Path,
    ) -> CommandSpec {
        let mut request = Self::build_request(directory, timeout);
        request.args.extend([
            "--package".into(),
            package.into(),
            "--out-dir".into(),
            out_dir.as_os_str().into(),
        ]);
        request
    }
    fn run(&self, mut request: CommandSpec) -> Result<CapturedCommand, StellarError> {
        request.program = self.executable.clone();
        let capture = self
            .runner
            .execute(&request)
            .map_err(StellarError::Runner)?;
        if capture.spawn_error_kind == Some(io::ErrorKind::NotFound) {
            return Err(StellarError::Missing(Box::new(capture)));
        }
        if capture.record.status != (CommandStatus::Exited { code: 0 }) {
            return Err(StellarError::Failed(Box::new(capture)));
        }
        Ok(capture)
    }
    pub fn version(&self, directory: &Path) -> Result<CapturedCommand, StellarError> {
        self.run(Self::version_request(directory, self.timeout))
    }
    pub fn contract_build(&self, directory: &Path) -> Result<CapturedCommand, StellarError> {
        self.run(Self::build_request(directory, self.timeout))
    }
    pub fn info(
        &self,
        operation: InfoOperation,
        wasm: &Path,
        directory: &Path,
    ) -> Result<CapturedCommand, StellarError> {
        // CLI 27 build.rs performs GitHub HTTP requests for source_repo metadata.
        // It exposes no offline flag. Refuse before invoking even an injected runner.
        if operation == InfoOperation::Build {
            return Err(StellarError::NetworkForbidden);
        }
        let mut args: Vec<OsString> = ["contract", "info", operation.argument(), "--wasm"]
            .into_iter()
            .map(Into::into)
            .collect();
        args.push(wasm.as_os_str().into());
        if matches!(
            operation,
            InfoOperation::Interface | InfoOperation::Meta | InfoOperation::EnvMeta
        ) {
            args.extend(["--output".into(), "json".into()]);
        }
        self.run(CommandSpec {
            program: self.executable.clone(),
            args,
            working_directory: directory.into(),
            timeout: self.timeout,
        })
    }
}
