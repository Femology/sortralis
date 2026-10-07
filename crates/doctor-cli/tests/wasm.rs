#![allow(clippy::unwrap_used)]
use std::{fs, process::Command};
#[test]
fn wasm_cli_rejects_invalid_files_and_reports_missing_tool_with_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("path with spaces.wasm");
    fs::write(&path, b"invalid wasm").unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_sud"))
        .arg("wasm")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    fs::write(&path, b"\0asm\x01\0\0\0").unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_sud"))
        .arg("wasm")
        .arg(&path)
        .arg("--json")
        .env("PATH", dir.path())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(3));
    assert!(String::from_utf8(missing.stderr)
        .unwrap()
        .contains("not installed"));
    assert!(missing.stdout.is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"\0asm\x01\0\0\0");
    let help = Command::new(env!("CARGO_BIN_EXE_sud"))
        .args(["wasm", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout).unwrap().contains("--json"));
}
