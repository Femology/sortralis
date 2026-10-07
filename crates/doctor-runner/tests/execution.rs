#![allow(clippy::unwrap_used)]

use doctor_core::CommandStatus;
use doctor_runner::{execute, CommandSpec};
use std::{
    path::PathBuf,
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};

fn helper() -> &'static PathBuf {
    static HELPER: OnceLock<PathBuf> = OnceLock::new();
    HELPER.get_or_init(|| {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/child.rs");
        let target = std::env::temp_dir().join(format!(
            "sortralis-runner-helper-{}{}",
            std::process::id(),
            std::env::consts::EXE_SUFFIX
        ));
        let output = Command::new("rustc")
            .args(["--edition", "2021"])
            .arg(source)
            .arg("-o")
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        target
    })
}

fn request(mode: &str) -> CommandSpec {
    CommandSpec {
        program: helper().as_os_str().to_owned(),
        args: vec![mode.into()],
        working_directory: std::env::current_dir().unwrap(),
        timeout: Duration::from_secs(5),
    }
}

#[test]
fn successful_command_records_identity_output_and_status() {
    let spec = request("success");
    let result = execute(&spec).unwrap();
    assert_eq!(result.record.status, CommandStatus::Exited { code: 0 });
    assert_eq!(result.record.program, spec.program.to_string_lossy());
    assert_eq!(result.record.args, ["success"]);
    assert_eq!(result.record.working_directory, spec.working_directory);
    assert_eq!(result.record.stdout, "standard output\n");
    assert_eq!(result.record.stderr, "standard error\n");
}

#[test]
fn failed_command_preserves_real_exit_code_and_both_streams() {
    let result = execute(&request("failure")).unwrap();
    assert_eq!(result.record.status, CommandStatus::Exited { code: 7 });
    assert_eq!(result.stdout_bytes, b"partial result\n");
    assert_eq!(result.stderr_bytes, b"real failure diagnostic\n");
}

#[test]
fn missing_binary_is_structured_and_is_not_a_panic() {
    let mut spec = request("success");
    spec.program = spec
        .working_directory
        .join("nonexistent-sortralis-binary")
        .into_os_string();
    let result = execute(&spec).unwrap();
    assert!(matches!(
        result.record.status,
        CommandStatus::FailedToStart { .. }
    ));
    assert_eq!(result.spawn_error_kind, Some(std::io::ErrorKind::NotFound));
    assert!(result.stdout_bytes.is_empty());
    assert!(result.stderr_bytes.is_empty());
}

#[test]
fn timeout_is_bounded_and_preserves_partial_output() {
    let mut spec = request("sleep");
    spec.timeout = Duration::from_millis(500);
    let start = Instant::now();
    let result = execute(&spec).unwrap();
    assert_eq!(result.record.status, CommandStatus::TimedOut);
    assert!(result.record.stdout.contains("started"));
    assert!(result.record.elapsed_millis >= 500);
    assert!(start.elapsed() < Duration::from_secs(4));
}

#[test]
fn large_output_drains_both_pipes_without_deadlocking() {
    let result = execute(&request("flood")).unwrap();
    assert_eq!(result.record.status, CommandStatus::Exited { code: 0 });
    assert_eq!(result.stdout_bytes, vec![b'a'; 256 * 1024]);
    assert_eq!(result.stderr_bytes, vec![b'b'; 256 * 1024]);
}

#[test]
fn non_utf8_output_is_preserved_as_bytes() {
    let result = execute(&request("bytes")).unwrap();
    assert_eq!(result.stdout_bytes, [0xff, 0x00, b'x']);
    assert_eq!(result.stderr_bytes, [0xfe, b'y']);
}

#[test]
fn shell_metacharacters_are_a_literal_argument_and_do_not_create_a_file() {
    let directory =
        std::env::temp_dir().join(format!("sortralis-injection-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let mut spec = request("echo");
    let payload =
        "literal; touch INJECTION_MARKER && $(touch INJECTION_MARKER) | cat > INJECTION_MARKER";
    spec.args.push(payload.into());
    spec.working_directory = directory.clone();
    let result = execute(&spec).unwrap();
    assert_eq!(result.record.status, CommandStatus::Exited { code: 0 });
    assert_eq!(result.record.stdout.trim_end(), payload);
    assert!(!directory.join("INJECTION_MARKER").exists());
    std::fs::remove_dir(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn timeout_also_covers_descendants_holding_pipes_after_parent_exit() {
    let mut spec = request("descendant");
    spec.timeout = Duration::from_millis(500);
    let start = Instant::now();
    let result = execute(&spec).unwrap();
    assert_eq!(result.record.status, CommandStatus::TimedOut);
    assert!(result.record.stdout.contains("descendant"));
    assert!(start.elapsed() < Duration::from_secs(4));
}

#[test]
fn invalid_working_directory_is_a_typed_error() {
    let mut spec = request("success");
    spec.working_directory = spec.working_directory.join("nonexistent-runner-directory");
    assert!(execute(&spec).is_err());
}

#[test]
fn zero_timeout_is_rejected_before_spawning() {
    let mut spec = request("success");
    spec.timeout = Duration::ZERO;
    assert!(matches!(
        execute(&spec),
        Err(doctor_runner::RunnerError::InvalidTimeout)
    ));
}
