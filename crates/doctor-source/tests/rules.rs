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


#[test]
fn sdk28_deploy_rule_flags_deploy_v2_but_not_supported_deployer_helpers() {
    let path = Path::new("deploy.rs");
    let legacy = r#"
        fn f(env: soroban_sdk::Env, hash: soroban_sdk::BytesN<32>) {
            env.deployer().with_current_contract([0u8; 32]).deploy_v2(hash, ());
        }
    "#;
    let findings = analyze_text(path, legacy, TargetContext::Sdk28).unwrap();
    let deploy = findings
        .iter()
        .find(|finding| finding.id.as_str() == "P28-DEPLOY-001")
        .unwrap();
    assert_eq!(deploy.severity, Severity::Breaking);

    let modern = r#"
        use soroban_sdk::{ContractExecutable, Env};
        fn f(env: Env, hash: soroban_sdk::BytesN<32>) {
            let deployer = env.deployer().with_current_contract([0u8; 32]);
            let _predicted = deployer.deployed_address();
            deployer.deploy_contract(ContractExecutable::Wasm(hash), ());
            let _ = env.deployer().with_address(env.current_contract_address(), [1u8; 32]);
            let _ = env.deployer().upload_contract_wasm(&[]);
        }
    "#;
    let findings = analyze_text(path, modern, TargetContext::Sdk28).unwrap();
    assert!(
        findings
            .iter()
            .all(|finding| finding.id.as_str() != "P28-DEPLOY-001"),
        "{findings:?}"
    );
}

#[test]
fn verified_stellar_macros_auth_attributes_suppress_upgrade_review() {
    let path = Path::new("upgrade.rs");

    for source in [
        r#"
            use stellar_macros::only_role;
            impl C {
                #[only_role(operator, "manager")]
                fn upgrade(e: &soroban_sdk::Env, operator: soroban_sdk::Address) {}
            }
        "#,
        r#"
            impl C {
                #[stellar_macros::only_owner]
                fn upgrade(e: &soroban_sdk::Env) {}
            }
        "#,
        r#"
            use stellar_macros::{only_admin as admin_guard, only_any_role};
            impl C {
                #[admin_guard]
                fn upgrade(e: &soroban_sdk::Env) {}
                #[only_any_role(operator, ["manager", "admin"])]
                fn upgrade_contract(e: &soroban_sdk::Env, operator: soroban_sdk::Address) {}
            }
        "#,
    ] {
        let findings = analyze_text(path, source, TargetContext::Sdk28).unwrap();
        assert!(
            findings
                .iter()
                .all(|finding| finding.id.as_str() != "P28-AUTH-001"),
            "{source}\n{findings:?}"
        );
    }
}

#[test]
fn lookalike_or_non_authorizing_macros_do_not_suppress_upgrade_review() {
    let path = Path::new("upgrade.rs");

    for source in [
        r#"
            #[only_role(operator, "manager")]
            fn upgrade(e: &soroban_sdk::Env, operator: soroban_sdk::Address) {}
        "#,
        r#"
            use stellar_macros::has_role;
            #[has_role(operator, "manager")]
            fn upgrade(e: &soroban_sdk::Env, operator: soroban_sdk::Address) {}
        "#,
        r#"
            use other::only_owner;
            #[only_owner]
            fn upgrade(e: &soroban_sdk::Env) {}
        "#,
    ] {
        let findings = analyze_text(path, source, TargetContext::Sdk28).unwrap();
        assert!(
            findings
                .iter()
                .any(|finding| finding.id.as_str() == "P28-AUTH-001"),
            "{source}\n{findings:?}"
        );
    }
}
