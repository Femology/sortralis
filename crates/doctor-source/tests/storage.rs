#![allow(clippy::unwrap_used)]
use doctor_core::storage::*;
use doctor_source::{inventory_storage_text, SourceOptions};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> StorageInventory {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/storage")
        .join(name)
        .join("lib.rs");
    inventory_storage_text(
        Path::new("lib.rs"),
        &std::fs::read_to_string(file).unwrap(),
        &SourceOptions::default(),
    )
    .unwrap()
}
#[test]
fn phase_gate_fixture_classifications() {
    let before = fixture("identical");
    assert_eq!(before.entries.len(), 1);
    assert_eq!(before.entries[0].durability, Durability::Persistent);
    assert_eq!(before.entries[0].key.kind, KeyKind::EnumVariant);
    assert_eq!(before.entries[0].value_types, ["State"]);
    assert_eq!(
        before.entries[0].evidence[0].path.as_deref(),
        Some(Path::new("lib.rs"))
    );
    assert_eq!(before.entries[0].evidence[0].line.unwrap().get(), 9);
    assert!(compare_storage(&before, &fixture("identical"))
        .records
        .is_empty());
    for (name, id, classification) in [
        (
            "added-independent-key",
            "STORAGE_KEY_ADDED",
            StorageClassification::Info,
        ),
        (
            "changed-field-type",
            "STORAGE_CONTRACT_TYPE_CHANGED",
            StorageClassification::MigrationLikely,
        ),
        (
            "removed-field",
            "STORAGE_CONTRACT_TYPE_CHANGED",
            StorageClassification::MigrationLikely,
        ),
        (
            "durability-change",
            "STORAGE_DURABILITY_CHANGED",
            StorageClassification::MigrationLikely,
        ),
        (
            "dynamic-key",
            "STORAGE_DYNAMIC_KEY_REVIEW",
            StorageClassification::ReviewRequired,
        ),
    ] {
        let diff = compare_storage(&before, &fixture(name));
        assert!(
            diff.records
                .iter()
                .any(|r| r.id == id && r.classification == classification),
            "{name}: {diff:?}"
        );
        assert!(
            diff.records
                .iter()
                .all(|r| r.classification == classification),
            "{name}: unexpected classification: {diff:?}"
        );
    }
    assert!(compare_storage(&before, &fixture("unrelated-contracttype"))
        .records
        .is_empty());
    let unrelated = before
        .contract_types
        .iter()
        .find(|t| t.identity.ends_with("Unrelated"))
        .unwrap();
    assert!(!unrelated.used_as_key && !unrelated.used_as_value);
}

#[test]
fn source_keys_aliases_and_all_durabilities() {
    let text = r#"
use soroban_sdk::{Env as E, contracttype, symbol_short};
#[contracttype] enum DataKey { Balance(u32) }
fn f(e: &E, amount: u64, id: u32) {
    let storage = e.storage();
    let p = storage.persistent();
    let key = DataKey::Balance(id);
    p.set(&key, &amount);
    e.storage().instance().set(&symbol_short!("ADMIN"), &true);
    e.storage().temporary().get::<_, u32>(&7u32);
    e.storage().temporary().set(&"name", &3i64);
}"#;
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap();
    assert_eq!(inv.entries.len(), 4, "{inv:?}");
    assert!(inv
        .entries
        .iter()
        .any(|e| e.key.kind == KeyKind::EnumVariant && e.key.dynamic && e.value_types == ["u64"]));
    assert!(inv
        .entries
        .iter()
        .any(|e| e.key.kind == KeyKind::Symbol && e.value_types == ["bool"]));
    assert!(inv
        .entries
        .iter()
        .any(|e| e.key.kind == KeyKind::Primitive && e.value_types == ["u32"]));
    assert!(inv
        .entries
        .iter()
        .any(|e| e.key.kind == KeyKind::String && e.value_types == ["i64"]));
}

#[test]
fn no_regex_false_positives_and_typed_parse_errors() {
    let text = r#"
struct Env;
fn f(env: Env) { env.storage().persistent().set(&1, &2); }
// env.storage().temporary().set(&"fake", &3);
const TEXT: &str = "env.storage().instance().set(&1, &2)";
"#;
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap();
    assert!(inv.entries.is_empty());
    assert!(!inv.uncertainties.is_empty());
    assert!(
        inventory_storage_text(Path::new("bad.rs"), "fn {", &SourceOptions::default()).is_err()
    );
}

#[test]
fn constants_typed_dynamic_keys_and_nested_storage_types() {
    let text = r#"
use soroban_sdk::{Env, contracttype, symbol_short};
const ADMIN: soroban_sdk::Symbol = symbol_short!("ADMIN");
#[contracttype] struct Inner { count: u32 }
#[contracttype] struct Outer { inner: Inner }
#[contracttype] enum DataKey { Owner }
fn save(env: Env, value: Outer, key: DataKey) {
    env.storage().persistent().set(&ADMIN, &value);
    env.storage().instance().set(&key, &value);
}"#;
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap();
    assert!(inv
        .entries
        .iter()
        .any(|e| e.key.kind == KeyKind::Symbol && !e.key.dynamic));
    assert!(
        inv.contract_types
            .iter()
            .filter(|t| t.used_as_value)
            .count()
            == 2
    );
    assert!(inv
        .contract_types
        .iter()
        .any(|t| t.identity.ends_with("DataKey") && t.used_as_key));
}

#[test]
fn local_shadowing_and_closure_parameters_do_not_reuse_outer_env() {
    let text = r#"
use soroban_sdk::Env;
fn f(env: Env) {
    { let env = fake(); env.storage().persistent().set(&1u32, &true); }
    let closure = |env| env.storage().instance().set(&2u32, &true);
    env.storage().temporary().set(&3u32, &true);
}"#;
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap();
    assert_eq!(inv.entries.len(), 1);
    assert_eq!(inv.entries[0].durability, Durability::Temporary);
    assert!(!inv.uncertainties.is_empty());
}

#[test]
fn directory_exclusions_and_relative_identities() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/storage");
    let options = SourceOptions {
        exclude: vec![PathBuf::from("dynamic-key")],
        ..SourceOptions::default()
    };
    let inv = doctor_source::inventory_storage_directory(&root, &options).unwrap();
    assert!(inv.entries.iter().all(|e| e.evidence.iter().all(|e| e
        .path
        .as_ref()
        .is_some_and(|p| p.is_relative() && !p.starts_with("dynamic-key")))));
    assert!(doctor_source::inventory_storage_directory(
        &root,
        &SourceOptions {
            exclude: vec!["../escape".into()],
            ..options
        }
    )
    .is_err());
}

#[test]
fn variant_structure_value_changes_and_new_fields_are_classified() {
    let before = r#"
use soroban_sdk::{Env, contracttype};
#[contracttype] enum DataKey { User(u32) }
#[contracttype] struct State { n: u32 }
fn f(e: Env, s: State) { e.storage().persistent().set(&DataKey::User(1u32), &s); }
"#;
    let inventory = |text: &str| {
        inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap()
    };
    let old = inventory(before);
    for (text, id, classification) in [
        (
            before.replace("User(u32)", "User(u64)"),
            "STORAGE_KEY_TYPE_CHANGED",
            StorageClassification::ReviewRequired,
        ),
        (
            before.replace("n: u32", "n: u32, added: Option<u64>"),
            "STORAGE_CONTRACT_TYPE_CHANGED",
            StorageClassification::MigrationLikely,
        ),
        (
            before.replace("&s);", "&true);"),
            "STORAGE_VALUE_TYPE_CHANGED",
            StorageClassification::MigrationLikely,
        ),
        (
            before.replace("&s);", "&compute());"),
            "STORAGE_VALUE_UNKNOWN",
            StorageClassification::ReviewRequired,
        ),
        (
            before.replace(
                "e.storage().persistent().set(&DataKey::User(1u32), &s);",
                "",
            ),
            "STORAGE_KEY_REMOVED",
            StorageClassification::ReviewRequired,
        ),
    ] {
        let diff = compare_storage(&old, &inventory(&text));
        let record = diff.records.iter().find(|r| r.id == id).unwrap();
        assert_eq!(record.classification, classification, "{diff:?}");
        assert!(!record.before.is_empty());
        if id != "STORAGE_KEY_REMOVED" {
            assert!(!record.after.is_empty());
        }
        assert!(!compare_storage(&old, &old)
            .records
            .iter()
            .any(|r| r.id == id));
    }
}

#[test]
fn sdk_namespace_aliases_nested_modules_and_ttl_methods() {
    let text = r#"
use sdk as stellar;
use sdk::Env;
fn f(e: Env) {
    e.storage().instance().extend_ttl(10, 100);
    e.storage().persistent().extend_ttl(&7u32, 10, 100);
    e.storage().temporary().remove(&9u32);
    e.storage().instance().has(&stellar::symbol_short!("ADMIN"));
}

mod nested {
    use sdk::{Env, contracttype};
    #[contracttype] struct State { n: u32 }
    fn save(e: Env, value: State) { e.storage().instance().set(&true, &value); }
}"#;
    let options = SourceOptions {
        sdk_crate_names: vec!["sdk".into()],
        ..SourceOptions::default()
    };
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &options).unwrap();
    assert_eq!(inv.entries.len(), 4);
    assert!(inv
        .contract_types
        .iter()
        .any(|t| t.identity == "lib.rs::nested::State" && t.used_as_value));
    assert_eq!(
        inv.entries
            .iter()
            .filter(|e| e.operations.contains(&"extend_ttl".into()))
            .count(),
        1
    );
    assert!(compare_storage(&inv, &inv).records.is_empty());
}

#[test]
fn loop_and_match_patterns_shadow_environment_names() {
    let text = r#"
use soroban_sdk::Env;
fn f(e: Env) {
    for e in values() { e.storage().persistent().set(&1u32, &true); }
    match value() { e => e.storage().instance().set(&2u32, &true) }
    e.storage().temporary().set(&3u32, &true);
}"#;
    let inv = inventory_storage_text(Path::new("lib.rs"), text, &SourceOptions::default()).unwrap();
    assert_eq!(inv.entries.len(), 1);
    assert_eq!(inv.entries[0].durability, Durability::Temporary);
    assert!(!inv.uncertainties.is_empty());
}
