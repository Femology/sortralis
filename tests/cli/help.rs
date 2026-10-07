#![allow(clippy::unwrap_used)]

use std::process::Command;

#[test]
fn help_succeeds_and_lists_all_phase_one_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for command in ["scan", "compare", "check", "explain"] {
        assert!(
            stdout
                .lines()
                .any(|line| line.split_whitespace().next() == Some(command)),
            "missing command {command}: {stdout}"
        );
    }
}

#[test]
fn version_reports_the_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        concat!("sortralis ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unfinished_commands_fail_explicitly_without_success_output() {
    for command in ["compare", "check"] {
        let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
            .arg(command)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "{command} must not report success"
        );
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(command), "{stderr}");
        assert!(stderr.contains("not implemented yet"), "{stderr}");
    }
}

#[test]
fn environment_mode_reports_missing_tools_with_structured_ids() {
    let directory =
        std::env::temp_dir().join(format!("sortralis-empty-path-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .args(["scan", "--environment"])
        .env("PATH", &directory)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("repository analysis is not implemented"));
    for program in ["RUSTC", "CARGO", "STELLAR"] {
        assert!(
            stderr.contains(&format!("{program}_NOT_INSTALLED")),
            "{stderr}"
        );
    }
    std::fs::remove_dir(directory).unwrap();
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}
#[test]
fn scan_prints_workspace_and_multiple_contract_candidates() {
    let path = fixture("healthy-v28");
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("scan")
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("Workspace: {}", path.display())));
    assert!(stdout.contains("Workspace packages: 2"));
    assert!(stdout.contains("Soroban contract candidates: 2"));
    for name in ["alpha", "beta"] {
        assert!(stdout.contains(name));
    }
    assert!(stdout.contains("=28.0.0"));
    assert!(stdout.contains("Full upgrade analysis is not implemented"));
}
#[test]
fn scan_no_soroban_is_a_completed_discovery() {
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("scan")
        .arg(fixture("non-soroban"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("No Soroban contract candidates found"));
}
#[test]
fn scan_non_cargo_input_and_missing_cargo_fail_usefully() {
    let fixture = fixture("non-soroban");
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("scan")
        .arg(fixture.join("src"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("no Cargo.toml"));
    let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("scan")
        .arg(&fixture)
        .env("PATH", fixture.join("src"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Cargo metadata failed"));
}

#[test]
fn explain_returns_registry_docs_and_manual_rule_status() {
    for id in [
        "SDK28_REMOVED_EXPORT_ARGUMENT",
        "CUSTOM_ACCOUNT_EXECUTABLE_REVIEW",
        "SDK28_EVENT_SHAPE_REVIEW",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
            .args(["explain", id])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(id));
        assert!(stdout.contains("https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/"));
        if id == "SDK28_EVENT_SHAPE_REVIEW" {
            assert!(stdout.contains("Manual only"));
        }
    }
}
#[test]
fn explain_unknown_or_missing_rule_returns_invalid_input() {
    for args in [
        vec!["explain"],
        vec!["explain", "UNKNOWN_RULE"],
        vec!["explain", "sdk28_removed_export_argument"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_sortralis"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
    }
}
