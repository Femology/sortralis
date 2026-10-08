#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

#[test]
fn test_cli_check_format_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("check")
        .arg(fixture("non-soroban"))
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"schema_version\": \"1.0\""));
    assert!(stdout.contains("\"tool_name\": \"sortralis\""));
    assert!(stdout.contains("\"verdict\":"));
    assert!(stdout.contains("\"summary\":"));
}

#[test]
fn test_cli_check_format_sarif() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("check")
        .arg(fixture("non-soroban"))
        .args(["--format", "sarif"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"version\": \"2.1.0\""));
    assert!(stdout.contains("sarif-schema-2.1.0.json"));
    assert!(stdout.contains("\"name\": \"sortralis\""));
}

#[test]
fn test_cli_check_format_html_output_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let report_file = temp_dir.path().join("custom_report.html");

    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("check")
        .arg(fixture("non-soroban"))
        .args([
            "--format",
            "html",
            "--output",
            report_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{:?}", output);
    assert!(report_file.is_file());

    let content = fs::read_to_string(&report_file).unwrap();
    assert!(content.contains("<!DOCTYPE html>"));
    assert!(content.contains("Sortralis Upgrade Report"));
    assert!(content.contains("Passing Sortralis is not a security audit"));
}

#[test]
fn test_cli_check_format_html_default_output() {
    let temp_dir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .current_dir(temp_dir.path())
        .arg("check")
        .arg(fixture("non-soroban"))
        .args(["--format", "html"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{:?}", output);
    let default_file = temp_dir.path().join("report.html");
    assert!(default_file.is_file());

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("HTML report written to report.html"));

    let content = fs::read_to_string(&default_file).unwrap();
    assert!(content.contains("<!DOCTYPE html>"));
    assert!(content.contains("Sortralis Upgrade Report"));
}

#[test]
fn test_cli_check_format_terminal_output_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let report_file = temp_dir.path().join("report.txt");

    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("check")
        .arg(fixture("non-soroban"))
        .args([
            "--format",
            "terminal",
            "--output",
            report_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{:?}", output);
    assert!(report_file.is_file());

    let content = fs::read_to_string(&report_file).unwrap();
    assert!(content.contains("Workspace:"));
    assert!(content.contains("Summary:"));
    assert!(content.contains("Tests:"));
    assert!(content.contains("Build:"));
}
