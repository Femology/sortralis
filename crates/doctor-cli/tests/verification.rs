#![allow(clippy::unwrap_used)]
mod support;
use doctor_cli::verify::{verify_with, VerificationOptions, OUTPUT_LIMIT};
use doctor_core::{verification::*, CommandResult, CommandStatus, ExitCode};
use doctor_runner::{CapturedCommand, CommandRunner, CommandSpec, RunnerError, SystemRunner};
use std::{cell::RefCell, fs, io, path::PathBuf};
use support::Project;
struct Fake {
    calls: RefCell<Vec<CommandSpec>>,
    failure: Option<String>,
    missing: bool,
    no_artifact: bool,
    large: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: RefCell::new(vec![]),
            failure: None,
            missing: false,
            no_artifact: false,
            large: false,
        }
    }
}
impl CommandRunner for Fake {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        self.calls.borrow_mut().push(request.clone());
        if request.args[0] == "metadata" {
            return SystemRunner.execute(request);
        }
        let operation = request.args[0].to_string_lossy().into_owned();
        let build = request.args[0] == "contract" && request.args[1] == "build";
        let failed = self.failure.as_deref().is_some_and(|f| {
            f == operation || (f == "build" && build) || (build && request.args[3] == f)
        });
        if build && !failed && !self.missing && !self.no_artifact {
            let package = request.args[3].to_string_lossy().replace('-', "_");
            let dir = PathBuf::from(&request.args[5]);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{package}.wasm")), b"\0asm\x01\0\0\0").unwrap();
        }
        let version = request.args[0] == "--version";
        let info = request.args.get(1).is_some_and(|a| a == "info");
        let stdout = if version {
            "stellar 27.0.0".into()
        } else if info && request.args[2] == "hash" {
            "a".repeat(64)
        } else if info {
            "[]".into()
        } else if self.large {
            format!(
                "API_KEY = do-not-show\n{}\n",
                "large ordinary diagnostic\n".repeat(100_000)
            )
        } else {
            "ordinary command output\n".into()
        };
        let stderr = "token=not-for-display\nraw diagnostic retained\n".to_string();
        if self.failure.as_deref() == Some("runner-error") {
            return Err(RunnerError::WorkingDirectory {
                path: request.working_directory.clone(),
                source: io::Error::other("API_KEY=not-for-display"),
            });
        }
        let status = if self.failure.as_deref() == Some("timeout") {
            CommandStatus::TimedOut
        } else if self.missing {
            CommandStatus::FailedToStart {
                message: "missing binary".into(),
            }
        } else {
            CommandStatus::Exited {
                code: if failed { 101 } else { 0 },
            }
        };
        Ok(CapturedCommand {
            request: request.clone(),
            record: CommandResult {
                program: request.program.to_string_lossy().into(),
                args: request
                    .args
                    .iter()
                    .map(|a| a.to_string_lossy().into())
                    .collect(),
                working_directory: request.working_directory.clone(),
                status,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                elapsed_millis: 9,
            },
            stdout_bytes: stdout.into_bytes(),
            stderr_bytes: stderr.into_bytes(),
            spawn_error_kind: if self.missing {
                Some(io::ErrorKind::NotFound)
            } else {
                None
            },
        })
    }
}
#[test]
fn passing_multi_contract_order_and_fresh_outputs_are_recorded_and_cleaned() {
    let project = Project::copy("healthy-v28");
    let fake = Fake::new();
    let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
    assert_eq!(report.exit_code, ExitCode::Completed, "{report:#?}");
    assert_eq!(
        report.steps[..4]
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["Cargo discovery", "Format", "Tests", "Clippy"]
    );
    assert_eq!(
        report
            .steps
            .iter()
            .filter(|s| s.name.starts_with("Build "))
            .count(),
        2
    );
    assert_eq!(
        report
            .steps
            .iter()
            .filter(|s| s.name.ends_with(": interface"))
            .count(),
        2
    );
    assert!(report
        .steps
        .iter()
        .all(|s| s.status == VerificationStatus::Passed));
    for call in fake.calls.borrow().iter() {
        assert!(!call.args.iter().any(|a| a == "--all-features"));
        if call.args.first().is_some_and(|a| a == "contract") && call.args[1] == "build" {
            assert!(!PathBuf::from(&call.args[5]).exists());
        }
    }
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("not-for-display"));
    assert_eq!(
        serde_json::from_str::<VerificationReport>(&json).unwrap(),
        report
    );
}
#[test]
fn project_failure_is_normal_data_and_independent_steps_continue() {
    let project = Project::copy("healthy-v28");
    for failure in ["fmt", "test", "clippy", "build"] {
        let mut fake = Fake::new();
        fake.failure = Some(failure.into());
        let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
        assert_eq!(report.exit_code, ExitCode::PolicyFailed);
        assert!(report
            .steps
            .iter()
            .any(|s| s.status == VerificationStatus::Failed && s.finding.is_some()));
        assert_eq!(
            report
                .steps
                .iter()
                .filter(|s| s.name.starts_with("Build "))
                .count(),
            2
        );
        if failure == "build" {
            assert!(!fake
                .calls
                .borrow()
                .iter()
                .any(|c| c.args.get(1).is_some_and(|a| a == "info")));
            assert_eq!(
                report
                    .steps
                    .iter()
                    .filter(|s| s.name.starts_with("Inspect ")
                        && s.status == VerificationStatus::Skipped)
                    .count(),
                2
            );
        }
    }
}
#[test]
fn missing_tools_and_missing_artifacts_skip_dependent_inspection() {
    let project = Project::copy("healthy-v28");
    for missing in [true, false] {
        let mut fake = Fake::new();
        fake.missing = missing;
        fake.no_artifact = true;
        let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
        assert_eq!(
            report.exit_code,
            if missing {
                ExitCode::ExternalFailure
            } else {
                ExitCode::PolicyFailed
            }
        );
        assert!(!fake
            .calls
            .borrow()
            .iter()
            .any(|c| c.args.get(1).is_some_and(|a| a == "info")));
        assert!(report
            .steps
            .iter()
            .any(|s| s.name.starts_with("Inspect ") && s.status == VerificationStatus::Skipped));
    }
}
#[test]
fn large_output_and_failure_evidence_are_bounded_and_redacted() {
    let project = Project::copy("healthy-v28");
    let mut fake = Fake::new();
    fake.large = true;
    fake.failure = Some("test".into());
    let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
    for step in &report.steps {
        if let Some(c) = &step.command {
            assert!(c.stdout.len() <= OUTPUT_LIMIT);
            assert!(c.stderr.len() <= OUTPUT_LIMIT);
        }
    }
    assert!(report.steps.iter().any(|s| s.stdout_truncated));
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("do-not-show"));
    assert!(!json.contains("not-for-display"));
    assert!(json.contains("raw diagnostic retained"));
    let test = report.steps.iter().find(|s| s.name == "Tests").unwrap();
    assert_eq!(
        test.finding.as_ref().unwrap().id.as_str(),
        "VERIFY_TEST_FAILED"
    );
    assert_eq!(test.exit_code, Some(101));
}
#[test]
fn configured_disables_and_no_contract_project_are_explicit_skips() {
    let project = Project::copy("healthy-v28");
    fs::write(
        project.0.join("upgrade-doctor.toml"),
        "run_tests=false\nbuild_contracts=false\n",
    )
    .unwrap();
    let fake = Fake::new();
    let options = VerificationOptions {
        skip_clippy: true,
        ..Default::default()
    };
    let report = verify_with(&project.0, &options, &fake).unwrap();
    assert_eq!(report.exit_code, ExitCode::Completed);
    assert!(report
        .steps
        .iter()
        .filter(
            |s| ["Tests", "Clippy", "Contract build", "Wasm inspection"].contains(&s.name.as_str())
        )
        .all(|s| s.status == VerificationStatus::Skipped));
    assert!(!fake
        .calls
        .borrow()
        .iter()
        .any(|c| c.args[0] == "test" || c.args[0] == "clippy" || c.args[0] == "contract"));
}
#[test]
#[ignore = "Opt-in: real P28 fmt/tests/clippy/build/inspection, requires cached SDK, rustfmt, clippy, Stellar and Wasm target"]
fn real_p28_multi_contract_verification_is_green() {
    let project = Project::copy("healthy-v28");
    let report = doctor_cli::verify::verify(&project.0, &VerificationOptions::default()).unwrap();
    assert_eq!(report.exit_code, ExitCode::Completed, "{report:#?}");
    assert_eq!(
        report
            .steps
            .iter()
            .filter(|s| s.name.starts_with("Build ") && s.status == VerificationStatus::Passed)
            .count(),
        2
    );
    assert_eq!(
        report
            .steps
            .iter()
            .filter(|s| s.name.ends_with(": interface") && s.status == VerificationStatus::Passed)
            .count(),
        2
    );
}

#[test]
fn failed_first_contract_does_not_block_independent_second_contract() {
    let project = Project::copy("healthy-v28");
    let mut fake = Fake::new();
    fake.failure = Some("alpha".into());
    let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
    assert_eq!(report.exit_code, ExitCode::PolicyFailed);
    assert!(report
        .steps
        .iter()
        .any(|s| s.name == "Build alpha" && s.status == VerificationStatus::Failed));
    assert!(report
        .steps
        .iter()
        .any(|s| s.name == "Inspect alpha" && s.status == VerificationStatus::Skipped));
    assert!(report
        .steps
        .iter()
        .any(|s| s.name == "Inspect beta: interface" && s.status == VerificationStatus::Passed));
}

#[test]
fn invalid_configuration_stops_before_any_verification_command() {
    let project = Project::copy("healthy-v28");
    fs::write(
        project.0.join("upgrade-doctor.toml"),
        "run_tests=\"invalid\"\n",
    )
    .unwrap();
    let fake = Fake::new();
    assert!(verify_with(&project.0, &VerificationOptions::default(), &fake).is_err());
    assert!(fake.calls.borrow().iter().all(|c| c.args[0] == "metadata"));
}

#[test]
#[ignore = "Opt-in: real failing target build through sud doctor --verify; needs cached SDK and Stellar"]
fn real_failed_target_build_returns_structured_json_not_doctor_crash() {
    let project = Project::copy("verification-failing-build");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sud"))
        .arg("doctor")
        .arg(&project.0)
        .args(["--verify", "--json", "--skip-clippy"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report: VerificationReport = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report.steps.iter().any(|s| s.name.starts_with("Build ")
        && s.status == VerificationStatus::Failed
        && s.finding
            .as_ref()
            .is_some_and(|f| f.id.as_str() == "VERIFY_BUILD_FAILED")));
    assert!(report
        .steps
        .iter()
        .any(|s| s.name.starts_with("Inspect ") && s.status == VerificationStatus::Skipped));
    assert!(serde_json::to_string(&report)
        .unwrap()
        .contains("intentional verification build failure"));
}

#[test]
fn cli_missing_tool_is_a_report_and_discovery_without_verify_does_not_execute_checks() {
    let project = Project::copy("healthy-v28");
    let empty_path = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sud"))
        .arg("doctor")
        .arg(&project.0)
        .args(["--verify", "--json"])
        .env("PATH", empty_path.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: VerificationReport = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report.steps[0].status, VerificationStatus::MissingTool);
    assert!(report.steps[1..]
        .iter()
        .all(|s| s.status == VerificationStatus::Skipped));
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sud"))
        .arg("doctor")
        .arg(&project.0)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("Cargo discovery only"));
    assert!(!project.0.join("Cargo.lock").exists());
    assert!(!project.0.join("target").exists());
}

#[test]
fn truncation_boundary_does_not_expose_partial_secret() {
    let input = format!(
        "normal diagnostic\n{}API_KEY=hidden-value",
        "x".repeat(OUTPUT_LIMIT)
    );
    let (output, truncated) = doctor_cli::verify::bounded_text(&input);
    assert!(truncated);
    assert!(output.len() <= OUTPUT_LIMIT);
    assert!(!output.contains("hidden-value"));
    assert!(output.contains("normal diagnostic"));
    let secret = format!("S{}", "A".repeat(55));
    let input = format!(
        "normal diagnostic\n{}{}",
        "x".repeat(OUTPUT_LIMIT - 20),
        secret
    );
    let (output, _) = doctor_cli::verify::bounded_text(&input);
    assert!(!output.contains("SAAAA"));
}

#[test]
fn non_soroban_verification_runs_rust_checks_without_requiring_stellar() {
    let project = Project::copy("non-soroban");
    let fake = Fake::new();
    let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
    assert_eq!(report.exit_code, ExitCode::Completed);
    assert!(report
        .steps
        .iter()
        .any(|s| s.name == "Contract build" && s.status == VerificationStatus::Skipped));
    assert!(!fake.calls.borrow().iter().any(|c| c.program == "stellar"));
}

#[test]
fn timeout_and_runner_errors_retain_command_plan_and_normal_report() {
    let project = Project::copy("healthy-v28");
    for failure in ["timeout", "runner-error"] {
        let mut fake = Fake::new();
        fake.failure = Some(failure.into());
        let report = verify_with(&project.0, &VerificationOptions::default(), &fake).unwrap();
        assert_eq!(report.exit_code, ExitCode::ExternalFailure);
        let tests = report.steps.iter().find(|s| s.name == "Tests").unwrap();
        assert!(tests.command.is_some());
        assert!(tests.displayed_command.as_ref().unwrap().contains("test"));
        assert_eq!(
            tests.status,
            if failure == "timeout" {
                VerificationStatus::TimedOut
            } else {
                VerificationStatus::ExecutionError
            }
        );
        assert!(!serde_json::to_string(&report)
            .unwrap()
            .contains("not-for-display"));
    }
}
