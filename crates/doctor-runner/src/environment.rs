use crate::{CapturedCommand, CommandRunner, CommandSpec, RunnerError, SystemRunner};
use doctor_core::{Category, CommandStatus, Evidence, Finding, RuleId, Severity};
use semver::Version;
use std::{io, path::Path, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolState {
    Detected { version: Version },
    NotInstalled,
    ExecutionFailed,
    UnrecognizedVersion,
}

#[derive(Debug, Clone)]
pub struct ToolDetection {
    pub program: String,
    pub state: ToolState,
    pub command: CapturedCommand,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub tools: Vec<ToolDetection>,
    pub findings: Vec<Finding>,
}

/// Parse only lines naming the requested executable. Extra Stellar XDR lines
/// never supply the Stellar CLI version. No compatibility threshold is applied.
pub fn parse_version(program: &str, output: &str) -> Option<Version> {
    for line in output.lines() {
        let mut tokens = line.split_whitespace();
        let Some(name) = tokens.next() else { continue };
        if !name.trim_end_matches(':').eq_ignore_ascii_case(program) {
            continue;
        }
        for token in tokens {
            if matches!(
                token.to_ascii_lowercase().trim_end_matches(':'),
                "cli" | "version"
            ) {
                continue;
            }
            if let Ok(version) = Version::parse(token.trim_start_matches('v').trim_end_matches(','))
            {
                return Some(version);
            }
            // The first substantive token must be a version, never a later date/hash.
            break;
        }
    }
    None
}

pub fn inspect_tool(program: &str, command: CapturedCommand) -> ToolDetection {
    let state = match command.record.status {
        CommandStatus::FailedToStart { .. }
            if command.spawn_error_kind == Some(io::ErrorKind::NotFound) =>
        {
            ToolState::NotInstalled
        }
        CommandStatus::Exited { code: 0 } => {
            match parse_version(program, &command.record.stdout)
                .or_else(|| parse_version(program, &command.record.stderr))
            {
                Some(version) => ToolState::Detected { version },
                None => ToolState::UnrecognizedVersion,
            }
        }
        _ => ToolState::ExecutionFailed,
    };
    ToolDetection {
        program: program.into(),
        state,
        command,
    }
}

fn finding(tool: &ToolDetection) -> Result<Finding, doctor_core::InvalidRuleId> {
    let (suffix, title, summary, recommendation) = match tool.state {
        ToolState::NotInstalled => (
            "NOT_INSTALLED",
            "Tool not installed",
            "The executable was not found when requested.",
            "Install the tool or make it available on PATH, then rerun environment detection.",
        ),
        ToolState::UnrecognizedVersion => (
            "VERSION_UNRECOGNIZED",
            "Tool version unrecognized",
            "The command succeeded but its version could not be parsed.",
            "Review the captured version output; do not infer tool compatibility.",
        ),
        _ => (
            "DETECTION_FAILED",
            "Tool detection failed",
            "The version command did not complete successfully.",
            "Review the command status and captured diagnostics, then fix the execution failure.",
        ),
    };
    Ok(Finding {
        id: RuleId::new(format!("{}_{}", tool.program.to_ascii_uppercase(), suffix))?,
        title: format!("{}: {title}", tool.program),
        severity: Severity::Warning,
        category: Category::Toolchain,
        summary: summary.into(),
        why_it_matters: "Tool versions are needed for later compatibility checks. Detection alone does not establish compatibility.".into(),
        evidence: vec![Evidence {
            path: None, line: None,
            message: format!("{} --version: {:?}", tool.program, tool.command.record.status),
            command: Some(tool.command.record.clone()),
        }],
        recommendation: recommendation.into(),
        references: vec![],
    })
}

#[derive(Debug)]
pub enum EnvironmentError {
    Runner(RunnerError),
    RuleId(doctor_core::InvalidRuleId),
}

impl std::fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runner(error) => write!(f, "{error}"),
            Self::RuleId(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EnvironmentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runner(error) => Some(error),
            Self::RuleId(error) => Some(error),
        }
    }
}

pub fn detect_environment(
    directory: &Path,
    timeout: Duration,
) -> Result<Environment, EnvironmentError> {
    detect_environment_with(directory, timeout, &SystemRunner)
}
pub fn detect_environment_with(
    directory: &Path,
    timeout: Duration,
    runner: &impl CommandRunner,
) -> Result<Environment, EnvironmentError> {
    let mut environment = Environment {
        tools: vec![],
        findings: vec![],
    };
    for program in ["rustc", "cargo", "stellar"] {
        let command = runner
            .execute(&CommandSpec {
                program: program.into(),
                args: vec!["--version".into()],
                working_directory: directory.into(),
                timeout,
            })
            .map_err(EnvironmentError::Runner)?;
        let tool = inspect_tool(program, command);
        if !matches!(tool.state, ToolState::Detected { .. }) {
            environment
                .findings
                .push(finding(&tool).map_err(EnvironmentError::RuleId)?);
        }
        environment.tools.push(tool);
    }
    Ok(environment)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use doctor_core::CommandResult;

    fn command(
        status: CommandStatus,
        output: &str,
        kind: Option<io::ErrorKind>,
    ) -> CapturedCommand {
        CapturedCommand {
            request: CommandSpec {
                program: "stellar".into(),
                args: vec!["--version".into()],
                working_directory: ".".into(),
                timeout: Duration::from_secs(1),
            },
            record: CommandResult {
                program: "stellar".into(),
                args: vec!["--version".into()],
                working_directory: ".".into(),
                status,
                stdout: output.into(),
                stderr: "raw diagnostic".into(),
                elapsed_millis: 1,
            },
            stdout_bytes: output.as_bytes().to_vec(),
            stderr_bytes: b"raw diagnostic".to_vec(),
            spawn_error_kind: kind,
        }
    }

    #[test]
    fn missing_stellar_has_a_structured_not_installed_finding() {
        let tool = inspect_tool(
            "stellar",
            command(
                CommandStatus::FailedToStart {
                    message: "No such file or directory".into(),
                },
                "",
                Some(io::ErrorKind::NotFound),
            ),
        );
        assert_eq!(tool.state, ToolState::NotInstalled);
        let finding = finding(&tool).unwrap();
        assert_eq!(finding.id.as_str(), "STELLAR_NOT_INSTALLED");
        assert_eq!(finding.category, Category::Toolchain);
        assert_eq!(
            finding.evidence[0].command.as_ref(),
            Some(&tool.command.record)
        );
    }

    #[test]
    fn permissions_nonzero_and_timeout_are_failures_not_missing_tools() {
        for capture in [
            command(
                CommandStatus::FailedToStart {
                    message: "denied".into(),
                },
                "",
                Some(io::ErrorKind::PermissionDenied),
            ),
            command(CommandStatus::Exited { code: 7 }, "stellar 27.0.0", None),
            command(CommandStatus::TimedOut, "stellar 27.0.0", None),
        ] {
            let tool = inspect_tool("stellar", capture);
            assert_eq!(tool.state, ToolState::ExecutionFailed);
            assert_eq!(
                finding(&tool).unwrap().id.as_str(),
                "STELLAR_DETECTION_FAILED"
            );
        }
    }

    #[test]
    fn unknown_version_is_a_reviewable_finding_and_stderr_versions_are_supported() {
        let mut capture = command(CommandStatus::Exited { code: 0 }, "unrecognized", None);
        let tool = inspect_tool("stellar", capture.clone());
        assert_eq!(tool.state, ToolState::UnrecognizedVersion);
        assert_eq!(
            finding(&tool).unwrap().id.as_str(),
            "STELLAR_VERSION_UNRECOGNIZED"
        );
        capture.record.stderr = "stellar 27.0.0".into();
        assert!(matches!(
            inspect_tool("stellar", capture).state,
            ToolState::Detected { .. }
        ));
    }
}
