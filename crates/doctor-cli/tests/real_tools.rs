#![allow(clippy::unwrap_used)]
mod support;
use doctor_cli::check::{check, StepStatus};
use doctor_core::{CommandStatus, ExitCode};
use support::Project;
#[test]
#[ignore = "Opt-in: runs real Cargo tests and Stellar CLI builds; needs SDK downloads and Wasm target"]
fn real_healthy_v28_tests_and_stellar_build_succeed() {
    let project = Project::copy("healthy-v28");
    let result = check(&project.0).unwrap();
    assert_eq!(
        result.exit_code,
        ExitCode::Completed,
        "{:#?}",
        result.analysis
    );
    for step in [&result.tests, &result.build] {
        match step {
            StepStatus::Executed { command_index } => assert_eq!(
                result.analysis.command_results[*command_index].status,
                CommandStatus::Exited { code: 0 }
            ),
            other => panic!("real command was not run: {other:?}"),
        }
    }
}
#[test]
#[ignore = "Opt-in: runs real failing Cargo tests followed by a real Stellar CLI build"]
fn real_failing_test_fixture_records_failure_then_builds() {
    let project = Project::copy("failing-tests");
    let result = check(&project.0).unwrap();
    assert_eq!(
        result.exit_code,
        ExitCode::PolicyFailed,
        "{:#?}",
        result.analysis
    );
    let finding = result
        .analysis
        .findings
        .iter()
        .find(|finding| finding.id.as_str() == "TEST_FAILED")
        .unwrap();
    let command = finding.evidence[0].command.as_ref().unwrap();
    assert!(command.stdout.contains("intentional failing-tests fixture"));
    assert_eq!(command.status, CommandStatus::Exited { code: 101 });
    match result.build {
        StepStatus::Executed { command_index } => assert_eq!(
            result.analysis.command_results[command_index].status,
            CommandStatus::Exited { code: 0 }
        ),
        other => panic!("build was not run: {other:?}"),
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sortralis"))
        .arg("check")
        .arg(&project.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("TEST_FAILED"));
    assert!(stdout.contains("intentional failing-tests fixture"));
    assert!(stdout.contains("Build: passed (recorded exit 0)"));
    assert!(stderr.contains("error: test failed"));
}
