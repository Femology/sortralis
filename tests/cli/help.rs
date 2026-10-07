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
    for command in ["scan", "compare", "check", "explain"] {
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
