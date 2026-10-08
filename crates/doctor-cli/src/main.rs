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
    about = "Local-first CLI for analyzing Soroban contract upgrades",
    after_help = "Sortralis analyzes source migration risks, storage and interface changes, committed Git revisions, local Wasm artifacts, and configured verification evidence.\nPassing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
)]
struct Cli {
    #[command(subcommand)]
    command: DoctorCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CliFormat {
    Terminal,
    Json,
    Sarif,
    Html,
}

impl From<CliFormat> for doctor_core::report::ReportFormat {
    fn from(fmt: CliFormat) -> Self {
        match fmt {
            CliFormat::Terminal => Self::Terminal,
            CliFormat::Json => Self::Json,
            CliFormat::Sarif => Self::Sarif,
            CliFormat::Html => Self::Html,
        }
    }
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Discover source packages, or explicitly execute ordered project verification
    Doctor {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        verify: bool,
        #[arg(long, requires = "verify")]
        json: bool,
        #[arg(long, requires = "verify")]
        skip_clippy: bool,
        /// Report format: terminal, json, sarif, or html
        #[arg(long)]
        format: Option<CliFormat>,
        /// Destination output file path
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Discover Cargo packages and Soroban candidates; --environment detects tool versions only
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        environment: bool,
    },
    /// Inspect a local Wasm through Stellar CLI, with no network artifact lookup
    Wasm {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Compare contract interfaces between two Wasm artifacts or project directories
    Compare {
        #[arg(long)]
        before: PathBuf,
        #[arg(long)]
        after: PathBuf,
        #[arg(long)]
        json: bool,
    },
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
        /// Report format: terminal, json, sarif, or html
        #[arg(long, default_value = "terminal")]
        format: CliFormat,
        /// Destination output file path (defaults to stdout for terminal/json/sarif, report.html for html)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Explain a registered source migration rule
    Explain { rule_id: String },
}

#[derive(Debug)]
enum CliError {
    Environment(EnvironmentError),
    Cargo(CargoError),
    EnvironmentDetectionFailed,
    UnknownRule(String),
    Check(CheckError),
    Output(std::io::Error),
    Source(doctor_source::SourceError),
    Json(serde_json::Error),
    Diff(doctor_cli::diff::DiffError),
    Wasm(doctor_wasm::WasmError),
    Verification(doctor_cli::verify::VerificationError),
    Compare(doctor_cli::compare::CompareError),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Check(error) => write!(formatter, "{error}"),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
            Self::Source(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "cannot serialize result: {error}"),
            Self::Diff(error) => write!(formatter, "{error}"),
            Self::Wasm(error) => write!(formatter, "{error}"),
            Self::Verification(error) => write!(formatter, "{error}"),
            Self::Compare(error) => write!(formatter, "{error}"),
            Self::Environment(error) => write!(formatter, "{error}"),
            Self::Cargo(error) => write!(formatter, "{error}"),
            Self::UnknownRule(id) => write!(formatter, "unknown rule ID: {id}"),
            Self::EnvironmentDetectionFailed => formatter
                .write_str("environment detection incomplete; review the diagnostics above"),
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
            Self::Wasm(error) => Some(error),
            Self::Verification(error) => Some(error),
            Self::Compare(error) => Some(error),
            Self::Environment(error) => Some(error),
            Self::Cargo(error) => Some(error),
            _ => None,
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    match cli.command {
        DoctorCommand::Doctor {
            path,
            verify: false,
            ..
        } => run(Cli {
            command: DoctorCommand::Scan {
                path,
                environment: false,
            },
        }),
        DoctorCommand::Doctor {
            path,
            verify: true,
            json,
            skip_clippy,
            format,
            output,
        } => {
            let options = doctor_cli::verify::VerificationOptions {
                skip_clippy,
                ..Default::default()
            };
            let rep =
                doctor_cli::verify::verify(&path, &options).map_err(CliError::Verification)?;
            if let Some(fmt) = format {
                let unified = report_from_verification(&rep);
                write_formatted_report(&unified, fmt.into(), output.as_deref())?;
            } else if json {
                use std::io::Write;
                let mut stdout = std::io::stdout().lock();
                serde_json::to_writer_pretty(&mut stdout, &rep).map_err(CliError::Json)?;
                writeln!(stdout).map_err(CliError::Output)?;
            } else {
                let mut stdout = std::io::stdout().lock();
                doctor_report::write_verification(&mut stdout, &rep).map_err(CliError::Output)?;
            }
            Ok(ExitCode::from(rep.exit_code.as_u8()))
        }
        DoctorCommand::Wasm { path, json } => {
            let cwd = std::env::current_dir().map_err(CliError::Output)?;
            let report = doctor_wasm::inspect(&path, &cwd).map_err(CliError::Wasm)?;
            let mut stdout = std::io::stdout().lock();
            if json {
                use std::io::Write;
                serde_json::to_writer_pretty(&mut stdout, &report).map_err(CliError::Json)?;
                writeln!(stdout).map_err(CliError::Output)?;
            } else {
                doctor_report::write_wasm_inspection(&mut stdout, &report)
                    .map_err(CliError::Output)?;
            }
            Ok(ExitCode::SUCCESS)
        }
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
        DoctorCommand::Compare {
            before,
            after,
            json,
        } => {
            let diff = doctor_cli::compare::compare(&before, &after).map_err(CliError::Compare)?;
            let mut stdout = std::io::stdout().lock();
            if json {
                use std::io::Write;
                serde_json::to_writer_pretty(&mut stdout, &diff).map_err(CliError::Json)?;
                writeln!(stdout).map_err(CliError::Output)?;
            } else {
                doctor_report::write_interface_diff(&mut stdout, &diff)
                    .map_err(CliError::Output)?;
            }
            if diff.has_breaking_changes {
                Ok(ExitCode::from(1))
            } else {
                Ok(ExitCode::SUCCESS)
            }
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
            println!("Environment detection only; repository analysis was not requested.");
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
            println!("Cargo discovery only; use `check` for upgrade analysis and configured verification.");
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
        DoctorCommand::Check {
            path,
            format,
            output,
        } => {
            let result = check(&path).map_err(CliError::Check)?;
            let report = report_from_check_result(&path, &result);
            write_formatted_report(&report, format.into(), output.as_deref())?;
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
fn report_from_check_result(
    root: &std::path::Path,
    result: &CheckResult,
) -> doctor_core::report::Report {
    let env = detect_environment(root, Duration::from_secs(5)).ok();
    let mut tools = Vec::new();
    if let Some(env) = env {
        for tool in env.tools {
            let version = match &tool.state {
                ToolState::Detected { version } => Some(version.to_string()),
                _ => None,
            };
            tools.push(doctor_core::report::ToolInfo {
                name: tool.program,
                version,
                status: format!("{:?}", tool.state),
            });
        }
    }

    let steps = vec![
        doctor_core::report::ReportStep {
            name: "Tests".into(),
            status: step_status(&result.tests, result),
            description: None,
            exit_code: None,
            duration_millis: None,
            command: None,
        },
        doctor_core::report::ReportStep {
            name: "Build".into(),
            status: step_status(&result.build, result),
            description: None,
            exit_code: None,
            duration_millis: None,
            command: None,
        },
    ];

    doctor_core::report::Report::new(doctor_core::report::ReportParams {
        repository_path: result.analysis.repository_path.clone(),
        verdict: result.analysis.verdict,
        exit_code: result.exit_code.as_u8(),
        packages: result.analysis.detected_packages.clone(),
        tools,
        steps,
        findings: result.analysis.findings.clone(),
        command_results: result.analysis.command_results.clone(),
    })
}

fn report_from_verification(
    rep: &doctor_core::verification::VerificationReport,
) -> doctor_core::report::Report {
    let mut findings = Vec::new();
    let mut command_results = Vec::new();
    let mut steps = Vec::new();

    for s in &rep.steps {
        if let Some(f) = &s.finding {
            findings.push(f.clone());
        }
        if let Some(c) = &s.command {
            command_results.push(c.clone());
        }
        steps.push(doctor_core::report::ReportStep {
            name: s.name.clone(),
            status: format!("{:?}", s.status),
            description: s.reason.clone(),
            exit_code: s.exit_code,
            duration_millis: Some(s.duration_millis),
            command: s.displayed_command.clone(),
        });
    }

    let verdict = if rep.exit_code == doctor_core::ExitCode::Completed {
        doctor_core::Verdict::ChecksCompletedWithWarnings
    } else {
        doctor_core::Verdict::NotReady
    };

    doctor_core::report::Report::new(doctor_core::report::ReportParams {
        repository_path: rep.repository_path.clone(),
        verdict,
        exit_code: rep.exit_code.as_u8(),
        packages: vec![],
        tools: vec![],
        steps,
        findings,
        command_results,
    })
}

fn write_formatted_report(
    report: &doctor_core::report::Report,
    format: doctor_core::report::ReportFormat,
    output: Option<&std::path::Path>,
) -> Result<(), CliError> {
    use doctor_core::report::ReportFormat;
    use std::io::IsTerminal;

    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    match output {
        Some(p) if p != std::path::Path::new("-") => {
            let file = std::fs::File::create(p).map_err(CliError::Output)?;
            let mut writer = std::io::BufWriter::new(file);
            doctor_report::write_report(&mut writer, report, format, false)
                .map_err(CliError::Output)?;
            if format == ReportFormat::Html {
                eprintln!("HTML report written to {}", p.display());
            }
        }
        _ => {
            if format == ReportFormat::Html && output.is_none() {
                let default_path = PathBuf::from("report.html");
                let file = std::fs::File::create(&default_path).map_err(CliError::Output)?;
                let mut writer = std::io::BufWriter::new(file);
                doctor_report::write_report(&mut writer, report, format, false)
                    .map_err(CliError::Output)?;
                eprintln!("HTML report written to report.html");
            } else {
                let mut stdout = std::io::stdout().lock();
                doctor_report::write_report(&mut stdout, report, format, use_color)
                    .map_err(CliError::Output)?;
            }
        }
    }
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
                CliError::Wasm(error) => ExitCode::from(error.exit_code()),
                CliError::Verification(error) => ExitCode::from(error.exit_code()),
                CliError::Compare(error) => ExitCode::from(error.exit_code()),
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
