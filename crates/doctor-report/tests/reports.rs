#![allow(clippy::unwrap_used)]

use doctor_core::report::{Report, ReportFormat, ReportParams, ReportStep, ToolInfo};
use doctor_core::{
    Category, CommandResult, CommandStatus, DetectedPackage, Evidence, Finding, RuleId, Severity,
    Verdict, REPORT_SCHEMA_VERSION,
};
use doctor_report::{
    html_escape, write_html_report, write_json_report, write_report, write_sarif_report,
    write_terminal_report, SarifLog,
};
use std::num::NonZeroU32;
use std::path::PathBuf;

fn sample_finding(rule: &str, severity: Severity) -> Finding {
    Finding {
        id: RuleId::new(rule).unwrap(),
        title: format!("Title for {rule}"),
        severity,
        category: Category::Source,
        summary: format!("Summary for {rule}"),
        why_it_matters: format!("Why {rule} matters to the upgrade."),
        evidence: vec![Evidence {
            path: Some(PathBuf::from("contracts/hello/src/lib.rs")),
            line: NonZeroU32::new(42),
            message: "Deprecated invocation detected".into(),
            command: None,
        }],
        recommendation: format!("Remediation guide for {rule}."),
        references: vec!["https://soroban.stellar.org/docs".into()],
    }
}

fn sample_report() -> Report {
    let findings = vec![
        sample_finding("SOROBAN_STORAGE_KEY_CHANGED", Severity::Breaking),
        sample_finding("SOROBAN_DEPRECATED_MACRO", Severity::Warning),
        sample_finding("SOROBAN_MANUAL_REVIEW_AUTH", Severity::ManualReview),
    ];

    let steps = vec![
        ReportStep {
            name: "Tests".into(),
            status: "passed (recorded exit 0)".into(),
            description: None,
            exit_code: Some(0),
            duration_millis: Some(150),
            command: Some("cargo test".into()),
        },
        ReportStep {
            name: "Build".into(),
            status: "passed (recorded exit 0)".into(),
            description: None,
            exit_code: Some(0),
            duration_millis: Some(300),
            command: Some("stellar contract build".into()),
        },
    ];

    let packages = vec![DetectedPackage {
        name: "my-contract".into(),
        manifest_path: PathBuf::from("Cargo.toml"),
        soroban_sdk_requirement: Some("28.0.0".into()),
    }];

    let tools = vec![
        ToolInfo {
            name: "rustc".into(),
            version: Some("1.96.0".into()),
            status: "detected".into(),
        },
        ToolInfo {
            name: "stellar".into(),
            version: Some("27.0.0".into()),
            status: "detected".into(),
        },
    ];

    let commands = vec![CommandResult {
        program: "cargo".into(),
        args: vec!["metadata".into()],
        working_directory: PathBuf::from("/workspace/sample"),
        status: CommandStatus::Exited { code: 0 },
        stdout: "{}".into(),
        stderr: "".into(),
        elapsed_millis: 45,
    }];

    Report::new(ReportParams {
        repository_path: PathBuf::from("/workspace/sample"),
        verdict: Verdict::NotReady,
        exit_code: 1,
        packages,
        tools,
        steps,
        findings,
        command_results: commands,
    })
}

#[test]
fn test_terminal_report_output() {
    let report = sample_report();
    let mut buf = Vec::new();
    write_terminal_report(&mut buf, &report, false).unwrap();
    let text = String::from_utf8(buf).unwrap();

    // Concise summary and counts
    assert!(text.contains("Workspace: /workspace/sample"));
    assert!(text.contains("Summary: 1 breaking, 1 warning, 1 manual review, 0 info; 2 steps (2 passed, 0 failed, 0 other)"));

    // Project and tools versions
    assert!(text.contains("Packages: my-contract (SDK: 28.0.0)"));
    assert!(text.contains("Tools: rustc 1.96.0, stellar 27.0.0"));

    // Verification steps
    assert!(text.contains("Tests: passed (recorded exit 0) (150 ms)"));
    assert!(text.contains("Build: passed (recorded exit 0) (300 ms)"));

    // Findings with location, evidence, remediation
    assert!(text.contains("[BREAKING] SOROBAN_STORAGE_KEY_CHANGED"));
    assert!(text.contains("Location: contracts/hello/src/lib.rs:42"));
    assert!(text.contains("Evidence: Deprecated invocation detected"));
    assert!(text.contains("Why it matters: Why SOROBAN_STORAGE_KEY_CHANGED matters"));
    assert!(text.contains("Remediation: Remediation guide for SOROBAN_STORAGE_KEY_CHANGED"));
    assert!(text.contains("Reference: https://soroban.stellar.org/docs"));

    // Verdict and disclaimer
    assert!(text.contains("Verdict: NotReady; exit 1"));
    assert!(text.contains("Passing Sortralis is not a security audit"));
}

#[test]
fn test_json_report_schema_and_round_trip() {
    let report = sample_report();
    let mut buf = Vec::new();
    write_json_report(&mut buf, &report).unwrap();
    let json_str = String::from_utf8(buf).unwrap();

    assert!(json_str.contains("\"schema_version\": \"1.0\""));
    assert!(json_str.contains("\"tool_name\": \"sortralis\""));
    assert!(json_str.contains("\"verdict\": \"NOT_READY\""));
    assert!(json_str.contains("\"SOROBAN_STORAGE_KEY_CHANGED\""));

    // Round-trip deserialization
    let deserialized: Report = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.schema_version, REPORT_SCHEMA_VERSION);
    assert_eq!(deserialized.summary.breaking_count, 1);
    assert_eq!(deserialized.findings.len(), 3);
    assert_eq!(deserialized, report);
}

#[test]
fn test_sarif_validation_and_parsing() {
    let report = sample_report();
    let mut buf = Vec::new();
    write_sarif_report(&mut buf, &report).unwrap();
    let sarif_str = String::from_utf8(buf).unwrap();

    // Parse as SARIF Log
    let sarif: SarifLog = serde_json::from_str(&sarif_str).unwrap();
    assert_eq!(sarif.version, "2.1.0");
    assert!(sarif.schema.contains("sarif-schema-2.1.0.json"));
    assert_eq!(sarif.runs.len(), 1);

    let run = &sarif.runs[0];
    assert_eq!(run.tool.driver.name, "sortralis");
    assert!(!run.tool.driver.version.is_empty());
    assert_eq!(
        run.tool.driver.information_uri,
        "https://github.com/Femology/sortralis"
    );

    // Validate rules metadata
    assert_eq!(run.tool.driver.rules.len(), 3);
    let rule_breaking = run
        .tool
        .driver
        .rules
        .iter()
        .find(|r| r.id == "SOROBAN_STORAGE_KEY_CHANGED")
        .unwrap();
    assert_eq!(rule_breaking.default_configuration.level, "error");
    assert!(rule_breaking
        .help_uri
        .contains("SOROBAN_STORAGE_KEY_CHANGED"));
    assert!(!rule_breaking.short_description.text.is_empty());
    assert!(!rule_breaking.help.text.is_empty());

    // Validate results mapping
    assert_eq!(run.results.len(), 3);
    let res_breaking = run
        .results
        .iter()
        .find(|r| r.rule_id == "SOROBAN_STORAGE_KEY_CHANGED")
        .unwrap();
    assert_eq!(res_breaking.level, "error");
    assert_eq!(res_breaking.locations.len(), 1);
    let loc = &res_breaking.locations[0];
    assert_eq!(
        loc.physical_location.artifact_location.uri,
        "contracts/hello/src/lib.rs"
    );
    assert_eq!(
        loc.physical_location.region.as_ref().unwrap().start_line,
        42
    );

    let res_warning = run
        .results
        .iter()
        .find(|r| r.rule_id == "SOROBAN_DEPRECATED_MACRO")
        .unwrap();
    assert_eq!(res_warning.level, "warning");
}

#[test]
fn test_html_escaping_security_gate() {
    // Malicious project and finding names intended for HTML / XSS injection
    let malicious_path = PathBuf::from("/workspace/<script>alert('pwned_path')</script>/repo");
    let malicious_finding = Finding {
        id: RuleId::new("SOROBAN_DEPRECATED_MACRO").unwrap(),
        title: "Title with <img src=x onerror=alert('xss_title')>".into(),
        severity: Severity::Breaking,
        category: Category::Source,
        summary:
            "Summary with <b>bold</b> & 'single' \"double\" <script>alert('xss_summary')</script>"
                .into(),
        why_it_matters: "Why it matters with <svg onload=alert('xss_why')>".into(),
        evidence: vec![Evidence {
            path: Some(PathBuf::from("src/<malicious_file>.rs")),
            line: NonZeroU32::new(10),
            message: "Evidence with <iframe src=\"javascript:alert(1)\">".into(),
            command: None,
        }],
        recommendation: "Remediation with <a href=\"javascript:alert(2)\">click me</a>".into(),
        references: vec!["https://example.com/?q=<script>alert(3)</script>".into()],
    };

    let malicious_package = DetectedPackage {
        name: "<evil-pkg>".into(),
        manifest_path: PathBuf::from("Cargo.toml"),
        soroban_sdk_requirement: Some("<28.0.0>".into()),
    };

    let malicious_step = ReportStep {
        name: "Test <step>".into(),
        status: "failed <error>".into(),
        description: Some("Desc <script>".into()),
        exit_code: Some(1),
        duration_millis: Some(10),
        command: Some("cmd <evil>".into()),
    };

    let report = Report::new(ReportParams {
        repository_path: malicious_path,
        verdict: Verdict::NotReady,
        exit_code: 1,
        packages: vec![malicious_package],
        tools: vec![],
        steps: vec![malicious_step],
        findings: vec![malicious_finding],
        command_results: vec![],
    });

    let mut buf = Vec::new();
    write_html_report(&mut buf, &report).unwrap();
    let html = String::from_utf8(buf).unwrap();

    // Security Gate: Ensure NO raw unescaped HTML tags from user inputs exist
    assert!(
        !html.contains("<script>alert("),
        "raw script tag must not exist"
    );
    assert!(!html.contains("<img src=x"), "raw img tag must not exist");
    assert!(!html.contains("<svg onload="), "raw svg tag must not exist");
    assert!(
        !html.contains("<iframe src="),
        "raw iframe tag must not exist"
    );
    assert!(!html.contains("<evil-pkg>"), "raw evil-pkg must not exist");
    assert!(
        !html.contains("<malicious_file>"),
        "raw malicious file name must not exist"
    );

    // Ensure properly escaped entities appear
    assert!(html.contains("&lt;script&gt;alert("));
    assert!(html.contains("&lt;img src=x"));
    assert!(html.contains("&lt;svg onload="));
    assert!(html.contains("&lt;iframe src="));
    assert!(html.contains("&lt;evil-pkg&gt;"));
    assert!(html.contains("&lt;malicious_file&gt;"));
    assert!(html.contains("&amp;"));
    assert!(html.contains("&#39;single&#39;"));
    assert!(html.contains("&quot;double&quot;"));

    // Ensure self-contained offline requirements
    assert!(!html.contains("http://"), "no remote http assets allowed");
    assert!(!html.contains("https://cdn"), "no external cdn allowed");
    assert!(!html.contains("google-analytics"), "no tracking allowed");
}

#[test]
fn test_all_formats_contain_same_known_rule_id() {
    let report = sample_report();
    let known_rule = "SOROBAN_STORAGE_KEY_CHANGED";

    // 1. Terminal
    let mut term_buf = Vec::new();
    write_report(&mut term_buf, &report, ReportFormat::Terminal, false).unwrap();
    let term_out = String::from_utf8(term_buf).unwrap();
    assert!(
        term_out.contains(known_rule),
        "terminal output must contain known rule ID"
    );

    // 2. JSON
    let mut json_buf = Vec::new();
    write_report(&mut json_buf, &report, ReportFormat::Json, false).unwrap();
    let json_out = String::from_utf8(json_buf).unwrap();
    assert!(
        json_out.contains(known_rule),
        "json output must contain known rule ID"
    );

    // 3. SARIF
    let mut sarif_buf = Vec::new();
    write_report(&mut sarif_buf, &report, ReportFormat::Sarif, false).unwrap();
    let sarif_out = String::from_utf8(sarif_buf).unwrap();
    assert!(
        sarif_out.contains(known_rule),
        "sarif output must contain known rule ID"
    );

    // 4. HTML
    let mut html_buf = Vec::new();
    write_report(&mut html_buf, &report, ReportFormat::Html, false).unwrap();
    let html_out = String::from_utf8(html_buf).unwrap();
    assert!(
        html_out.contains(known_rule),
        "html output must contain known rule ID"
    );
}

#[test]
fn test_html_escape_unit_function() {
    assert_eq!(html_escape("a & b"), "a &amp; b");
    assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    assert_eq!(html_escape("\"quote\""), "&quot;quote&quot;");
    assert_eq!(html_escape("'single'"), "&#39;single&#39;");
    assert_eq!(html_escape("normal text 123"), "normal text 123");
}
