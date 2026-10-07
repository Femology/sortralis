use clap::{Parser, Subcommand};
use doctor_cargo::{analyze, CargoError};
use doctor_runner::{detect_environment, EnvironmentError, ToolState};
use doctor_source::registry;
use std::{error::Error, fmt, path::PathBuf, process::ExitCode, time::Duration};

#[derive(Debug, Parser)]
#[command(
    name = "sortralis",
    version,
    about = "Pre-alpha CLI for analyzing Soroban contract upgrades",
    after_help = "Phase 5: scan discovers Cargo packages; explain documents source migration rules. Full upgrade analysis is not implemented.\nPassing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
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
    /// Check upgrade readiness (not implemented yet)
    Check,
    /// Explain a registered source migration rule
    Explain { rule_id: String },
}

impl DoctorCommand {
    fn name(&self) -> &'static str {
        match self {
            Self::Scan { .. } => "scan",
            Self::Compare => "compare",
            Self::Check => "check",
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
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::Environment(error) => Some(error),
            Self::Cargo(error) => Some(error),
            _ => None,
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
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
                Ok(())
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
            Ok(())
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
            Ok(())
        }
        command => Err(CliError::NotImplemented(command)),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            match error {
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
