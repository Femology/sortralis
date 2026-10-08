#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("matrix")
        .join(name)
}

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .args(args)
        .output()
        .unwrap();

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (code, stdout, stderr)
}

fn parse_report_json(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("CLI stdout must be valid JSON")
}

fn contract_findings(json: &Value) -> Vec<&Value> {
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["category"] != "TOOLCHAIN")
        .collect()
}

fn stable_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(stable_value).collect()),
        Value::Object(map) => {
            let mut stable = serde_json::Map::new();
            for (key, child) in map {
                if key != "elapsed_millis" {
                    stable.insert(key.clone(), stable_value(child));
                }
            }
            Value::Object(stable)
        }
        _ => value.clone(),
    }
}

// 1. p28-healthy: No P28 error findings; completed with exit code 0
#[test]
fn test_fixture_01_p28_healthy() {
    let path = fixture_dir("p28-healthy");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 0, "p28-healthy should exit 0");

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["breaking_count"], 0);
    let findings = contract_findings(&json);
    assert!(
        findings.is_empty(),
        "p28-healthy must have no contract/P28 findings, found: {findings:?}"
    );
}

// 2. p27-legacy-upgrade-api: P28-API-001 at exact source location
#[test]
fn test_fixture_02_p27_legacy_upgrade_api() {
    let path = fixture_dir("p27-legacy-upgrade-api");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 1, "p27-legacy-upgrade-api should exit 1 (Breaking)");

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["breaking_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-API-001");
    assert_eq!(findings[0]["severity"], "BREAKING");
    assert_eq!(findings[0]["evidence"][0]["line"], 11);
}

// 3. p27-legacy-deploy-api: P28 deploy finding
#[test]
fn test_fixture_03_p27_legacy_deploy_api() {
    let path = fixture_dir("p27-legacy-deploy-api");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 1, "p27-legacy-deploy-api should exit 1 (Breaking)");

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["breaking_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-DEPLOY-001");
    assert_eq!(findings[0]["severity"], "BREAKING");
}

// 4. p28-sparse-event-risk: Event finding
#[test]
fn test_fixture_04_p28_sparse_event_risk() {
    let path = fixture_dir("p28-sparse-event-risk");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 0,
        "p28-sparse-event-risk should exit 0 (ManualReview)"
    );

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["manual_review_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-EVENT-001");
    assert_eq!(findings[0]["category"], "EVENT");
}

// 5. p28-contracttrait-invalid: Macro finding only when AST can prove invalid use
#[test]
fn test_fixture_05_p28_contracttrait_invalid() {
    let path = fixture_dir("p28-contracttrait-invalid");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 1,
        "p28-contracttrait-invalid should exit 1 (Breaking)"
    );

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["breaking_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-MACRO-001");
    assert_eq!(findings[0]["severity"], "BREAKING");
}

// 6. p28-internal-spec-reference: Spec-symbol finding
#[test]
fn test_fixture_06_p28_internal_spec_reference() {
    let path = fixture_dir("p28-internal-spec-reference");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 1,
        "p28-internal-spec-reference should exit 1 (Breaking)"
    );

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["breaking_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-SPEC-001");
    assert_eq!(findings[0]["severity"], "BREAKING");
}

// 7. upgrade-auth-direct: Safe auth observation / no missing-auth warning
#[test]
fn test_fixture_07_upgrade_auth_direct() {
    let path = fixture_dir("upgrade-auth-direct");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 0, "upgrade-auth-direct should exit 0");

    let json = parse_report_json(&stdout);
    let findings = contract_findings(&json);
    assert!(
        findings.is_empty(),
        "upgrade-auth-direct should have no contract findings"
    );
}

// 8. upgrade-auth-none: Expected REVIEW_REQUIRED or documented policy result
#[test]
fn test_fixture_08_upgrade_auth_none() {
    let path = fixture_dir("upgrade-auth-none");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 0,
        "upgrade-auth-none should exit 0 under default policy"
    );

    let json = parse_report_json(&stdout);
    assert_eq!(json["summary"]["manual_review_count"], 1);
    let findings = contract_findings(&json);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["id"], "P28-AUTH-001");
    assert_eq!(findings[0]["category"], "AUTH");
}

// 9. storage-v1 and storage-v2: Expected migration-risk diff
#[test]
fn test_fixture_09_storage_diff() {
    let v1 = fixture_dir("storage-v1");
    let v2 = fixture_dir("storage-v2");
    let (code, stdout, _) = run_cli(&[
        "compare-source",
        "--before",
        v1.to_str().unwrap(),
        "--after",
        v2.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, 0, "compare-source should exit 0");

    let json = parse_report_json(&stdout);
    let records = json["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], "STORAGE_CONTRACT_TYPE_CHANGED");
    assert_eq!(records[0]["classification"], "MIGRATION_LIKELY");
}

// 10. interface-v1 and interface-v2: Expected breaking interface diff
#[test]
fn test_fixture_10_interface_diff() {
    let v1 = fixture_dir("interface-v1");
    let v2 = fixture_dir("interface-v2");
    let (code, stdout, _) = run_cli(&[
        "compare",
        "--before",
        v1.to_str().unwrap(),
        "--after",
        v2.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, 1, "breaking interface diff should exit 1");

    let json = parse_report_json(&stdout);
    assert_eq!(json["has_breaking_changes"], true);
    let records = json["records"].as_array().unwrap();
    assert!(records.iter().any(|r| r["id"] == "FUNCTION_REMOVED"));
}

// 11. no-soroban: Expected clear user error
#[test]
fn test_fixture_11_no_soroban() {
    let path = fixture_dir("no-soroban").join("src");
    let (code, _, stderr) = run_cli(&["check", path.to_str().unwrap()]);
    assert_eq!(
        code, 2,
        "no-soroban directory missing Cargo.toml should exit 2 (InvalidInput)"
    );
    assert!(
        stderr.contains("no Cargo.toml"),
        "stderr must report clear user error"
    );
}

// 12. malformed-rust: Expected structured parse finding/error behavior
#[test]
fn test_fixture_12_malformed_rust() {
    let path = fixture_dir("malformed-rust");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        code, 3,
        "malformed rust syntax should exit 3 (ExternalFailure)"
    );

    let json = parse_report_json(&stdout);
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["id"] == "SOURCE_ANALYSIS_FAILED"));
}

// 13. multiline-formatting: Proves detection is not dependent on formatting
#[test]
fn test_fixture_13_multiline_formatting() {
    let path = fixture_dir("multiline-formatting");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 1, "multiline-formatting should exit 1");

    let json = parse_report_json(&stdout);
    let findings = contract_findings(&json);
    assert!(findings.iter().any(|f| f["id"] == "P28-API-001"));
}

// 14. comments-and-strings: PHASE GATE - Zero known false positives
#[test]
fn test_fixture_14_comments_and_strings_phase_gate() {
    let path = fixture_dir("comments-and-strings");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 0, "comments-and-strings should exit 0");

    let json = parse_report_json(&stdout);
    let findings = contract_findings(&json);
    assert!(
        findings.is_empty(),
        "PHASE GATE FAILED: False positive findings detected in comments-and-strings: {:?}",
        findings
    );
}

// 15. multi-contract-workspace: Expected deterministic results for all contracts
#[test]
fn test_fixture_15_multi_contract_workspace() {
    let path = fixture_dir("multi-contract-workspace");
    let (code, stdout, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(code, 0, "multi-contract-workspace should exit 0");

    let json = parse_report_json(&stdout);
    let packages = json["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0]["name"], "contract-a");
    assert_eq!(packages[1]["name"], "contract-b");
    let findings = contract_findings(&json);
    assert!(findings.is_empty());
}

// Determinism test: Run the matrix suite twice and confirm deterministic output
#[test]
fn test_matrix_determinism_suite() {
    let targets = [
        "p28-healthy",
        "p27-legacy-upgrade-api",
        "p27-legacy-deploy-api",
        "p28-sparse-event-risk",
        "p28-contracttrait-invalid",
        "p28-internal-spec-reference",
        "upgrade-auth-direct",
        "upgrade-auth-none",
        "comments-and-strings",
        "multiline-formatting",
        "multi-contract-workspace",
    ];

    for target in targets {
        let path = fixture_dir(target);

        let (code1, stdout1, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);
        let (code2, stdout2, _) = run_cli(&["check", path.to_str().unwrap(), "--format", "json"]);

        assert_eq!(
            code1, code2,
            "exit codes must match across runs for {target}"
        );

        let json1 = parse_report_json(&stdout1);
        let json2 = parse_report_json(&stdout2);

        // Findings, summary, verdict, exit_code, packages must match deterministically
        assert_eq!(
            json1["verdict"], json2["verdict"],
            "verdict must be deterministic for {target}"
        );
        assert_eq!(
            json1["summary"], json2["summary"],
            "summary counts must be deterministic for {target}"
        );
        assert_eq!(
            stable_value(&json1["findings"]),
            stable_value(&json2["findings"]),
            "findings must be deterministic for {target}"
        );
        assert_eq!(
            json1["packages"], json2["packages"],
            "packages must be deterministic for {target}"
        );
    }
}
