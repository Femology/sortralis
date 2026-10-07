use clap::{Parser, Subcommand};
use doctor_runner::{detect_environment, EnvironmentError, ToolState};
use std::{error::Error, fmt, path::PathBuf, process::ExitCode, time::Duration};

#[derive(Debug, Parser)]
#[command(
    name = "sortralis",
    version,
    about = "Pre-alpha CLI for analyzing Soroban contract upgrades",
    after_help = "Phase 3: scan --environment detects tool versions only. Repository analysis is not implemented.\nPassing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
)]
struct Cli {
    #[command(subcommand)]
    command: DoctorCommand,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Repository analysis is not implemented; --environment detects tool versions only
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
    /// Explain a rule (not implemented yet)
    Explain,
}

impl DoctorCommand {
    fn name(&self) -> &'static str {
        match self {
            Self::Scan { .. } => "scan",
            Self::Compare => "compare",
            Self::Check => "check",
            Self::Explain => "explain",
        }
    }
}

#[derive(Debug)]
enum CliError {
    NotImplemented(DoctorCommand),
    Environment(EnvironmentError),
    EnvironmentDetectionFailed,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment(error) => write!(formatter, "{error}"),
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
            println!("Environment detection only; repository analysis is not implemented.");
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
                _ => ExitCode::from(3),
            }
        }
    }
}
