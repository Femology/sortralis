use clap::{Parser, Subcommand};
use doctor_cargo::{analyze, CargoError};
use doctor_cli::check::{check, CheckError, CheckResult, StepStatus};
use doctor_core::CommandStatus;
use doctor_runner::{detect_environment, EnvironmentError, ToolState};
use doctor_source::registry;
use std::{error::Error, fmt, path::PathBuf, process::ExitCode, time::Duration};

#[derive(Debug, Parser)]
#[command(
    name = "sortralis",
    version,
    about = "Pre-alpha CLI for analyzing Soroban contract upgrades",
    after_help = "Phase 8: diff compares committed source snapshots without switching the active worktree. Wasm comparison and full scan report files are not implemented.\nPassing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
)]
struct Cli {
    #[command(subcommand)]
    command: DoctorCommand,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Discover Cargo packages and Soroban candidates; --environment detects tool versions only
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        environment: bool,
    },
    /// Compare contract artifacts (not implemented yet)
    Compare,
    /// Compare committed Git source snapshots without switching the active worktree
    Diff {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Compare storage observations in explicit before/after source snapshots
    CompareSource {
        #[arg(long)]
        before: PathBuf,
        #[arg(long)]
        after: PathBuf,
        /// Emit the versioned storage diff object to stdout
        #[arg(long)]
        json: bool,
        /// Relative path prefix to exclude on both sides (repeatable)
        #[arg(long)]
        exclude: Vec<PathBuf>,
        /// SDK Rust crate identifiers, including dependency aliases
        #[arg(long, default_value = "soroban_sdk")]
        sdk_crate: Vec<String>,
    },
    /// Run source checks, configured repository tests, and Stellar contract builds
    Check {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Explain a registered source migration rule
    Explain { rule_id: String },
}

impl DoctorCommand {
    fn name(&self) -> &'static str {
        match self {
            Self::Scan { .. } => "scan",
            Self::Compare => "compare",
            Self::Diff { .. } => "diff",
            Self::CompareSource { .. } => "compare-source",
            Self::Check { .. } => "check",
            Self::Explain { .. } => "explain",
        }
    }
}

#[derive(Debug)]
enum CliError {
    NotImplemented(DoctorCommand),
    Environment(EnvironmentError),
    Cargo(CargoError),
    EnvironmentDetectionFailed,
    UnknownRule(String),
    Check(CheckError),
    Output(std::io::Error),
    Source(doctor_source::SourceError),
    Json(serde_json::Error),
    Diff(doctor_cli::diff::DiffError),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check(error) => write!(formatter, "{error}"),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
            Self::Source(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "cannot serialize result: {error}"),
            Self::Diff(error) => write!(formatter, "{error}"),
            Self::Environment(error) => write!(formatter, "{error}"),
            Self::Cargo(error) => write!(formatter, "{error}"),
            Self::UnknownRule(id) => write!(formatter, "unknown rule ID: {id}"),
            Self::EnvironmentDetectionFailed => formatter
                .write_str("environment detection incomplete; review the diagnostics above"),
            Self::NotImplemented(command) => write!(
                formatter,
                "'{}' is not implemented yet; use --help or --version",
                command.name()
            ),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Check(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Diff(error) => Some(error),
            Self::Environment(error) => Some(error),
            Self::Cargo(error) => Some(error),
            _ => None,
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    match cli.command {
        DoctorCommand::Diff {
            from,
            to,
            repo,
            json,
        } => {
            let diff = doctor_cli::diff::git_diff(&repo, &from, &to).map_err(CliError::Diff)?;
            let mut stdout = std::io::stdout().lock();
            if json {
                use std::io::Write;
                serde_json::to_writer_pretty(&mut stdout, &diff).map_err(CliError::Json)?;
                writeln!(stdout).map_err(CliError::Output)?;
            } else {
                doctor_report::write_git_diff(&mut stdout, &diff).map_err(CliError::Output)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        DoctorCommand::CompareSource {
            before,
            after,
            json,
            exclude,
            sdk_crate,
        } => {
            let options = doctor_source::SourceOptions {
                exclude,
                sdk_crate_names: sdk_crate,
            };
            let before = doctor_source::inventory_storage_directory(&before, &options)
                .map_err(CliError::Source)?;
            let after = doctor_source::inventory_storage_directory(&after, &options)
                .map_err(CliError::Source)?;
            let diff = doctor_core::storage::compare_storage(&before, &after);
            let mut stdout = std::io::stdout().lock();
            if json {
                use std::io::Write;
                serde_json::to_writer_pretty(&mut stdout, &diff).map_err(CliError::Json)?;
                writeln!(stdout).map_err(CliError::Output)?;
            } else {
                doctor_report::write_storage_diff(&mut stdout, &diff).map_err(CliError::Output)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        DoctorCommand::Scan {
            path,
            environment: true,
        } => {
            let environment =
                detect_environment(&path, Duration::from_secs(5)).map_err(CliError::Environment)?;
            println!("Environment detection only; full repository analysis is not implemented.");
            let mut failed = false;
            for tool in &environment.tools {
                match &tool.state {
                    ToolState::Detected { version } => println!("{}: {version}", tool.program),
                    state => {
                        failed = true;
                        println!("{}: {:?}", tool.program, state);
                        eprintln!(
                            "{} --version status: {:?}",
                            tool.program, tool.command.record.status
                        );
                        eprint!(
                            "{}{}",
                            tool.command.record.stdout, tool.command.record.stderr
                        );
                    }
                }
            }
            for finding in environment.findings {
                eprintln!("{}: {}", finding.id.as_str(), finding.summary);
            }
            if failed {
                Err(CliError::EnvironmentDetectionFailed)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        DoctorCommand::Scan {
            path,
            environment: false,
        } => {
            let analysis = analyze(&path, Duration::from_secs(30)).map_err(CliError::Cargo)?;
            println!("Cargo discovery only. Full upgrade analysis is not implemented.");
            println!("Repository: {}", analysis.repository_path.display());
            println!("Workspace: {}", analysis.workspace_root.display());
            println!("Workspace packages: {}", analysis.packages.len());
            for package in &analysis.packages {
                println!("  {}: {}", package.name, package.manifest_path.display());
            }
            println!("Soroban contract candidates: {}", analysis.contracts.len());
            for contract in &analysis.contracts {
                println!(
                    "  {} ({})",
                    contract.package.name,
                    contract.package.manifest_path.display()
                );
                for sdk in &contract.sdk_dependencies {
                    println!(
                        "    soroban-sdk requested: {} (optional: {}, target: {})",
                        sdk.requirement,
                        sdk.optional,
                        sdk.target.as_deref().unwrap_or("all")
                    );
                }
                println!("    crate types: {}", contract.crate_types.join(", "));
            }
            if analysis.contracts.is_empty() {
                println!("No Soroban contract candidates found.");
            }
            println!("Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.");
            Ok(ExitCode::SUCCESS)
        }
        DoctorCommand::Check { path } => {
            let result = check(&path).map_err(CliError::Check)?;
            print_check(&result)?;
            Ok(ExitCode::from(result.exit_code.as_u8()))
        }
        DoctorCommand::Explain { rule_id } => {
            let registration = registry()
                .iter()
                .find(|rule| rule.documentation.id == rule_id)
                .ok_or_else(|| CliError::UnknownRule(rule_id.clone()))?;
            let doc = registration.documentation;
            println!("{}: {}", doc.id, doc.title);
            println!("{}", doc.description);
            println!("Supported context: upgrading to or using Soroban SDK v28");
            println!("Severity: {:?}", doc.severity);
            println!(
                "Detection: {}",
                if registration.analyzer.is_some() {
                    "Parser-based source check"
                } else {
                    "Manual only; no automatic detector"
                }
            );
            println!("Why it matters: {}", doc.why_it_matters);
            println!("Recommendation: {}", doc.recommendation);
            println!("Limitations: {}", doc.limitations);
            for reference in doc.references {
                println!("Reference: {reference}");
            }
            Ok(ExitCode::SUCCESS)
        }
        command => Err(CliError::NotImplemented(command)),
    }
}

fn step_status(step: &StepStatus, result: &CheckResult) -> String {
    match step {
        StepStatus::Disabled => "disabled by configuration".into(),
        StepStatus::NotApplicable => "not applicable: no Soroban packages".into(),
        StepStatus::Blocked { reason } => format!("not run: {reason}"),
        StepStatus::Executed { command_index } => {
            match result.analysis.command_results.get(*command_index) {
                Some(command) => match command.status {
                    CommandStatus::Exited { code: 0 } => "passed (recorded exit 0)".into(),
                    CommandStatus::Exited { code } => format!("failed (recorded exit {code})"),
                    _ => format!("execution failed: {:?}", command.status),
                },
                None => "missing command evidence".into(),
            }
        }
    }
}
fn print_check(result: &CheckResult) -> Result<(), CliError> {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    writeln!(
        stdout,
        "Workspace: {}",
        result.analysis.repository_path.display()
    )
    .map_err(CliError::Output)?;
    writeln!(stdout, "Tests: {}", step_status(&result.tests, result)).map_err(CliError::Output)?;
    writeln!(stdout, "Build: {}", step_status(&result.build, result)).map_err(CliError::Output)?;
    for finding in &result.analysis.findings {
        writeln!(
            stdout,
            "{} [{:?}]: {}",
            finding.id.as_str(),
            finding.severity,
            finding.summary
        )
        .map_err(CliError::Output)?;
        for evidence in &finding.evidence {
            if let Some(path) = &evidence.path {
                writeln!(
                    stdout,
                    "  {}{}: {}",
                    path.display(),
                    evidence
                        .line
                        .map(|line| format!(":{line}"))
                        .unwrap_or_default(),
                    evidence.message
                )
                .map_err(CliError::Output)?;
            }
        }
    }
    for capture in &result.captures {
        writeln!(
            stdout,
            "Command: {} {} ({:?})",
            capture.record.program,
            capture.record.args.join(" "),
            capture.record.status
        )
        .map_err(CliError::Output)?;
        if capture
            .record
            .args
            .first()
            .is_none_or(|arg| arg != "metadata")
            || capture.record.status != (CommandStatus::Exited { code: 0 })
        {
            stdout
                .write_all(&capture.stdout_bytes)
                .map_err(CliError::Output)?;
        }
        stderr
            .write_all(&capture.stderr_bytes)
            .map_err(CliError::Output)?;
    }
    writeln!(
        stdout,
        "Verdict: {:?}; exit {}",
        result.analysis.verdict,
        result.exit_code.as_u8()
    )
    .map_err(CliError::Output)?;
    writeln!(stdout,"Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.").map_err(CliError::Output)?;
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            match error {
                CliError::Check(error) => ExitCode::from(error.exit_code().as_u8()),
                CliError::Source(_) => ExitCode::from(2),
                CliError::Json(_) => ExitCode::from(4),
                CliError::Diff(error) => ExitCode::from(error.exit_code()),
                CliError::NotImplemented(_) => ExitCode::FAILURE,
                CliError::UnknownRule(_) => ExitCode::from(2),
                CliError::Cargo(CargoError::Path { .. } | CargoError::NoManifest { .. }) => {
                    ExitCode::from(2)
                }
                CliError::Cargo(CargoError::InvalidMetadata { .. }) => ExitCode::from(4),
                _ => ExitCode::from(3),
            }
        }
    }
}
