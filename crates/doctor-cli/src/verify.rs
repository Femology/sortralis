//! Explicit ordered verification. Target failures are data, not Doctor errors.
use doctor_cargo::{analyze_with, CargoError};
use doctor_core::{
    verification::*, Category, CommandStatus, Config, ConfigError, ExitCode, InvalidRuleId,
    CONFIG_FILE_NAME,
};
use doctor_runner::{
    redact, BoundedRunner, CapturedCommand, CommandRunner, CommandSpec, RunnerError, StellarCli,
};
use std::{
    cell::RefCell,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};
mod evidence;
pub use evidence::bounded_text;
use evidence::{error_step, failed, record, skip, task};

pub const OUTPUT_LIMIT: usize = 64 * 1024;
const JSON_LIMIT: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct VerificationOptions {
    pub timeout: Duration,
    pub skip_clippy: bool,
}
impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(600),
            skip_clippy: false,
        }
    }
}
#[derive(Debug)]
pub enum VerificationError {
    Path { path: PathBuf, source: io::Error },
    Config(ConfigError),
    RuleId(InvalidRuleId),
    InvalidTimeout,
    NoManifest(PathBuf),
}
impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Path { path, source } => {
                format!("invalid verification path {}: {source}", path.display())
            }
            Self::Config(e) => e.to_string(),
            Self::NoManifest(path) => format!("no Cargo manifest at {}", path.display()),
            Self::RuleId(e) => e.to_string(),
            Self::InvalidTimeout => "verification timeout must be nonzero".into(),
        };
        f.write_str(&bounded_text(&text).0)
    }
}
impl Error for VerificationError {}
impl VerificationError {
    pub fn exit_code(&self) -> u8 {
        if matches!(self, Self::RuleId(_)) {
            4
        } else {
            2
        }
    }
}
struct ProductionRunner;
impl CommandRunner for ProductionRunner {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        let json = request.args.first().is_some_and(|a| a == "metadata")
            || request.args.iter().any(|a| a == "--output");
        BoundedRunner {
            limit: if json { JSON_LIMIT } else { OUTPUT_LIMIT },
        }
        .execute(request)
    }
}
struct Recording<'a, R: CommandRunner> {
    runner: &'a R,
    captures: RefCell<Vec<CapturedCommand>>,
}
impl<R: CommandRunner> CommandRunner for Recording<'_, R> {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        let result = self.runner.execute(request);
        match &result {
            Ok(c) => self.captures.borrow_mut().push(c.clone()),
            Err(RunnerError::Execution { capture, .. }) => {
                self.captures.borrow_mut().push(*capture.clone())
            }
            _ => {}
        }
        result
    }
}
fn cargo(root: &Path, args: &[&str], timeout: Duration) -> CommandSpec {
    CommandSpec {
        program: "cargo".into(),
        args: args.iter().map(Into::into).collect(),
        working_directory: root.into(),
        timeout,
    }
}
fn finish(mut report: VerificationReport) -> VerificationReport {
    report.exit_code = if report.steps.iter().any(|s| {
        matches!(
            s.status,
            VerificationStatus::MissingTool
                | VerificationStatus::TimedOut
                | VerificationStatus::ExecutionError
        )
    }) {
        ExitCode::ExternalFailure
    } else if report.steps.iter().any(|s| failed(s.status)) {
        ExitCode::PolicyFailed
    } else {
        ExitCode::Completed
    };
    report
}
pub fn verify(
    repository: &Path,
    options: &VerificationOptions,
) -> Result<VerificationReport, VerificationError> {
    verify_with(repository, options, &ProductionRunner)
}
pub fn verify_with(
    repository: &Path,
    options: &VerificationOptions,
    runner: &impl CommandRunner,
) -> Result<VerificationReport, VerificationError> {
    if options.timeout.is_zero() {
        return Err(VerificationError::InvalidTimeout);
    }
    let path = fs::canonicalize(repository).map_err(|source| VerificationError::Path {
        path: repository.into(),
        source,
    })?;
    let mut report = VerificationReport::new(PathBuf::from(redact(&path.to_string_lossy())));
    let discovery = match analyze_with(&path, Duration::from_secs(30), runner) {
        Ok(d) => d,
        Err(e) => {
            match e {
                CargoError::Path { path, source } => {
                    return Err(VerificationError::Path { path, source })
                }
                CargoError::NoManifest { path } => return Err(VerificationError::NoManifest(path)),
                CargoError::MetadataFailed(c) | CargoError::InvalidMetadata { command: c, .. } => {
                    let ok = record(
                        &mut report,
                        "Cargo discovery",
                        "VERIFY_DISCOVERY_FAILED",
                        Category::Dependency,
                        *c,
                    )?;
                    if ok {
                        error_step(
                            &mut report,
                            "Cargo discovery JSON",
                            "VERIFY_DISCOVERY_FAILED",
                            Category::Dependency,
                            "Cargo metadata was malformed or exceeded the JSON capture limit",
                            true,
                        )?;
                    }
                }
                CargoError::Runner(RunnerError::Execution { capture, .. }) => {
                    record(
                        &mut report,
                        "Cargo discovery",
                        "VERIFY_DISCOVERY_FAILED",
                        Category::Dependency,
                        *capture,
                    )?;
                }
                e => error_step(
                    &mut report,
                    "Cargo discovery",
                    "VERIFY_DISCOVERY_FAILED",
                    Category::Dependency,
                    &e.to_string(),
                    true,
                )?,
            }
            for name in [
                "Format",
                "Tests",
                "Clippy",
                "Contract build",
                "Wasm inspection",
            ] {
                skip(
                    &mut report,
                    name,
                    "Workspace discovery failed; no safe workspace/artifact plan is available",
                );
            }
            return Ok(finish(report));
        }
    };
    let root = discovery.workspace_root;
    let config =
        Config::load_optional(root.join(CONFIG_FILE_NAME)).map_err(VerificationError::Config)?;
    report.repository_path = PathBuf::from(redact(&root.to_string_lossy()));
    record(
        &mut report,
        "Cargo discovery",
        "VERIFY_DISCOVERY_FAILED",
        Category::Dependency,
        discovery.command,
    )?;
    task(
        &mut report,
        &cargo(&root, &["fmt", "--all", "--", "--check"], options.timeout),
        "Format",
        "VERIFY_FORMAT_FAILED",
        Category::Source,
        runner,
    )?;
    if config.run_tests {
        task(
            &mut report,
            &cargo(&root, &["test", "--workspace"], options.timeout),
            "Tests",
            "VERIFY_TEST_FAILED",
            Category::Test,
            runner,
        )?;
    } else {
        skip(
            &mut report,
            "Tests",
            "run_tests=false in workspace configuration",
        );
    }
    if options.skip_clippy {
        skip(
            &mut report,
            "Clippy",
            "Explicit --skip-clippy; this check was not performed",
        );
    } else {
        task(
            &mut report,
            &cargo(
                &root,
                &["clippy", "--workspace", "--all-targets"],
                options.timeout,
            ),
            "Clippy",
            "VERIFY_CLIPPY_FAILED",
            Category::Source,
            runner,
        )?;
    }
    let contracts: Vec<_> = discovery
        .contracts
        .iter()
        .filter(|c| c.crate_types.iter().any(|t| t == "cdylib"))
        .collect();
    if !config.build_contracts || contracts.is_empty() {
        let reason = if !config.build_contracts {
            "build_contracts=false in workspace configuration"
        } else {
            "No Soroban package with a cdylib target was discovered"
        };
        skip(&mut report, "Contract build", reason);
        skip(&mut report, "Wasm inspection", reason);
        return Ok(finish(report));
    }
    let directory = match tempfile::Builder::new()
        .prefix("sortralis-verify-")
        .tempdir()
    {
        Ok(d) => d,
        Err(e) => {
            error_step(
                &mut report,
                "Artifact directory",
                "VERIFY_ARTIFACT_FAILED",
                Category::Build,
                &e.to_string(),
                true,
            )?;
            skip(
                &mut report,
                "Contract build",
                "Cannot create a private fresh output directory",
            );
            skip(
                &mut report,
                "Wasm inspection",
                "No successful build/artifact",
            );
            return Ok(finish(report));
        }
    };
    for (index, contract) in contracts.iter().enumerate() {
        let package = &contract.package.name;
        let output = directory.path().join(index.to_string());
        let request =
            StellarCli::<BoundedRunner>::build_to_request(&root, options.timeout, package, &output);
        let name = format!("Build {package}");
        let built = task(
            &mut report,
            &request,
            &name,
            "VERIFY_BUILD_FAILED",
            Category::Build,
            runner,
        )?;
        if !built {
            skip(&mut report, &format!("Inspect {package}"), "Dependency skipped: this package build did not succeed; stale/partial artifacts are not inspected");
            continue;
        }
        let artifact = output.join(format!("{}.wasm", package.replace('-', "_")));
        if !artifact.is_file() || artifact.is_symlink() || output.is_symlink() {
            error_step(
                &mut report,
                &format!("Artifact {package}"),
                "VERIFY_ARTIFACT_FAILED",
                Category::Build,
                "Successful CLI exit did not produce the expected regular fresh Wasm output",
                false,
            )?;
            skip(
                &mut report,
                &format!("Inspect {package}"),
                "Expected build artifact is missing/unsupported",
            );
            continue;
        }
        let recording = Recording {
            runner,
            captures: RefCell::new(vec![]),
        };
        let cli = StellarCli::new(&recording, Duration::from_secs(30));
        let inspected = doctor_wasm::inspect_with(&artifact, &root, &cli);
        for capture in recording.captures.into_inner() {
            let operation = if capture
                .request
                .args
                .first()
                .is_some_and(|a| a == "--version")
            {
                "version".into()
            } else {
                capture
                    .request
                    .args
                    .get(2)
                    .map(|a| a.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".into())
            };
            let absent = matches!(operation.as_str(), "meta" | "env-meta")
                && capture.record.status == (CommandStatus::Exited { code: 1 })
                && doctor_wasm::is_absent_metadata(&capture.record.stderr);
            record(
                &mut report,
                &format!("Inspect {package}: {operation}"),
                "VERIFY_WASM_FAILED",
                Category::Wasm,
                capture,
            )?;
            if absent {
                if let Some(step) = report.steps.last_mut() {
                    step.status = VerificationStatus::Skipped;
                    step.reason = Some(
                        "Metadata section absent (verified CLI diagnostic); actual exit 1 retained"
                            .into(),
                    );
                    step.finding = None;
                }
            }
        }
        if let Err(e) = inspected {
            let already_failed = report.steps.last().is_some_and(|s| failed(s.status));
            if !already_failed {
                error_step(
                    &mut report,
                    &format!("Inspect {package}: parse/validate"),
                    "VERIFY_WASM_FAILED",
                    Category::Wasm,
                    &e.to_string(),
                    false,
                )?;
            }
        }
    }
    let cleanup_path = directory.path().to_path_buf();
    if let Err(e) = directory.close() {
        error_step(
            &mut report,
            "Artifact cleanup",
            "VERIFY_CLEANUP_FAILED",
            Category::Build,
            &format!("Cannot clean {}: {e}", cleanup_path.display()),
            true,
        )?;
    }
    Ok(finish(report))
}
