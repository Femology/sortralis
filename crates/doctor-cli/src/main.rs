use clap::{Parser, Subcommand};
use std::{error::Error, fmt, process::ExitCode};

#[derive(Debug, Parser)]
#[command(
    name = "soroban-upgrade-doctor",
    version,
    about = "Pre-alpha scaffold for analyzing Soroban contract upgrades",
    after_help = "Phase 1: help and version only. Analysis commands are not implemented yet.\nPassing Soroban Upgrade Doctor is not a security audit and does not prove that an upgrade is safe to deploy."
)]
struct Cli {
    #[command(subcommand)]
    command: DoctorCommand,
}

#[derive(Debug, Subcommand)]
enum DoctorCommand {
    /// Scan a repository (not implemented in Phase 1)
    Scan,
    /// Compare contract artifacts (not implemented in Phase 1)
    Compare,
    /// Check upgrade readiness (not implemented in Phase 1)
    Check,
    /// Explain a rule (not implemented in Phase 1)
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
    NotImplementedInPhaseOne(DoctorCommand),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplementedInPhaseOne(command) => write!(
                formatter,
                "'{}' is not implemented in Phase 1; use --help or --version",
                command.name()
            ),
        }
    }
}

impl Error for CliError {}

fn run(cli: Cli) -> Result<(), CliError> {
    Err(CliError::NotImplementedInPhaseOne(cli.command))
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
