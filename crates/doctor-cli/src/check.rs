use doctor_source::TargetContext;
use semver::{Op, Version, VersionReq};

/// SDK 28.0.0 migration guide, spec-shaking section. This is a minimum build
/// requirement, not proof that a contract will compile or is safe to deploy.
pub const SDK28_MIN_STELLAR_CLI: (u64, u64, u64) = (25, 2, 0);
pub const SDK28_BUILD_REFERENCE: &str =
    "https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs";
pub fn sdk28_min_stellar_cli() -> Version {
    Version::new(
        SDK28_MIN_STELLAR_CLI.0,
        SDK28_MIN_STELLAR_CLI.1,
        SDK28_MIN_STELLAR_CLI.2,
    )
}
pub fn sdk28_cli_supported(version: &Version) -> bool {
    version >= &sdk28_min_stellar_cli()
}
/// Only single, major-constraining requirements establish the source context.
/// Broad bounds and path-only dependencies remain unknown; no resolution is claimed.
pub fn sdk_context(requirement: &str) -> TargetContext {
    let Ok(requirement) = VersionReq::parse(requirement) else {
        return TargetContext::Unknown;
    };
    match requirement.comparators.as_slice() {
        [comparator]
            if matches!(
                comparator.op,
                Op::Exact | Op::Caret | Op::Tilde | Op::Wildcard
            ) =>
        {
            if comparator.major == 28 {
                TargetContext::Sdk28
            } else {
                TargetContext::OtherSdk
            }
        }
        _ => TargetContext::Unknown,
    }
}

use doctor_cargo::{analyze_with, CargoError};
use doctor_core::{
    evaluate_policy, exit_code, AnalysisResult, Category, CommandStatus, Config, ConfigError,
    Evidence, ExitCode, Finding, InvalidRuleId, RuleId, RunOutcome, Severity, Verdict,
    CONFIG_FILE_NAME, REPORT_SCHEMA_VERSION,
};
use doctor_runner::{
    detect_environment_with, CapturedCommand, CommandRunner, CommandSpec, EnvironmentError,
    RunnerError, SystemRunner, ToolState,
};
use doctor_source::{analyze_directory, SourceError, SourceOptions};
use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct CheckTimeouts {
    pub environment: Duration,
    pub metadata: Duration,
    pub task: Duration,
}
impl Default for CheckTimeouts {
    fn default() -> Self {
        Self {
            environment: Duration::from_secs(5),
            metadata: Duration::from_secs(30),
            task: Duration::from_secs(600),
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum StepStatus {
    Disabled,
    NotApplicable,
    Blocked { reason: String },
    Executed { command_index: usize },
}
#[derive(Debug)]
pub struct CheckResult {
    pub analysis: AnalysisResult,
    pub exit_code: ExitCode,
    pub tests: StepStatus,
    pub build: StepStatus,
    /// Exact bytes and OS arguments are retained alongside the serializable views.
    pub captures: Vec<CapturedCommand>,
}
#[derive(Debug)]
pub enum CheckError {
    Path { path: PathBuf, source: io::Error },
    NoManifest(PathBuf),
    Config(ConfigError),
    Exclusion(SourceError),
    InvalidTimeout,
    Environment(EnvironmentError),
    Cargo(CargoError),
    RuleId(InvalidRuleId),
}
impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { path, source } => {
                write!(f, "invalid repository {}: {source}", path.display())
            }
            Self::NoManifest(path) => write!(f, "no Cargo.toml at {}", path.display()),
            Self::Config(error) => write!(f, "{error}"),
            Self::Exclusion(error) => write!(f, "{error}"),
            Self::InvalidTimeout => f.write_str("check timeouts must be greater than zero"),
            Self::Environment(error) => write!(f, "{error}"),
            Self::Cargo(error) => write!(f, "{error}"),
            Self::RuleId(error) => write!(f, "{error}"),
        }
    }
}
impl Error for CheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path { source, .. } => Some(source),
            Self::Config(error) => Some(error),
            Self::Exclusion(error) => Some(error),
            Self::Environment(error) => Some(error),
            Self::Cargo(error) => Some(error),
            Self::RuleId(error) => Some(error),
            _ => None,
        }
    }
}
impl CheckError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Path { .. }
            | Self::NoManifest(_)
            | Self::Config(_)
            | Self::Exclusion(_)
            | Self::InvalidTimeout => ExitCode::InvalidInput,
            Self::RuleId(_) | Self::Cargo(CargoError::InvalidMetadata { .. }) => {
                ExitCode::InternalError
            }
            _ => ExitCode::ExternalFailure,
        }
    }
}
fn source_options(config: &Config) -> SourceOptions {
    SourceOptions {
        exclude: config.exclude.iter().map(PathBuf::from).collect(),
        ..SourceOptions::default()
    }
}
fn finding(
    id: &str,
    category: Category,
    severity: Severity,
    summary: String,
    path: Option<PathBuf>,
    command: Option<doctor_core::CommandResult>,
    references: Vec<String>,
) -> Result<Finding, CheckError> {
    Ok(Finding {
        id: RuleId::new(id).map_err(CheckError::RuleId)?,
        title: id.replace('_', " "),
        severity,
        category,
        summary: summary.clone(),
        why_it_matters: "Incomplete or failed checks require review before any deployment decision.".into(),
        evidence: vec![Evidence {path, line: None, message: summary, command}],
        recommendation: "Review the source evidence and unmodified command diagnostics; fix the issue and rerun check.".into(),
        references,
    })
}
pub fn check(repository: &Path) -> Result<CheckResult, CheckError> {
    check_with(repository, &CheckTimeouts::default(), &SystemRunner)
}
pub fn check_with(
    repository: &Path,
    timeouts: &CheckTimeouts,
    runner: &impl CommandRunner,
) -> Result<CheckResult, CheckError> {
    if [timeouts.environment, timeouts.metadata, timeouts.task]
        .iter()
        .any(Duration::is_zero)
    {
        return Err(CheckError::InvalidTimeout);
    }
    let path = fs::canonicalize(repository).map_err(|source| CheckError::Path {
        path: repository.into(),
        source,
    })?;
    let directory = if path.is_dir() {
        path.clone()
    } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
        path.parent()
            .ok_or_else(|| CheckError::NoManifest(path.clone()))?
            .to_path_buf()
    } else {
        return Err(CheckError::NoManifest(path));
    };
    if !directory.join("Cargo.toml").is_file() {
        return Err(CheckError::NoManifest(directory));
    }
    let mut config =
        Config::load_optional(directory.join(CONFIG_FILE_NAME)).map_err(CheckError::Config)?;
    source_options(&config)
        .validate()
        .map_err(CheckError::Exclusion)?;
    let environment = detect_environment_with(&directory, timeouts.environment, runner)
        .map_err(CheckError::Environment)?;
    let discovery =
        analyze_with(&directory, timeouts.metadata, runner).map_err(CheckError::Cargo)?;
    let root = discovery.workspace_root;
    // Workspace membership is authoritative only after Cargo metadata. Revalidate
    // root configuration before analysis/tasks when called with a member manifest.
    if root != directory {
        config = Config::load_optional(root.join(CONFIG_FILE_NAME)).map_err(CheckError::Config)?;
        source_options(&config)
            .validate()
            .map_err(CheckError::Exclusion)?;
    }
    let mut findings = environment.findings;
    let mut captures: Vec<_> = environment
        .tools
        .iter()
        .map(|tool| tool.command.clone())
        .collect();
    captures.push(discovery.command);
    let mut uses_sdk28 = false;
    let mut source_failed = false;
    for contract in &discovery.contracts {
        let contexts: Vec<_> = contract
            .sdk_dependencies
            .iter()
            .map(|sdk| sdk_context(&sdk.requirement))
            .collect();
        let context = if contexts
            .iter()
            .all(|context| *context == TargetContext::Sdk28)
        {
            TargetContext::Sdk28
        } else if contexts
            .iter()
            .all(|context| *context == TargetContext::OtherSdk)
        {
            TargetContext::OtherSdk
        } else {
            TargetContext::Unknown
        };
        uses_sdk28 |= context == TargetContext::Sdk28;
        let Some(package_root) = contract.package.manifest_path.parent() else {
            return Err(CheckError::NoManifest(
                contract.package.manifest_path.clone(),
            ));
        };
        let mut options = source_options(&config);
        options.sdk_crate_names = contract
            .sdk_dependencies
            .iter()
            .map(|sdk| {
                sdk.alias
                    .as_deref()
                    .unwrap_or("soroban_sdk")
                    .replace('-', "_")
            })
            .collect();
        // Translate workspace-relative exclusions into each package's source root.
        if let Ok(relative) = package_root.strip_prefix(&root) {
            if config.exclude.iter().map(PathBuf::from).any(|exclude| {
                if exclude.components().count() == 1 {
                    relative
                        .components()
                        .any(|part| part.as_os_str() == exclude.as_os_str())
                } else {
                    relative.starts_with(exclude)
                }
            }) {
                continue;
            }
            options.exclude = options
                .exclude
                .into_iter()
                .filter_map(|exclude| {
                    if exclude.components().count() == 1 {
                        Some(exclude)
                    } else {
                        exclude.strip_prefix(relative).ok().map(Path::to_path_buf)
                    }
                })
                .collect();
        }
        if context == TargetContext::Unknown {
            findings.push(finding("SDK_CONTEXT_UNRESOLVED",Category::Dependency,Severity::ManualReview,
                "Requested SDK constraints do not establish a single supported source-rule context; v28 source rules were not applied.".into(),Some(contract.package.manifest_path.clone()),None,vec![])?);
        }
        match analyze_directory(package_root, context, &options) {
            Ok(source_findings) => findings.extend(source_findings),
            Err(error) => {
                source_failed = true;
                findings.push(finding(
                    "SOURCE_ANALYSIS_FAILED",
                    Category::Source,
                    Severity::ManualReview,
                    error.to_string(),
                    Some(contract.package.manifest_path.clone()),
                    None,
                    vec![],
                )?);
            }
        }
    }
    let has_contracts = !discovery.contracts.is_empty();
    let detected = |program: &str| {
        environment
            .tools
            .iter()
            .find(|tool| tool.program == program)
            .is_some_and(|tool| matches!(tool.state, ToolState::Detected { .. }))
    };
    let mut external_failure = source_failed
        || has_contracts
            && (config.run_tests || config.build_contracts)
            && (!detected("rustc") || !detected("cargo"));
    let tests = if !config.run_tests {
        StepStatus::Disabled
    } else if !has_contracts {
        StepStatus::NotApplicable
    } else {
        run_task(
            &task_request("cargo", &["test"], &root, timeouts.task),
            Category::Test,
            "TEST_FAILED",
            runner,
            &mut captures,
            &mut findings,
            &mut external_failure,
        )?
    };
    let mut build_block = None;
    if has_contracts && config.build_contracts {
        if !detected("stellar") {
            external_failure = true;
            build_block=Some("Stellar CLI version could not be detected; required build tool unavailable or unverified".into());
        } else if uses_sdk28 {
            if let Some(tool) = environment
                .tools
                .iter()
                .find(|tool| tool.program == "stellar")
            {
                if let ToolState::Detected { version } = &tool.state {
                    let supported = sdk28_cli_supported(version);
                    findings.push(finding("SDK28_STELLAR_BUILD_REQUIREMENT",Category::Toolchain,if supported {Severity::Info} else {Severity::Breaking},
                        format!("SDK v28 requires Stellar CLI >={}; detected {version}. This minimum does not establish build success.", sdk28_min_stellar_cli()),None,Some(tool.command.record.clone()),vec![SDK28_BUILD_REFERENCE.into()])?);
                    if !supported {
                        external_failure = true;
                        build_block =
                            Some("Stellar CLI is below the verified SDK v28 build minimum".into());
                    }
                }
            }
        }
    }
    let build = if !config.build_contracts {
        StepStatus::Disabled
    } else if !has_contracts {
        StepStatus::NotApplicable
    } else if let Some(reason) = build_block {
        StepStatus::Blocked { reason }
    } else {
        run_task(
            &task_request("stellar", &["contract", "build"], &root, timeouts.task),
            Category::Build,
            "BUILD_FAILED",
            runner,
            &mut captures,
            &mut findings,
            &mut external_failure,
        )?
    };
    let mut grouped = std::collections::BTreeMap::<RuleId, Finding>::new();
    for finding in findings {
        if let Some(existing) = grouped.get_mut(&finding.id) {
            existing.evidence.extend(finding.evidence);
        } else {
            grouped.insert(finding.id.clone(), finding);
        }
    }
    let findings: Vec<_> = grouped
        .into_values()
        .map(|mut finding| {
            finding.evidence.sort_by(|a, b| {
                (&a.path, &a.line, &a.message).cmp(&(&b.path, &b.line, &b.message))
            });
            finding.evidence.dedup();
            finding
        })
        .collect();
    let outcome = if external_failure {
        RunOutcome::ExternalFailure
    } else {
        RunOutcome::Completed
    };
    let code = exit_code(outcome, &findings, &config);
    let verdict = if code.as_u8() != 0 {
        Verdict::NotReady
    } else {
        evaluate_policy(&findings, &config).verdict
    };
    let analysis = AnalysisResult {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        repository_path: root,
        detected_packages: discovery.packages,
        findings,
        command_results: captures
            .iter()
            .map(|capture| capture.record.clone())
            .collect(),
        verdict,
    };
    Ok(CheckResult {
        analysis,
        exit_code: code,
        tests,
        build,
        captures,
    })
}
fn task_request(program: &str, args: &[&str], root: &Path, timeout: Duration) -> CommandSpec {
    CommandSpec {
        program: program.into(),
        args: args.iter().map(|arg| (*arg).into()).collect(),
        working_directory: root.into(),
        timeout,
    }
}
fn run_task(
    request: &CommandSpec,
    category: Category,
    id: &str,
    runner: &impl CommandRunner,
    captures: &mut Vec<CapturedCommand>,
    findings: &mut Vec<Finding>,
    external_failure: &mut bool,
) -> Result<StepStatus, CheckError> {
    let root = &request.working_directory;
    let capture = match runner.execute(request) {
        Ok(capture) => capture,
        Err(RunnerError::Execution { capture, .. }) => *capture,
        Err(error) => {
            *external_failure = true;
            findings.push(finding(
                id,
                category,
                Severity::Breaking,
                error.to_string(),
                Some(root.clone()),
                None,
                vec![],
            )?);
            return Ok(StepStatus::Blocked {
                reason: error.to_string(),
            });
        }
    };
    let index = captures.len();
    if capture.record.status != (CommandStatus::Exited { code: 0 }) {
        if !matches!(capture.record.status, CommandStatus::Exited { .. }) {
            *external_failure = true;
        }
        findings.push(finding(
            id,
            category,
            Severity::Breaking,
            format!(
                "{} {}: {:?}",
                capture.record.program,
                capture.record.args.join(" "),
                capture.record.status
            ),
            Some(root.clone()),
            Some(capture.record.clone()),
            vec![],
        )?);
    }
    captures.push(capture);
    Ok(StepStatus::Executed {
        command_index: index,
    })
}
