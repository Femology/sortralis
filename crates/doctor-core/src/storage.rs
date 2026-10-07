//! Normalized source observations, never an exact on-ledger schema.
use crate::Evidence;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

pub const STORAGE_SCHEMA_VERSION: &str = "1.0";
pub const STORAGE_SCOPE: &str = "Source observations only: no macro expansion, cfg evaluation, full Rust name/type resolution, ledger state, or serialized XDR analysis. Unchanged source types do not prove that no migration is required.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Durability {
    Instance,
    Persistent,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeyKind {
    EnumVariant,
    Symbol,
    String,
    Primitive,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageKey {
    /// Source identity, not an encoded ledger key.
    pub identity: String,
    pub expression: String,
    pub kind: KeyKind,
    pub dynamic: bool,
    pub contract_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEntry {
    pub key: StorageKey,
    pub durability: Durability,
    /// Explicit or locally inferred Rust spellings; an empty list means unknown.
    pub value_types: Vec<String>,
    pub operations: Vec<String>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageField {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageVariant {
    pub name: String,
    pub fields: Vec<StorageField>,
    pub shape: String,
    pub discriminant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContractTypeShape {
    Struct {
        shape: String,
        fields: Vec<StorageField>,
    },
    Enum {
        variants: Vec<StorageVariant>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageContractType {
    pub identity: String,
    pub shape: ContractTypeShape,
    pub used_as_key: bool,
    pub used_as_value: bool,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageInventory {
    pub schema_version: String,
    pub scope: String,
    pub entries: Vec<StorageEntry>,
    /// Includes unobserved contract types, clearly distinguished by usage flags.
    pub contract_types: Vec<StorageContractType>,
    pub uncertainties: Vec<Evidence>,
}
impl Default for StorageInventory {
    fn default() -> Self {
        Self {
            schema_version: STORAGE_SCHEMA_VERSION.into(),
            scope: STORAGE_SCOPE.into(),
            entries: vec![],
            contract_types: vec![],
            uncertainties: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageClassification {
    Info,
    ReviewRequired,
    MigrationLikely,
}
impl fmt::Display for StorageClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "INFO",
            Self::ReviewRequired => "REVIEW_REQUIRED",
            Self::MigrationLikely => "MIGRATION_LIKELY",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDiffRecord {
    pub id: String,
    pub subject: String,
    pub classification: StorageClassification,
    pub summary: String,
    pub before: Vec<Evidence>,
    pub after: Vec<Evidence>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageDiff {
    pub schema_version: String,
    pub scope: String,
    /// Explicit normalized snapshots allow machine consumers to inspect old/new
    /// shapes and values by each record's subject without parsing summary text.
    pub before_inventory: StorageInventory,
    pub after_inventory: StorageInventory,
    pub records: Vec<StorageDiffRecord>,
}

fn type_change_summary(
    before: Option<&StorageContractType>,
    after: Option<&StorageContractType>,
) -> String {
    match (before.map(|t| &t.shape), after.map(|t| &t.shape)) {
        (
            Some(ContractTypeShape::Struct {
                shape: old_shape,
                fields: old,
            }),
            Some(ContractTypeShape::Struct {
                shape: new_shape,
                fields: new,
            }),
        ) if old_shape == new_shape => {
            let names: BTreeSet<_> = old.iter().chain(new).map(|f| f.name.as_str()).collect();
            let mut changes = Vec::new();
            for name in names {
                let old = old.iter().find(|f| f.name == name);
                let new = new.iter().find(|f| f.name == name);
                match (old, new) {
                    (Some(o), Some(n)) if o.type_name != n.type_name => {
                        changes.push(format!("field {name}: {} -> {}", o.type_name, n.type_name))
                    }
                    (Some(o), None) => {
                        changes.push(format!("field {name} removed ({})", o.type_name))
                    }
                    (None, Some(n)) => {
                        changes.push(format!("field {name} added ({})", n.type_name))
                    }
                    _ => {}
                }
            }
            if changes.is_empty() {
                "Storage-associated field ordering changed; review representation.".into()
            } else {
                format!("Storage-associated {old_shape} struct changed: {}. Review existing values and write-back behavior.", changes.join("; "))
            }
        }
        (
            Some(ContractTypeShape::Enum { variants: old }),
            Some(ContractTypeShape::Enum { variants: new }),
        ) => {
            let names: BTreeSet<_> = old.iter().chain(new).map(|v| v.name.as_str()).collect();
            let changes: Vec<_> = names
                .into_iter()
                .filter_map(|name| {
                    let o = old.iter().find(|v| v.name == name);
                    let n = new.iter().find(|v| v.name == name);
                    if o == n {
                        None
                    } else {
                        Some(format!(
                            "variant {name} {}",
                            if o.is_none() {
                                "added"
                            } else if n.is_none() {
                                "removed"
                            } else {
                                "shape/discriminant changed"
                            }
                        ))
                    }
                })
                .collect();
            format!(
                "Storage-associated enum changed: {}. Review encoded keys/values.",
                if changes.is_empty() {
                    "variant order changed".into()
                } else {
                    changes.join("; ")
                }
            )
        }
        (None, Some(_)) => {
            "Storage-associated contract type added; review key/value encoding and initialization."
                .into()
        }
        (Some(_), None) => {
            "Storage-associated contract type removed; review existing ledger entries and readers."
                .into()
        }
        _ => "Storage-associated contract type representation changed; review encoded keys/values."
            .into(),
    }
}

fn record(
    id: &str,
    subject: &str,
    classification: StorageClassification,
    summary: String,
    before: Vec<Evidence>,
    after: Vec<Evidence>,
) -> StorageDiffRecord {
    StorageDiffRecord {
        id: id.into(),
        subject: subject.into(),
        classification,
        summary,
        before,
        after,
    }
}
fn entry_evidence(entries: &[&StorageEntry]) -> Vec<Evidence> {
    entries.iter().flat_map(|e| e.evidence.clone()).collect()
}
fn values<'a>(entries: &[&'a StorageEntry]) -> BTreeSet<&'a str> {
    entries
        .iter()
        .flat_map(|e| e.value_types.iter().map(String::as_str))
        .collect()
}
fn unknown_value(entry: &StorageEntry) -> bool {
    entry.value_types.is_empty()
        && entry
            .operations
            .iter()
            .any(|op| matches!(op.as_str(), "get" | "set" | "update" | "try_update"))
}

/// Conservative comparison of explicit before/after source inventories.
/// Dynamic observations always require review, even if their spelling is identical.
pub fn compare_storage(before: &StorageInventory, after: &StorageInventory) -> StorageDiff {
    use StorageClassification::{Info, MigrationLikely, ReviewRequired};
    let mut records = Vec::new();
    let keys: BTreeSet<_> = before
        .entries
        .iter()
        .chain(&after.entries)
        .map(|e| e.key.identity.as_str())
        .collect();
    for key in keys {
        let old: Vec<_> = before
            .entries
            .iter()
            .filter(|e| e.key.identity == key)
            .collect();
        let new: Vec<_> = after
            .entries
            .iter()
            .filter(|e| e.key.identity == key)
            .collect();
        let b = entry_evidence(&old);
        let a = entry_evidence(&new);
        if old.iter().chain(&new).any(|e| e.key.dynamic) {
            records.push(record("STORAGE_DYNAMIC_KEY_REVIEW", key, ReviewRequired,
                "Dynamic/unknown key observations cannot establish key identity or independence; review runtime keys.".into(), b.clone(), a.clone()));
        }
        if old.is_empty() {
            records.push(record("STORAGE_KEY_ADDED", key,
                if new.iter().any(|e| e.key.dynamic || unknown_value(e))
                    || !new.iter().any(|e| e.operations.iter().any(|op| matches!(op.as_str(), "set" | "update" | "try_update")))
                { ReviewRequired } else { Info },
                "Key use added in source; usually non-breaking if independent. Confirm runtime independence and initialization.".into(), b, a));
        } else if new.is_empty() {
            records.push(record(
                "STORAGE_KEY_REMOVED",
                key,
                ReviewRequired,
                "Key use removed from source; existing ledger entries and readers require review."
                    .into(),
                b,
                a,
            ));
        } else {
            let old_d: BTreeSet<_> = old.iter().map(|e| e.durability).collect();
            let new_d: BTreeSet<_> = new.iter().map(|e| e.durability).collect();
            if old_d != new_d {
                records.push(record("STORAGE_DURABILITY_CHANGED", key, MigrationLikely,
                    format!("Observed durability changed from {old_d:?} to {new_d:?}; migration is likely."), b.clone(), a.clone()));
            }
            if old_d.is_disjoint(&new_d) && values(&old) != values(&new) {
                let unknown = values(&old).is_empty() || values(&new).is_empty();
                records.push(record(
                    if unknown {
                        "STORAGE_VALUE_UNKNOWN"
                    } else {
                        "STORAGE_VALUE_TYPE_CHANGED"
                    },
                    key,
                    if unknown {
                        ReviewRequired
                    } else {
                        MigrationLikely
                    },
                    format!(
                        "Observed value types changed across durability classes: {:?} -> {:?}.",
                        values(&old),
                        values(&new)
                    ),
                    b.clone(),
                    a.clone(),
                ));
            }
            // Compare per durability as well, so swapping value types between
            // two durability classes is not hidden by a union of types.
            for durability in old_d.intersection(&new_d) {
                let o: Vec<_> = old
                    .iter()
                    .copied()
                    .filter(|e| e.durability == *durability)
                    .collect();
                let n: Vec<_> = new
                    .iter()
                    .copied()
                    .filter(|e| e.durability == *durability)
                    .collect();
                let ov = values(&o);
                let nv = values(&n);
                if ov != nv || o.iter().chain(&n).any(|e| unknown_value(e)) {
                    let unknown = ov.is_empty()
                        || nv.is_empty()
                        || o.iter().chain(&n).any(|e| unknown_value(e));
                    records.push(record(if unknown { "STORAGE_VALUE_UNKNOWN" } else { "STORAGE_VALUE_TYPE_CHANGED" }, key,
                        if unknown { ReviewRequired } else { MigrationLikely },
                        format!("{durability:?} observed value types: {ov:?} -> {nv:?}; review encoding and existing values."),
                        entry_evidence(&o), entry_evidence(&n)));
                }
            }
        }
    }
    let names: BTreeSet<_> = before
        .contract_types
        .iter()
        .chain(&after.contract_types)
        .filter(|t| t.used_as_key || t.used_as_value)
        .map(|t| t.identity.as_str())
        .collect();
    for name in names {
        let old = before.contract_types.iter().find(|t| t.identity == name);
        let new = after.contract_types.iter().find(|t| t.identity == name);
        if old.map(|t| &t.shape) != new.map(|t| &t.shape) {
            let key_only = old.into_iter().chain(new).all(|t| !t.used_as_value);
            records.push(record(
                if key_only {
                    "STORAGE_KEY_TYPE_CHANGED"
                } else {
                    "STORAGE_CONTRACT_TYPE_CHANGED"
                },
                name,
                if key_only {
                    ReviewRequired
                } else {
                    MigrationLikely
                },
                type_change_summary(old, new),
                old.map(|t| vec![t.evidence.clone()]).unwrap_or_default(),
                new.map(|t| vec![t.evidence.clone()]).unwrap_or_default(),
            ));
        }
    }
    if !before.uncertainties.is_empty() || !after.uncertainties.is_empty() {
        records.push(record(
            "STORAGE_ANALYSIS_INCOMPLETE",
            "source coverage",
            ReviewRequired,
            "Unresolved storage-like expressions or macro-generated source require manual review."
                .into(),
            before.uncertainties.clone(),
            after.uncertainties.clone(),
        ));
    }
    records.sort_by(|a, b| (&a.subject, &a.id, &a.summary).cmp(&(&b.subject, &b.id, &b.summary)));
    StorageDiff {
        schema_version: STORAGE_SCHEMA_VERSION.into(),
        scope: STORAGE_SCOPE.into(),
        before_inventory: before.clone(),
        after_inventory: after.clone(),
        records,
    }
}
