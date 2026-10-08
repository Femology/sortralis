#![allow(clippy::unwrap_used)]

use doctor_core::interface::{AnalysisSource, InterfaceClassification, InterfaceDiff};
use std::{path::PathBuf, process::Command};

fn compare_cmd(before: &str, after: &str, json: bool) -> std::process::Output {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/git-diff");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sortralis"));
    cmd.arg("compare")
        .arg("--before")
        .arg(root.join(before))
        .arg("--after")
        .arg(root.join(after));
    if json {
        cmd.arg("--json");
    }
    cmd.output().unwrap()
}

#[test]
fn test_cli_compare_breaking_fixture_returns_exit_code_1() {
    let output = compare_cmd("before", "after", false);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Contract interface diff [source_approximation]"));
    assert!(text.contains("NOTICE: Source-derived approximation"));
    assert!(text.contains("[BREAKING] FUNCTION_REMOVED"));
    assert!(text.contains("[BREAKING] FUNCTION_RETURN_TYPE_CHANGED"));
    assert!(text.contains("[BREAKING] STRUCT_FIELD_TYPE_CHANGED"));
    assert!(text.contains("[NON_BREAKING] FUNCTION_ADDED"));
    assert!(text.contains("Verdict: NOT READY (breaking interface changes detected)"));
}

#[test]
fn test_cli_compare_identical_fixture_returns_exit_code_0() {
    let output = compare_cmd("before", "before", false);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Identical contract interface: zero public function, type, error, or event changes detected."));
    assert!(text.contains("Verdict: CHECKS_COMPLETED (no breaking interface changes)"));
}

#[test]
fn test_cli_compare_json_output_and_analysis_source_label() {
    let output = compare_cmd("before", "after", true);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let diff: InterfaceDiff = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(diff.analysis_source, AnalysisSource::SourceApproximation);
    assert!(diff.has_breaking_changes);
    assert!(diff
        .records
        .iter()
        .any(|r| r.classification == InterfaceClassification::Breaking));
}

#[test]
fn test_cli_compare_invalid_paths_fail() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sortralis"));
    cmd.arg("compare")
        .arg("--before")
        .arg("/non/existent/path/1")
        .arg("--after")
        .arg("/non/existent/path/2");
    let output = cmd.output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
