#![allow(clippy::unwrap_used)]

use doctor_core::*;
use std::{collections::BTreeSet, path::PathBuf};

fn finding(severity: Severity) -> Finding {
    Finding {
        id: RuleId::new("PUBLIC_FUNCTION_REMOVED").unwrap(),
        title: "Public function removed".into(),
        severity,
        category: Category::Interface,
        summary: "An exported function was removed".into(),
        why_it_matters: "Existing callers may fail".into(),
        evidence: vec![Evidence {
            path: Some(PathBuf::from("contracts/counter.rs")),
            line: Some(12.try_into().unwrap()),
            message: "increment existed before the change".into(),
            command: None,
        }],
        recommendation: "Review callers before release".into(),
        references: vec!["fixture:interface-removal".into()],
    }
}

#[test]
fn analysis_round_trips_with_stable_wire_names_and_schema() {
    let result = AnalysisResult {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        tool_version: "0.1.0-alpha.1".into(),
        repository_path: PathBuf::from("contract project"),
        detected_packages: vec![DetectedPackage {
            name: "counter".into(),
            manifest_path: PathBuf::from("contracts/counter/Cargo.toml"),
            soroban_sdk_requirement: Some("28".into()),
        }],
        findings: vec![finding(Severity::Breaking)],
        command_results: vec![CommandResult {
            program: "cargo".into(),
            args: vec!["test".into()],
            working_directory: PathBuf::from("contract project"),
            status: CommandStatus::Exited { code: 1 },
            stdout: "test failed".into(),
            stderr: "actual diagnostic".into(),
            elapsed_millis: 10,
        }],
        verdict: Verdict::NotReady,
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["schema_version"], REPORT_SCHEMA_VERSION);
    assert_eq!(value["findings"][0]["severity"], "BREAKING");
    assert_eq!(value["findings"][0]["category"], "INTERFACE");
    assert_eq!(value["verdict"], "NOT_READY");
    assert_eq!(
        serde_json::from_value::<AnalysisResult>(value).unwrap(),
        result
    );
}

#[test]
fn evidence_can_be_absent_or_contain_command_diagnostics() {
    let mut value = finding(Severity::ManualReview);
    value.evidence[0] = Evidence {
        path: None,
        line: None,
        message: "command failure".into(),
        command: Some(CommandResult {
            program: "stellar".into(),
            args: vec!["--version".into()],
            working_directory: PathBuf::from("."),
            status: CommandStatus::FailedToStart {
                message: "not found".into(),
            },
            stdout: String::new(),
            stderr: String::new(),
            elapsed_millis: 0,
        }),
    };
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(serde_json::from_str::<Finding>(&json).unwrap(), value);
    assert!(serde_json::from_str::<Evidence>(
        r#"{"path":null,"line":0,"message":"bad line","command":null}"#
    )
    .is_err());
}

#[test]
fn severity_order_is_explicit_and_policy_is_not_a_threshold() {
    let ordered = BTreeSet::from([
        Severity::Breaking,
        Severity::Info,
        Severity::ManualReview,
        Severity::Warning,
    ]);
    assert_eq!(
        ordered.into_iter().collect::<Vec<_>>(),
        [
            Severity::Info,
            Severity::Warning,
            Severity::ManualReview,
            Severity::Breaking
        ]
    );
    let config = Config {
        fail_on: BTreeSet::from([Severity::Warning]),
        ..Config::default()
    };
    assert_eq!(
        evaluate_policy(&[finding(Severity::Breaking)], &config).exit_code,
        ExitCode::Completed
    );
    assert_eq!(
        evaluate_policy(&[finding(Severity::Warning)], &config).exit_code,
        ExitCode::PolicyFailed
    );
}

#[test]
fn defaults_match_the_build_plan() {
    let config = Config::default();
    assert_eq!(config.fail_on, BTreeSet::from([Severity::Breaking]));
    assert!(config.run_tests);
    assert!(config.build_contracts);
    assert_eq!(config.report_dir, PathBuf::from("doctor-reports"));
    assert_eq!(config.exclude, ["target", "vendor"]);
    assert_eq!(Config::from_toml("").unwrap(), config);
}

#[test]
fn explicit_and_partial_configuration_parse() {
    let config = Config::from_toml(
        r#"
fail_on = ["MANUAL_REVIEW", "BREAKING", "BREAKING"]
run_tests = false
build_contracts = false
report_dir = "reports with spaces"
exclude = ["target", "generated"]
"#,
    )
    .unwrap();
    assert_eq!(
        config.fail_on,
        BTreeSet::from([Severity::Breaking, Severity::ManualReview])
    );
    assert!(!config.run_tests);
    assert!(!config.build_contracts);
    assert_eq!(config.report_dir, PathBuf::from("reports with spaces"));
    assert_eq!(config.exclude, ["target", "generated"]);
    let partial = Config::from_toml("run_tests = false").unwrap();
    assert!(!partial.run_tests);
    assert!(partial.build_contracts);
    assert_eq!(partial.report_dir, Config::default().report_dir);
}

#[test]
fn malformed_unknown_and_invalid_configuration_is_rejected() {
    for source in [
        "fail_on = [\"CRITICAL\"]",
        "fail_on = [\"breaking\"]",
        "run_test = false",
        "run_tests = \"yes\"",
        "fail_on = \"BREAKING\"",
        "report_dir = \"\"",
        "exclude = [\"\"]",
        "fail_on = [",
    ] {
        assert!(Config::from_toml(source).is_err(), "{source}");
    }
    assert!(Config::from_toml("fail_on = [\"CRITICAL\"]")
        .unwrap_err()
        .to_string()
        .contains("CRITICAL"));
    assert!(serde_json::from_str::<Config>(r#"{"report_dir":""}"#).is_err());
    assert!(serde_json::from_str::<Severity>("\"CRITICAL\"").is_err());
}

#[test]
fn policy_verdicts_preserve_warnings_and_manual_review() {
    let config = Config::default();
    assert_eq!(
        evaluate_policy(&[], &config).verdict,
        Verdict::ReadyForManualReview
    );
    assert_eq!(
        evaluate_policy(&[finding(Severity::Info)], &config).verdict,
        Verdict::ReadyForManualReview
    );
    for severity in [Severity::Warning, Severity::ManualReview] {
        let decision = evaluate_policy(&[finding(severity)], &config);
        assert_eq!(decision.verdict, Verdict::ChecksCompletedWithWarnings);
        assert_eq!(decision.exit_code, ExitCode::Completed);
    }
    let decision = evaluate_policy(&[finding(Severity::Breaking)], &config);
    assert_eq!(decision.verdict, Verdict::NotReady);
    assert_eq!(decision.exit_code, ExitCode::PolicyFailed);
    let config = Config {
        fail_on: BTreeSet::new(),
        ..config
    };
    let decision = evaluate_policy(&[finding(Severity::Breaking)], &config);
    assert_eq!(decision.exit_code, ExitCode::Completed);
    assert_eq!(decision.verdict, Verdict::ChecksCompletedWithWarnings);
}

#[test]
fn rule_ids_are_preserved_and_validated_on_deserialization() {
    let id = RuleId::new("SDK28_REMOVED_EXPORT_ARGUMENT").unwrap();
    assert_eq!(id.as_str(), "SDK28_REMOVED_EXPORT_ARGUMENT");
    assert_eq!(
        serde_json::to_string(&id).unwrap(),
        "\"SDK28_REMOVED_EXPORT_ARGUMENT\""
    );
    assert_eq!(
        serde_json::from_str::<RuleId>("\"SDK28_REMOVED_EXPORT_ARGUMENT\"").unwrap(),
        id
    );
    for id in [
        "",
        "sdk28_export",
        "1_RULE",
        "HAS SPACE",
        "_RULE",
        "RULE_",
        "RULE__ID",
        "RÜLE",
    ] {
        assert!(RuleId::new(id).is_err(), "{id}");
        assert!(serde_json::from_value::<RuleId>(serde_json::json!(id)).is_err());
    }
}

#[test]
fn every_category_and_command_status_round_trips() {
    for category in [
        Category::Toolchain,
        Category::Dependency,
        Category::Source,
        Category::Storage,
        Category::Interface,
        Category::Event,
        Category::Auth,
        Category::Build,
        Category::Test,
        Category::Wasm,
        Category::Metadata,
    ] {
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(serde_json::from_str::<Category>(&json).unwrap(), category);
    }
    for status in [
        CommandStatus::Exited { code: 0 },
        CommandStatus::Exited { code: 7 },
        CommandStatus::TimedOut,
        CommandStatus::Terminated { signal: Some(15) },
        CommandStatus::ExecutionFailed {
            message: "pipe read failure".into(),
        },
        CommandStatus::FailedToStart {
            message: "missing binary".into(),
        },
    ] {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            serde_json::from_str::<CommandStatus>(&json).unwrap(),
            status
        );
    }
}

#[test]
fn exit_codes_and_failure_precedence_match_the_plan() {
    let config = Config::default();
    let findings = [finding(Severity::Breaking)];
    for (outcome, expected) in [
        (RunOutcome::Completed, ExitCode::PolicyFailed),
        (RunOutcome::InvalidInput, ExitCode::InvalidInput),
        (RunOutcome::ExternalFailure, ExitCode::ExternalFailure),
        (RunOutcome::InternalFailure, ExitCode::InternalError),
    ] {
        assert_eq!(exit_code(outcome, &findings, &config), expected);
    }
    assert_eq!(
        exit_code(RunOutcome::Completed, &[], &config),
        ExitCode::Completed
    );
    for (code, number) in [
        (ExitCode::Completed, 0),
        (ExitCode::PolicyFailed, 1),
        (ExitCode::InvalidInput, 2),
        (ExitCode::ExternalFailure, 3),
        (ExitCode::InternalError, 4),
    ] {
        assert_eq!(code.as_u8(), number);
    }
}

#[test]
fn configuration_json_round_trip_is_validated_and_deterministic() {
    let config =
        Config::from_toml("fail_on = [\"BREAKING\", \"INFO\", \"MANUAL_REVIEW\", \"WARNING\"]")
            .unwrap();
    let json = serde_json::to_string(&config).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        value["fail_on"],
        serde_json::json!(["INFO", "WARNING", "MANUAL_REVIEW", "BREAKING"])
    );
    assert_eq!(serde_json::from_str::<Config>(&json).unwrap(), config);
    assert_eq!(serde_json::to_string(&config).unwrap(), json);
}

#[test]
fn every_severity_can_be_configured_as_blocking_independently() {
    let severities = [
        Severity::Info,
        Severity::Warning,
        Severity::ManualReview,
        Severity::Breaking,
    ];
    for selected in severities {
        let config = Config {
            fail_on: BTreeSet::from([selected]),
            ..Config::default()
        };
        for actual in severities {
            let decision = evaluate_policy(&[finding(actual)], &config);
            assert_eq!(
                decision.exit_code == ExitCode::PolicyFailed,
                actual == selected
            );
            if actual == selected {
                assert_eq!(decision.verdict, Verdict::NotReady);
            }
        }
    }
}
