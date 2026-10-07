//! Verification results contain only bounded, redacted presentation evidence.
use crate::{CommandResult, ExitCode, Finding, REPORT_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Passed,
    Failed,
    MissingTool,
    TimedOut,
    ExecutionError,
    Skipped,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationStep {
    pub name: String,
    pub status: VerificationStatus,
    pub command: Option<CommandResult>,
    pub displayed_command: Option<String>,
    pub duration_millis: u64,
    pub exit_code: Option<i32>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub reason: Option<String>,
    pub finding: Option<Finding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    pub schema_version: String,
    pub tool_version: String,
    pub repository_path: PathBuf,
    pub steps: Vec<VerificationStep>,
    pub exit_code: ExitCode,
    pub scope: String,
}
impl VerificationReport {
    pub fn new(repository_path: PathBuf) -> Self {
        Self { schema_version: REPORT_SCHEMA_VERSION.into(), tool_version: env!("CARGO_PKG_VERSION").into(), repository_path, steps: vec![], exit_code: ExitCode::Completed, scope: "Explicit verification executes project Cargo tests/builds with default features. Captured output is bounded and heuristically redacted. Failed project commands are report results. No security audit or deployment-safety guarantee.".into() }
    }
}
