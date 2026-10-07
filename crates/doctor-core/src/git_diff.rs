//! Deterministic, source-only Git snapshot comparison models and policy.
use crate::{
    source_inventory::*,
    storage::{compare_storage, StorageDiff, StorageInventory},
    DetectedPackage, Finding,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};
pub const GIT_DIFF_SCOPE: &str = "Committed source snapshots only; dirty active files are excluded. Requested/lockfile SDK versions are not proof of compiled resolution. Interface/events are syntax inventories, not a Wasm spec. No macro expansion, cfg evaluation, target scripts, tests, builds or ledger inspection. Snapshot .cargo and rust-toolchain configuration is omitted. Unchanged source does not prove that no migration is required. This is not a security audit or a deployment-safety guarantee.";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSnapshot {
    pub package: DetectedPackage,
    pub sdk_requirements: Vec<String>,
    pub locked_sdk_versions: Vec<String>,
    pub rule_context: String,
    pub source: SourceInventory,
    pub storage: StorageInventory,
    pub findings: Vec<Finding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefAnalysis {
    pub commit: String,
    pub cargo_packages: Vec<DetectedPackage>,
    pub contracts: Vec<PackageSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemChange<T> {
    pub subject: String,
    pub before: Vec<T>,
    pub after: Vec<T>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkChange {
    pub package: PathBuf,
    pub before_requested: Vec<String>,
    pub after_requested: Vec<String>,
    pub before_locked: Vec<String>,
    pub after_locked: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStorageDiff {
    pub package: PathBuf,
    pub diff: StorageDiff,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitDiff {
    pub schema_version: String,
    pub scope: String,
    pub from_ref: String,
    pub to_ref: String,
    pub dirty_active_worktree: bool,
    pub before: RefAnalysis,
    pub after: RefAnalysis,
    pub sdk_changes: Vec<SdkChange>,
    pub functions: Vec<ItemChange<SourceFunction>>,
    pub types: Vec<ItemChange<SourceType>>,
    pub events: Vec<ItemChange<SourceEvent>>,
    pub storage: Vec<PackageStorageDiff>,
    pub findings: Vec<ItemChange<Finding>>,
}
fn items<T: Clone>(
    subject: &str,
    before: &[T],
    after: &[T],
    identity: impl Fn(&T) -> String,
    equal: impl Fn(&T, &T) -> bool,
) -> Vec<ItemChange<T>> {
    let names: BTreeSet<_> = before.iter().chain(after).map(&identity).collect();
    let mut result = Vec::new();
    for name in names {
        let b: Vec<_> = before
            .iter()
            .filter(|t| identity(t) == name)
            .cloned()
            .collect();
        let a: Vec<_> = after
            .iter()
            .filter(|t| identity(t) == name)
            .cloned()
            .collect();
        if b.len() != a.len() || !b.iter().zip(&a).all(|(b, a)| equal(b, a)) {
            result.push(ItemChange {
                subject: format!("{subject}::{name}"),
                before: b,
                after: a,
            });
        }
    }
    result
}
pub fn compare_refs(
    from_ref: &str,
    to_ref: &str,
    dirty: bool,
    before: RefAnalysis,
    after: RefAnalysis,
) -> GitDiff {
    let mut diff = GitDiff {
        schema_version: "1.0".into(),
        scope: GIT_DIFF_SCOPE.into(),
        from_ref: from_ref.into(),
        to_ref: to_ref.into(),
        dirty_active_worktree: dirty,
        before,
        after,
        sdk_changes: vec![],
        functions: vec![],
        types: vec![],
        events: vec![],
        storage: vec![],
        findings: vec![],
    };
    let paths: BTreeSet<_> = diff
        .before
        .contracts
        .iter()
        .chain(&diff.after.contracts)
        .map(|p| p.package.manifest_path.clone())
        .collect();
    for path in paths {
        let b = diff
            .before
            .contracts
            .iter()
            .find(|p| p.package.manifest_path == path);
        let a = diff
            .after
            .contracts
            .iter()
            .find(|p| p.package.manifest_path == path);
        let requested =
            |p: Option<&PackageSnapshot>| p.map(|p| p.sdk_requirements.clone()).unwrap_or_default();
        let locked = |p: Option<&PackageSnapshot>| {
            p.map(|p| p.locked_sdk_versions.clone()).unwrap_or_default()
        };
        if requested(b) != requested(a) || locked(b) != locked(a) {
            diff.sdk_changes.push(SdkChange {
                package: path.clone(),
                before_requested: requested(b),
                after_requested: requested(a),
                before_locked: locked(b),
                after_locked: locked(a),
            });
        }
        let source_before = b.map(|p| p.source.clone()).unwrap_or_default();
        let source_after = a.map(|p| p.source.clone()).unwrap_or_default();
        let subject = path.to_string_lossy();
        diff.functions.extend(items(
            &subject,
            &source_before.functions,
            &source_after.functions,
            |f| f.identity.clone(),
            |b, a| b.signature == a.signature && b.owner == a.owner && b.attributes == a.attributes,
        ));
        diff.types.extend(items(
            &subject,
            &source_before.types,
            &source_after.types,
            |t| t.identity.clone(),
            |b, a| b.shape == a.shape && b.attributes == a.attributes,
        ));
        diff.events.extend(items(
            &subject,
            &source_before.events,
            &source_after.events,
            |e| e.identity.clone(),
            |b, a| b.fields == a.fields && b.shape == a.shape && b.attributes == a.attributes,
        ));
        let storage_before = b.map(|p| p.storage.clone()).unwrap_or_default();
        let storage_after = a.map(|p| p.storage.clone()).unwrap_or_default();
        let storage = compare_storage(&storage_before, &storage_after);
        if !storage.records.is_empty() {
            diff.storage.push(PackageStorageDiff {
                package: path.clone(),
                diff: storage,
            });
        }
        diff.findings.extend(items(
            &subject,
            b.map(|p| p.findings.as_slice()).unwrap_or_default(),
            a.map(|p| p.findings.as_slice()).unwrap_or_default(),
            |f| f.id.as_str().into(),
            |b, a| b == a,
        ));
    }
    diff
}
