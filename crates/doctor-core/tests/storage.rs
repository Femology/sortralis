#![allow(clippy::unwrap_used)]
use doctor_core::storage::*;

fn inventory(durability: Durability, value: &str) -> StorageInventory {
    StorageInventory {
        entries: vec![StorageEntry {
            key: StorageKey {
                identity: "symbol:COUNT".into(),
                expression: "COUNT".into(),
                kind: KeyKind::Symbol,
                dynamic: false,
                contract_type: None,
            },
            durability,
            value_types: vec![value.into()],
            operations: vec!["set".into()],
            evidence: vec![],
        }],
        ..StorageInventory::default()
    }
}

#[test]
fn storage_round_trip_and_explicit_classification() {
    let before = inventory(Durability::Persistent, "u32");
    let encoded = serde_json::to_string(&before).unwrap();
    assert_eq!(
        serde_json::from_str::<StorageInventory>(&encoded).unwrap(),
        before
    );
    assert!(compare_storage(&before, &before).records.is_empty());
    let diff = compare_storage(&before, &inventory(Durability::Temporary, "u64"));
    assert_eq!(diff.records.len(), 2);
    assert!(diff
        .records
        .iter()
        .all(|r| r.classification == StorageClassification::MigrationLikely));
    assert_eq!(
        diff.records[0].classification.to_string(),
        "MIGRATION_LIKELY"
    );
    assert_eq!(
        serde_json::from_str::<StorageDiff>(&serde_json::to_string(&diff).unwrap()).unwrap(),
        diff
    );
    let mut old = before.clone();
    let mut has = old.entries[0].clone();
    has.value_types.clear();
    has.operations = vec!["has".into()];
    old.entries.push(has.clone());
    let mut new = inventory(Durability::Persistent, "u64");
    new.entries.push(has);
    assert!(compare_storage(&old, &new)
        .records
        .iter()
        .any(|r| r.id == "STORAGE_VALUE_TYPE_CHANGED"
            && r.classification == StorageClassification::MigrationLikely));
}

#[test]
fn additions_removals_and_unknowns_are_conservative() {
    let empty = StorageInventory::default();
    let known = inventory(Durability::Instance, "u32");
    assert_eq!(
        compare_storage(&empty, &known).records[0].classification,
        StorageClassification::Info
    );
    assert_eq!(
        compare_storage(&known, &empty).records[0].classification,
        StorageClassification::ReviewRequired
    );
    let mut unknown = known.clone();
    unknown.entries[0].key.dynamic = true;
    assert_eq!(
        compare_storage(&unknown, &unknown).records[0].classification,
        StorageClassification::ReviewRequired
    );
    unknown.entries[0].key.dynamic = false;
    unknown.entries[0].value_types.clear();
    assert_eq!(
        compare_storage(&known, &unknown).records[0].classification,
        StorageClassification::ReviewRequired
    );
}

#[test]
fn added_unknown_values_and_read_only_uses_need_review() {
    let empty = StorageInventory::default();
    let known = inventory(Durability::Persistent, "u32");
    let mut unknown = known.clone();
    unknown.entries[0].value_types.clear();
    assert_eq!(
        compare_storage(&empty, &unknown).records[0].classification,
        StorageClassification::ReviewRequired
    );
    let mut read = known.clone();
    read.entries[0].operations = vec!["get".into()];
    assert_eq!(
        compare_storage(&empty, &read).records[0].classification,
        StorageClassification::ReviewRequired
    );
    assert_eq!(
        compare_storage(&empty, &known).records[0].classification,
        StorageClassification::Info
    );
}
