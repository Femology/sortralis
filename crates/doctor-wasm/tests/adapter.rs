#![allow(clippy::unwrap_used)]
use doctor_core::{CommandResult, CommandStatus};
use doctor_runner::{
    CapturedCommand, CommandRunner, CommandSpec, InfoOperation, RunnerError, StellarCli,
    StellarError, SystemRunner,
};
use doctor_wasm::{inspect_with, SectionStatus, WasmError};
use std::{cell::RefCell, fs, io, time::Duration};
struct Fake {
    calls: RefCell<Vec<CommandSpec>>,
    fail: Option<CommandStatus>,
    malformed: bool,
    absent: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: RefCell::new(vec![]),
            fail: None,
            malformed: false,
            absent: false,
        }
    }
}
impl CommandRunner for Fake {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        self.calls.borrow_mut().push(request.clone());
        let version = request.args[0] == "--version";
        let operation = request
            .args
            .get(2)
            .map(|a| a.to_string_lossy())
            .unwrap_or_default();
        let stdout: &[u8] = if version {
            include_bytes!("../../../fixtures/stellar-cli-27/version.txt")
        } else if self.malformed {
            b"not JSON or a hash"
        } else {
            match operation.as_ref() {
                "interface" => include_bytes!("../../../fixtures/stellar-cli-27/interface.json"),
                "meta" => include_bytes!("../../../fixtures/stellar-cli-27/meta.json"),
                "env-meta" => include_bytes!("../../../fixtures/stellar-cli-27/env-meta.json"),
                "hash" => include_bytes!("../../../fixtures/stellar-cli-27/hash.txt"),
                _ => b"build double output",
            }
        };
        let absent = self.absent && matches!(operation.as_ref(), "meta" | "env-meta");
        let status = self.fail.clone().unwrap_or(CommandStatus::Exited {
            code: if absent { 1 } else { 0 },
        });
        let stderr = if absent {
            include_bytes!("../../../fixtures/stellar-cli-27/absent-meta.stderr").to_vec()
        } else {
            b"retained stderr\0\xff".to_vec()
        };
        Ok(CapturedCommand {
            request: request.clone(),
            record: CommandResult {
                program: request.program.to_string_lossy().into(),
                args: request
                    .args
                    .iter()
                    .map(|s| s.to_string_lossy().into())
                    .collect(),
                working_directory: request.working_directory.clone(),
                status: status.clone(),
                stdout: String::from_utf8_lossy(stdout).into(),
                stderr: String::from_utf8_lossy(&stderr).into(),
                elapsed_millis: 1,
            },
            stdout_bytes: stdout.into(),
            stderr_bytes: stderr,
            spawn_error_kind: if matches!(status, CommandStatus::FailedToStart { .. }) {
                Some(io::ErrorKind::NotFound)
            } else {
                None
            },
        })
    }
}
fn file(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let p = dir.path().join("contract with spaces;literal.wasm");
    fs::write(&p, b"\0asm\x01\0\0\0").unwrap();
    p
}
#[test]
fn injected_inspection_preserves_records_and_single_path_argument_without_network() {
    let dir = tempfile::tempdir().unwrap();
    let p = file(&dir);
    let fake = Fake::new();
    let cli = StellarCli::new(&fake, Duration::from_secs(7));
    let report = inspect_with(&p, dir.path(), &cli).unwrap();
    assert_eq!(report.stellar_cli_version, "27.0.0");
    assert_eq!(report.commands.len(), 5);
    assert_eq!(report.interface.entries[0]["function_v0"]["name"], "add");
    assert_eq!(
        report.build_info.status,
        doctor_wasm::BuildInfoStatus::SkippedNetworkPolicy
    );
    for request in fake.calls.borrow().iter().skip(1) {
        assert_eq!(request.args[4], p.as_os_str());
        assert_eq!(request.timeout, Duration::from_secs(7));
        assert!(!request
            .args
            .iter()
            .any(|a| a == "build" || a == "--contract-id" || a == "--wasm-hash"));
    }
    let encoded = serde_json::to_vec(&report).unwrap();
    assert_eq!(
        serde_json::from_slice::<doctor_wasm::WasmInspection>(&encoded).unwrap(),
        report
    );
    assert!(!dir.path().join("literal.wasm").exists());
}
#[test]
fn errors_missing_timeout_nonzero_and_malformed_preserve_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let p = file(&dir);
    for status in [
        CommandStatus::Exited { code: 17 },
        CommandStatus::TimedOut,
        CommandStatus::FailedToStart {
            message: "missing".into(),
        },
    ] {
        let mut fake = Fake::new();
        fake.fail = Some(status.clone());
        let error = inspect_with(
            &p,
            dir.path(),
            &StellarCli::new(&fake, Duration::from_secs(1)),
        )
        .unwrap_err();
        assert_eq!(error.exit_code(), 3);
        match error {
            WasmError::Stellar(StellarError::Missing(c) | StellarError::Failed(c)) => {
                assert_eq!(c.record.status, status);
                assert_eq!(c.stderr_bytes, b"retained stderr\0\xff");
            }
            e => panic!("{e}"),
        }
    }
    let mut fake = Fake::new();
    fake.malformed = true;
    assert!(matches!(
        inspect_with(
            &p,
            dir.path(),
            &StellarCli::new(&fake, Duration::from_secs(1))
        ),
        Err(WasmError::Output {
            operation: "interface",
            ..
        })
    ));
}
#[test]
fn invalid_files_fail_before_any_process_and_absent_meta_is_not_fake_success() {
    let dir = tempfile::tempdir().unwrap();
    let p = file(&dir);
    let fake = Fake::new();
    fs::write(&p, b"not wasm").unwrap();
    assert!(inspect_with(
        &p,
        dir.path(),
        &StellarCli::new(&fake, Duration::from_secs(1))
    )
    .is_err());
    assert!(fake.calls.borrow().is_empty());
    fs::write(&p, b"\0asm\x01\0\0\0").unwrap();
    let mut fake = Fake::new();
    fake.absent = true;
    let report = inspect_with(
        &p,
        dir.path(),
        &StellarCli::new(&fake, Duration::from_secs(1)),
    )
    .unwrap();
    assert_eq!(report.meta.status, SectionStatus::Absent);
    assert_eq!(report.env_meta.status, SectionStatus::Absent);
    assert_eq!(report.commands[2].status, CommandStatus::Exited { code: 1 });
    assert!(!report.commands[2].stderr.is_empty());
}
#[test]
fn adapter_build_and_version_are_centralized_and_remote_build_info_never_spawns() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Fake::new();
    let cli = StellarCli::new(&fake, Duration::from_secs(1));
    cli.version(dir.path()).unwrap();
    cli.contract_build(dir.path()).unwrap();
    assert_eq!(fake.calls.borrow()[1].args, ["contract", "build"]);
    let p = file(&dir);
    assert!(matches!(
        cli.info(InfoOperation::Build, &p, dir.path()),
        Err(StellarError::NetworkForbidden)
    ));
    assert_eq!(fake.calls.borrow().len(), 2);
}
#[test]
fn actual_missing_executable_is_a_clear_structured_error() {
    let dir = tempfile::tempdir().unwrap();
    let cli = StellarCli::new(&SystemRunner, Duration::from_secs(1))
        .with_executable(dir.path().join("missing-stellar").into_os_string());
    let error = cli.version(dir.path()).unwrap_err();
    assert!(error.to_string().contains("not installed"));
    assert!(matches!(error, StellarError::Missing(_)));
}
#[test]
fn local_real_cli_version_and_hash_smoke_when_installed() {
    let dir = tempfile::tempdir().unwrap();
    let p = file(&dir);
    let cli = StellarCli::new(&SystemRunner, Duration::from_secs(10));
    match cli.version(dir.path()) {
        Err(StellarError::Missing(_)) => {
            eprintln!("SKIP: Stellar CLI is absent");
            return;
        }
        Ok(_) => {}
        Err(e) => panic!("installed CLI failed: {e}"),
    }
    let hash = cli.info(InfoOperation::Hash, &p, dir.path()).unwrap();
    assert_eq!(
        doctor_wasm::parse_hash(&hash.stdout_bytes).unwrap().len(),
        64
    );
    assert_eq!(fs::read(p).unwrap(), b"\0asm\x01\0\0\0");
}
#[test]
#[ignore = "Opt-in: SUD_TEST_WASM must point to a real local Soroban contract; requires Stellar CLI"]
fn real_full_inspection_of_local_contract_with_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let source = std::env::var_os("SUD_TEST_WASM").unwrap();
    let p = dir.path().join("real contract with spaces.wasm");
    fs::copy(source, &p).unwrap();
    let before = fs::read(&p).unwrap();
    let report = doctor_wasm::inspect(&p, dir.path()).unwrap();
    assert!(!report.interface.entries.is_empty());
    assert_eq!(report.commands.len(), 5);
    assert_eq!(
        report.build_info.status,
        doctor_wasm::BuildInfoStatus::SkippedNetworkPolicy
    );
    assert_eq!(fs::read(p).unwrap(), before);
}
