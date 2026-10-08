//! Unified report models for Sortralis multi-format reporting.
use crate::{CommandResult, DetectedPackage, Finding, Severity, Verdict, REPORT_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Terminal,
    Json,
    Sarif,
    Html,
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "terminal" => Ok(Self::Terminal),
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            "html" => Ok(Self::Html),
            other => Err(format!(
                "invalid report format '{other}'; supported: terminal, json, sarif, html"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportStep {
    pub name: String,
    pub status: String,
    pub description: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_millis: Option<u64>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_findings: usize,
    pub breaking_count: usize,
    pub warning_count: usize,
    pub manual_review_count: usize,
    pub info_count: usize,
    pub total_steps: usize,
    pub passed_steps: usize,
    pub failed_steps: usize,
    pub other_steps: usize,
}

impl ReportSummary {
    pub fn new(findings: &[Finding], steps: &[ReportStep]) -> Self {
        let mut breaking_count = 0;
        let mut warning_count = 0;
        let mut manual_review_count = 0;
        let mut info_count = 0;

        for f in findings {
            match f.severity {
                Severity::Breaking => breaking_count += 1,
                Severity::Warning => warning_count += 1,
                Severity::ManualReview => manual_review_count += 1,
                Severity::Info => info_count += 1,
            }
        }

        let mut passed_steps = 0;
        let mut failed_steps = 0;
        let mut other_steps = 0;

        for s in steps {
            let status_lower = s.status.to_ascii_lowercase();
            if status_lower.contains("pass") {
                passed_steps += 1;
            } else if status_lower.contains("fail") {
                failed_steps += 1;
            } else {
                other_steps += 1;
            }
        }

        Self {
            total_findings: findings.len(),
            breaking_count,
            warning_count,
            manual_review_count,
            info_count,
            total_steps: steps.len(),
            passed_steps,
            failed_steps,
            other_steps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub repository_path: PathBuf,
    pub verdict: Verdict,
    pub exit_code: u8,
    pub summary: ReportSummary,
    pub packages: Vec<DetectedPackage>,
    pub tools: Vec<ToolInfo>,
    pub steps: Vec<ReportStep>,
    pub findings: Vec<Finding>,
    pub command_results: Vec<CommandResult>,
    pub disclaimer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportParams {
    pub repository_path: PathBuf,
    pub verdict: Verdict,
    pub exit_code: u8,
    pub packages: Vec<DetectedPackage>,
    pub tools: Vec<ToolInfo>,
    pub steps: Vec<ReportStep>,
    pub findings: Vec<Finding>,
    pub command_results: Vec<CommandResult>,
}

impl Report {
    pub fn new(params: ReportParams) -> Self {
        let summary = ReportSummary::new(&params.findings, &params.steps);
        let mut report = Self {
            schema_version: REPORT_SCHEMA_VERSION.into(),
            tool_name: "sortralis".into(),
            tool_version: env!("CARGO_PKG_VERSION").into(),
            repository_path: params.repository_path,
            verdict: params.verdict,
            exit_code: params.exit_code,
            summary,
            packages: params.packages,
            tools: params.tools,
            steps: params.steps,
            findings: params.findings,
            command_results: params.command_results,
            disclaimer: "Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.".into(),
        };
        report.sort_deterministically();
        report
    }

    /// Ensure deterministic ordering across all findings, packages, and tools.
    pub fn sort_deterministically(&mut self) {
        self.findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
                .then_with(|| {
                    let a_path = a.evidence.first().and_then(|e| e.path.as_ref());
                    let b_path = b.evidence.first().and_then(|e| e.path.as_ref());
                    a_path.cmp(&b_path)
                })
                .then_with(|| {
                    let a_line = a.evidence.first().and_then(|e| e.line);
                    let b_line = b.evidence.first().and_then(|e| e.line);
                    a_line.cmp(&b_line)
                })
                .then_with(|| a.summary.cmp(&b.summary))
        });
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
        self.tools.sort_by(|a, b| a.name.cmp(&b.name));
    }
}
