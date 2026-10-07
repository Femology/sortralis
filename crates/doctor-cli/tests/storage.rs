#![allow(clippy::unwrap_used)]
use std::{path::PathBuf, process::Command};
fn compare(before: &str, after: &str, json: bool) -> std::process::Output {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/storage");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sortralis"));
    command
        .arg("compare-source")
        .arg("--before")
        .arg(root.join(before))
        .arg("--after")
        .arg(root.join(after));
    if json {
        command.arg("--json");
    }
    command.output().unwrap()
}
#[test]
fn storage_comparison_outputs_human_records_and_json_objects() {
    let output = compare("identical", "changed-field-type", false);
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("STORAGE_CONTRACT_TYPE_CHANGED [MIGRATION_LIKELY]"));
    assert!(text.contains("Unchanged source types do not prove"));
    assert!(text.contains("lib.rs:"));
    let output = compare("identical", "removed-field", true);
    assert!(output.status.success(), "{output:?}");
    let diff: doctor_core::storage::StorageDiff = serde_json::from_slice(&output.stdout).unwrap();
    assert!(diff
        .records
        .iter()
        .any(|r| r.classification == doctor_core::storage::StorageClassification::MigrationLikely));
    assert!(output.stderr.is_empty());
    let identical = compare("identical", "identical", true);
    let diff: doctor_core::storage::StorageDiff =
        serde_json::from_slice(&identical.stdout).unwrap();
    assert!(diff.records.is_empty());
}
#[test]
fn invalid_snapshot_arguments_and_paths_fail() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("compare-source")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let output = compare("missing-snapshot", "identical", false);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
