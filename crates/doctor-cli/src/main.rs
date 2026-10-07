use clap::{Parser, Subcommand};
use std::{error::Error, fmt, process::ExitCode};

#[derive(Debug, Parser)]
#[command(
    name = "sortralis",
    version,
    about = "Pre-alpha CLI for analyzing Soroban contract upgrades",
    after_help = "Phase 2: help and version only. Analysis commands are not implemented yet.\nPassing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
)]
struct Cli {
    #[command(subcommand)]
    command: DoctorCommand,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Scan a repository (not implemented yet)
    Scan,
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
            Self::Scan => "scan",
            Self::Compare => "compare",
            Self::Check => "check",
            Self::Explain => "explain",
        }
    }
}

#[derive(Debug)]
enum CliError {
    NotImplemented(DoctorCommand),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented(command) => write!(
                formatter,
                "'{}' is not implemented yet; use --help or --version",
                command.name()
            ),
        }
    }
}

impl Error for CliError {}

fn run(cli: Cli) -> Result<(), CliError> {
    Err(CliError::NotImplemented(cli.command))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
