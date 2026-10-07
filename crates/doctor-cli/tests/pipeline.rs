#![allow(clippy::unwrap_used)]
use doctor_cli::check::{check_with, CheckTimeouts, StepStatus};
use doctor_core::{CommandResult, CommandStatus, ExitCode};
use doctor_runner::{CapturedCommand, CommandRunner, CommandSpec, RunnerError, SystemRunner};
use std::{cell::RefCell, path::PathBuf};
struct Double {
    calls: RefCell<Vec<CommandSpec>>,
    test_code: i32,
    build_code: i32,
    stellar: &'static str,
    task_status: Option<CommandStatus>,
}
impl Double {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            test_code: 0,
            build_code: 0,
            stellar: "stellar 27.0.0",
            task_status: None,
        }
    }
}
impl CommandRunner for Double {
    fn execute(&self, request: &CommandSpec) -> Result<CapturedCommand, RunnerError> {
        self.calls.borrow_mut().push(request.clone());
        if request.args.first().is_some_and(|arg| arg == "metadata") {
            return SystemRunner.execute(request);
        }
        let (code, stdout, stderr) = if request.args.first().is_some_and(|arg| arg == "--version") {
            (
                0,
                match request.program.to_string_lossy().as_ref() {
                    "stellar" => self.stellar.to_string(),
                    program => format!("{program} 1.96.0"),
                },
                String::new(),
            )
        } else if request.program == "cargo" {
            (
                self.test_code,
                "unit double test output".into(),
                "raw double test stderr".into(),
            )
        } else {
            (
                self.build_code,
                "unit double build output".into(),
                "raw double build stderr".into(),
            )
        };
        Ok(CapturedCommand {
            request: request.clone(),
            record: CommandResult {
                program: request.program.to_string_lossy().into(),
                args: request
                    .args
                    .iter()
                    .map(|arg| arg.to_string_lossy().into())
                    .collect(),
                working_directory: request.working_directory.clone(),
                status: if request.program == "stellar"
                    && request.args[0] == "--version"
                    && self.stellar.is_empty()
                {
                    CommandStatus::FailedToStart {
                        message: "unit double missing binary".into(),
                    }
                } else if request.args[0] != "--version" {
                    self.task_status
                        .clone()
                        .unwrap_or(CommandStatus::Exited { code })
                } else {
                    CommandStatus::Exited { code }
                },
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                elapsed_millis: 1,
            },
            stdout_bytes: stdout.into_bytes(),
            stderr_bytes: stderr.into_bytes(),
            spawn_error_kind: if request.program == "stellar"
                && request.args[0] == "--version"
                && self.stellar.is_empty()
            {
                Some(std::io::ErrorKind::NotFound)
            } else {
                None
            },
        })
    }
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}
#[test]
fn pipeline_records_explicit_success_and_uses_workspace_root_in_order() {
    let double = Double::new();
    let root = fixture("healthy-v28");
    let result = check_with(&root, &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.exit_code, ExitCode::Completed);
    assert!(matches!(result.tests, StepStatus::Executed { .. }));
    assert!(matches!(result.build, StepStatus::Executed { .. }));
    let calls = double.calls.borrow();
    assert_eq!(calls.len(), 6);
    assert_eq!(calls[4].program, "cargo");
    assert_eq!(calls[4].args, vec![std::ffi::OsString::from("test")]);
    assert_eq!(calls[5].program, "stellar");
    assert_eq!(
        calls[5].args,
        vec![std::ffi::OsString::from("contract"), "build".into()]
    );
    assert_eq!(calls[4].working_directory, root);
    assert_eq!(calls[5].working_directory, root);
    assert_eq!(
        result.analysis.command_results[4].status,
        CommandStatus::Exited { code: 0 }
    );
    assert_eq!(
        result.analysis.command_results[5].status,
        CommandStatus::Exited { code: 0 }
    );
}
#[test]
fn test_failure_preserves_stderr_and_still_runs_build() {
    let mut double = Double::new();
    double.test_code = 101;
    let result = check_with(&fixture("healthy-v28"), &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.exit_code, ExitCode::PolicyFailed);
    let finding = result
        .analysis
        .findings
        .iter()
        .find(|finding| finding.id.as_str() == "TEST_FAILED")
        .unwrap();
    assert_eq!(
        finding.evidence[0].command.as_ref().unwrap().stderr,
        "raw double test stderr"
    );
    assert!(matches!(result.build, StepStatus::Executed { .. }));
}

mod support;
use support::Project;
#[test]
fn static_findings_survive_test_and_build_failure_and_group_packages() {
    let project = Project::copy("healthy-v28");
    for package in ["alpha", "beta"] {
        let path = project.0.join(package).join("src/lib.rs");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str(
            "\n#[sdk::contracttype(export = false)] pub struct Legacy { pub value: u32 }\n",
        );
        std::fs::write(path, text).unwrap();
    }
    let mut double = Double::new();
    double.test_code = 101;
    double.build_code = 1;
    let result = check_with(&project.0, &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.exit_code, ExitCode::PolicyFailed);
    for id in [
        "SDK28_REMOVED_EXPORT_ARGUMENT",
        "TEST_FAILED",
        "BUILD_FAILED",
    ] {
        assert!(result
            .analysis
            .findings
            .iter()
            .any(|finding| finding.id.as_str() == id));
    }
    let source = result
        .analysis
        .findings
        .iter()
        .find(|finding| finding.id.as_str() == "SDK28_REMOVED_EXPORT_ARGUMENT")
        .unwrap();
    assert_eq!(source.evidence.len(), 2);
    let build = result
        .analysis
        .findings
        .iter()
        .find(|finding| finding.id.as_str() == "BUILD_FAILED")
        .unwrap();
    assert_eq!(
        build.evidence[0].command.as_ref().unwrap().stderr,
        "raw double build stderr"
    );
}
#[test]
fn disabled_checks_do_not_execute_and_workspace_root_config_wins_for_members() {
    let project = Project::copy("healthy-v28");
    std::fs::write(
        project.0.join("upgrade-doctor.toml"),
        "run_tests = false\nbuild_contracts = false\n",
    )
    .unwrap();
    let mut double = Double::new();
    double.stellar = "";
    let result = check_with(&project.0.join("alpha"), &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.tests, StepStatus::Disabled);
    assert_eq!(result.build, StepStatus::Disabled);
    assert_eq!(result.exit_code, ExitCode::Completed);
    assert_eq!(double.calls.borrow().len(), 4);
}
#[test]
fn invalid_configuration_is_rejected_before_external_commands() {
    let project = Project::copy("healthy-v28");
    for config in [
        "unknown = true",
        "exclude = [\"../outside\"]",
        "fail_on = [\"invalid\"]",
    ] {
        std::fs::write(project.0.join("upgrade-doctor.toml"), config).unwrap();
        let double = Double::new();
        let error = check_with(&project.0, &CheckTimeouts::default(), &double).unwrap_err();
        assert_eq!(error.exit_code(), ExitCode::InvalidInput);
        assert!(double.calls.borrow().is_empty());
    }
}
#[test]
fn missing_or_incompatible_stellar_is_external_failure_and_build_is_not_claimed() {
    for version in ["", "stellar 25.1.0", "stellar UNKNOWN"] {
        let mut double = Double::new();
        double.stellar = version;
        let result =
            check_with(&fixture("healthy-v28"), &CheckTimeouts::default(), &double).unwrap();
        assert_eq!(result.exit_code, ExitCode::ExternalFailure);
        assert!(matches!(result.build, StepStatus::Blocked { .. }));
        assert_eq!(double.calls.borrow().len(), 5);
        assert_eq!(result.analysis.command_results.len(), 5);
        if version == "stellar 25.1.0" {
            let finding = result
                .analysis
                .findings
                .iter()
                .find(|finding| finding.id.as_str() == "SDK28_STELLAR_BUILD_REQUIREMENT")
                .unwrap();
            assert_eq!(
                finding.references,
                vec![doctor_cli::check::SDK28_BUILD_REFERENCE]
            );
            assert_eq!(finding.severity, doctor_core::Severity::Breaking);
        }
    }
}
#[test]
fn timeout_is_external_failure_with_failure_evidence() {
    let mut double = Double::new();
    double.task_status = Some(CommandStatus::TimedOut);
    let result = check_with(&fixture("healthy-v28"), &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.exit_code, ExitCode::ExternalFailure);
    for id in ["TEST_FAILED", "BUILD_FAILED"] {
        let finding = result
            .analysis
            .findings
            .iter()
            .find(|finding| finding.id.as_str() == id)
            .unwrap();
        assert_eq!(
            finding.evidence[0].command.as_ref().unwrap().status,
            CommandStatus::TimedOut
        );
    }
}
#[test]
fn policy_configuration_is_applied_after_failed_commands() {
    let project = Project::copy("healthy-v28");
    std::fs::write(project.0.join("upgrade-doctor.toml"), "fail_on = []").unwrap();
    let mut double = Double::new();
    double.test_code = 101;
    let result = check_with(&project.0, &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.exit_code, ExitCode::Completed);
    assert_eq!(
        result.analysis.verdict,
        doctor_core::Verdict::ChecksCompletedWithWarnings
    );
    assert_eq!(
        result.analysis.command_results[4].status,
        CommandStatus::Exited { code: 101 }
    );
}
#[test]
fn no_soroban_packages_have_explicit_not_applicable_steps() {
    let double = Double::new();
    let result = check_with(&fixture("non-soroban"), &CheckTimeouts::default(), &double).unwrap();
    assert_eq!(result.tests, StepStatus::NotApplicable);
    assert_eq!(result.build, StepStatus::NotApplicable);
    assert_eq!(double.calls.borrow().len(), 4);
}
#[test]
fn incomplete_source_analysis_cannot_report_a_completed_check() {
    let project = Project::copy("healthy-v28");
    std::fs::write(project.0.join("alpha/src/lib.rs"), "fn broken( {").unwrap();
    let result = check_with(&project.0, &CheckTimeouts::default(), &Double::new()).unwrap();
    assert_eq!(result.exit_code, ExitCode::ExternalFailure);
    assert!(result
        .analysis
        .findings
        .iter()
        .any(|finding| finding.id.as_str() == "SOURCE_ANALYSIS_FAILED"));
    assert!(matches!(result.tests, StepStatus::Executed { .. }));
}
