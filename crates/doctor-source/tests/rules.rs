#![allow(clippy::unwrap_used)]
use doctor_core::Severity;
use doctor_source::{analyze_text, TargetContext};
use std::path::Path;
#[test]
fn export_argument_has_stable_id_breaking_severity_and_exact_evidence() {
    let source = include_str!("../../../fixtures/source/legacy-export-arg/lib.rs");
    let findings = analyze_text(Path::new("legacy/lib.rs"), source, TargetContext::Sdk28).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].id.as_str(), "SDK28_REMOVED_EXPORT_ARGUMENT");
    assert_eq!(findings[0].severity, Severity::Breaking);
    assert_eq!(findings[0].evidence.len(), 2);
    assert_eq!(
        findings[0].evidence[0].path.as_deref(),
        Some(Path::new("legacy/lib.rs"))
    );
    assert_eq!(findings[0].evidence[0].line.unwrap().get(), 3);
    assert_eq!(findings[0].evidence[1].line.unwrap().get(), 6);
}
#[test]
fn healthy_v28_and_ordinary_contract_have_no_automatic_findings() {
    for source in [
        include_str!("../../../fixtures/source/healthy-v28/lib.rs"),
        include_str!("../../../fixtures/source/ordinary-contract/lib.rs"),
    ] {
        assert!(
            analyze_text(Path::new("lib.rs"), source, TargetContext::Sdk28)
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn custom_account_is_manual_review_with_a_real_method_location() {
    let findings = analyze_text(
        Path::new("account/lib.rs"),
        include_str!("../../../fixtures/source/custom-account/lib.rs"),
        TargetContext::Sdk28,
    )
    .unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].id.as_str(), "CUSTOM_ACCOUNT_EXECUTABLE_REVIEW");
    assert_eq!(findings[0].severity, Severity::ManualReview);
    assert_eq!(findings[0].evidence[0].line.unwrap().get(), 8);
    assert_eq!(
        findings[0].evidence[0].path.as_deref(),
        Some(Path::new("account/lib.rs"))
    );
}

#[test]
fn comments_strings_unrelated_macros_and_non_export_arguments_are_not_findings() {
    for source in [
        "// #[soroban_sdk::contracttype(export = false)]\nconst S: &str = \"__check_auth contracterror(export = true)\";",
        "#[other::contracttype(export = true)] struct S;",
        "#[contracttype(export = true)] struct S;",
        "use soroban_sdk::*; use other::contracttype; #[contracttype(export = true)] struct S;",
        "use soroban_sdk::*; use other::*; #[contracttype(export = true)] struct S;",
        "use soroban_sdk::contracttype; macro_rules! contracttype { () => {} } #[contracttype(export = true)] struct S;",
        "use soroban_sdk::contracttype; #[contracttype(lib = \"export\")] struct S;",
        "fn __check_auth() {} trait T {fn __check_auth();} impl S {fn f() { self.__check_auth(); }}",
        "macro_rules! make { () => { #[soroban_sdk::contracttype(export = false)] struct S; } }",
    ] {assert!(analyze_text(Path::new("lib.rs"), source, TargetContext::Sdk28).unwrap().is_empty(), "{source}");}
}
#[test]
fn sdk_context_is_explicit_not_inferred_from_a_matching_identifier() {
    let source = include_str!("../../../fixtures/source/legacy-export-arg/lib.rs");
    for context in [TargetContext::OtherSdk, TargetContext::Unknown] {
        assert!(analyze_text(Path::new("lib.rs"), source, context)
            .unwrap()
            .is_empty());
    }
}
#[test]
fn qualified_aliased_multiline_attributes_and_module_scope_are_supported() {
    let source="use soroban_sdk::contracttype as ct;\n#[ct(\n export = false\n)] struct S;\nmod nested { #[soroban_sdk::contracterror(export = true)] enum E { A = 1 } }\nmod unrelated { #[ct(export = true)] struct U; }";
    let findings = analyze_text(Path::new("lib.rs"), source, TargetContext::Sdk28).unwrap();
    assert_eq!(findings[0].evidence.len(), 2);
    assert_eq!(findings[0].evidence[0].line.unwrap().get(), 3);
    assert_eq!(findings[0].evidence[1].line.unwrap().get(), 5);
}
#[test]
fn rules_are_independently_testable_and_manual_registration_is_explicit() {
    use doctor_source::{
        registry, CustomAccountRule, ExportArgumentRule, ParsedSource, SourceOptions, SourceRule,
    };
    let path = Path::new("lib.rs");
    let parsed = ParsedSource::parse(
        path,
        "#[soroban_sdk::contracttype(export = false)] struct S;",
        &SourceOptions::default().sdk_crate_names,
    )
    .unwrap();
    assert_eq!(
        ExportArgumentRule.documentation().id,
        "SDK28_REMOVED_EXPORT_ARGUMENT"
    );
    assert_eq!(
        ExportArgumentRule
            .analyze(&parsed, TargetContext::Sdk28)
            .unwrap()
            .len(),
        1
    );
    assert!(CustomAccountRule
        .analyze(&parsed, TargetContext::Sdk28)
        .unwrap()
        .is_empty());
    let account = ParsedSource::parse(
        path,
        "impl Account { fn __check_auth() {} }",
        &SourceOptions::default().sdk_crate_names,
    )
    .unwrap();
    assert_eq!(
        CustomAccountRule.documentation().id,
        "CUSTOM_ACCOUNT_EXECUTABLE_REVIEW"
    );
    assert_eq!(
        CustomAccountRule
            .analyze(&account, TargetContext::Sdk28)
            .unwrap()
            .len(),
        1
    );
    assert!(CustomAccountRule
        .analyze(&account, TargetContext::OtherSdk)
        .unwrap()
        .is_empty());
    let event = registry()
        .iter()
        .find(|rule| rule.documentation.id == "SDK28_EVENT_SHAPE_REVIEW")
        .unwrap();
    assert!(event.analyzer.is_none());
    assert!(event.documentation.description.contains("Currently manual"));
    for source in [
        "#[soroban_sdk::contractevent] struct E { value: Option<u32> }",
        "#[soroban_sdk::contractevent(sparse = false)] struct E { value: () }",
    ] {
        assert!(analyze_text(path, source, TargetContext::Sdk28)
            .unwrap()
            .is_empty());
    }
}
#[test]
fn invalid_rust_is_a_typed_error_with_path_and_line() {
    let error = analyze_text(
        Path::new("broken.rs"),
        "\nfn broken( {",
        TargetContext::Sdk28,
    )
    .unwrap_err();
    assert!(matches!(error, doctor_source::SourceError::Parse { .. }));
    assert!(error.to_string().contains("broken.rs:2"));
}

#[test]
fn sdk_extern_crate_alias_is_resolved_without_treating_other_crates_as_sdk() {
    let path = Path::new("lib.rs");
    let positive = "extern crate soroban_sdk as sdk; use sdk::contracttype; #[contracttype(export = false)] struct S;";
    assert_eq!(
        analyze_text(path, positive, TargetContext::Sdk28)
            .unwrap()
            .len(),
        1
    );
    let negative = "extern crate other as sdk; use sdk::contracttype; #[contracttype(export = false)] struct S;";
    assert!(analyze_text(path, negative, TargetContext::Sdk28)
        .unwrap()
        .is_empty());
}
