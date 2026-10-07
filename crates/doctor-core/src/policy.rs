use crate::{Config, Finding, Severity, Verdict};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum ExitCode {
    Completed = 0,
    PolicyFailed = 1,
    InvalidInput = 2,
    ExternalFailure = 3,
    InternalError = 4,
}

impl ExitCode {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    InvalidInput,
    ExternalFailure,
    InternalFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyDecision {
    pub verdict: Verdict,
    pub exit_code: ExitCode,
}

/// Evaluate findings only after analysis completes. An empty finding list is not
/// evidence that any checks ran; the caller must supply the actual run outcome.
pub fn evaluate_policy(findings: &[Finding], config: &Config) -> PolicyDecision {
    if findings
        .iter()
        .any(|finding| config.fail_on.contains(&finding.severity))
    {
        PolicyDecision {
            verdict: Verdict::NotReady,
            exit_code: ExitCode::PolicyFailed,
        }
    } else {
        let has_concerns = findings
            .iter()
            .any(|finding| finding.severity != Severity::Info);
        PolicyDecision {
            verdict: if has_concerns {
                Verdict::ChecksCompletedWithWarnings
            } else {
                Verdict::ReadyForManualReview
            },
            exit_code: ExitCode::Completed,
        }
    }
}

/// Input, external execution, and internal failures take precedence over findings.
/// A completed nonzero build/test becomes a finding in later phases; failure to
/// execute a required tool is an external failure.
pub fn exit_code(outcome: RunOutcome, findings: &[Finding], config: &Config) -> ExitCode {
    match outcome {
        RunOutcome::Completed => evaluate_policy(findings, config).exit_code,
        RunOutcome::InvalidInput => ExitCode::InvalidInput,
        RunOutcome::ExternalFailure => ExitCode::ExternalFailure,
        RunOutcome::InternalFailure => ExitCode::InternalError,
    }
}
