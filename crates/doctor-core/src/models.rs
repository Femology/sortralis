use crate::RuleId;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, num::NonZeroU32, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Info,
    Warning,
    Breaking,
    ManualReview,
}

impl Severity {
    /// Display/sort priority only. Blocking policy uses exact set membership.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::ManualReview => 2,
            Self::Breaking => 3,
        }
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Category {
    Toolchain,
    Dependency,
    Source,
    Storage,
    Interface,
    Event,
    Auth,
    Build,
    Test,
    Wasm,
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: RuleId,
    pub title: String,
    pub severity: Severity,
    pub category: Category,
    pub summary: String,
    pub why_it_matters: String,
    pub evidence: Vec<Evidence>,
    pub recommendation: String,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub path: Option<PathBuf>,
    /// One-based source line, when known; zero is rejected during deserialization.
    pub line: Option<NonZeroU32>,
    pub message: String,
    pub command: Option<CommandResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub status: CommandStatus,
    pub stdout: String,
    pub stderr: String,
    pub elapsed_millis: u64,
}

/// Recorded outcomes only. This type does not run or simulate commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandStatus {
    Exited { code: i32 },
    Terminated { signal: Option<i32> },
    TimedOut,
    FailedToStart { message: String },
    ExecutionFailed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedPackage {
    pub name: String,
    pub manifest_path: PathBuf,
    pub soroban_sdk_requirement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    ReadyForManualReview,
    ChecksCompletedWithWarnings,
    NotReady,
}

/// Populated by future analyzers; creating this structure does not perform checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub schema_version: String,
    pub tool_version: String,
    pub repository_path: PathBuf,
    pub detected_packages: Vec<DetectedPackage>,
    pub findings: Vec<Finding>,
    pub command_results: Vec<CommandResult>,
    pub verdict: Verdict,
}
