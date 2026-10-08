#![allow(clippy::unwrap_used)]

use doctor_core::interface::*;
use doctor_source::{inventory_interface_directory, SourceOptions};
use doctor_wasm::{diff_interfaces, interface_from_source, parse_contract_interface};
use serde_json::json;
use std::path::PathBuf;

fn sample_wasm_entries() -> Vec<serde_json::Value> {
    vec![
        json!({
            "function_v0": {
                "doc": "Adds two numbers",
                "name": "add",
                "inputs": [
                    {"doc": "", "name": "a", "type_": "u64"},
                    {"doc": "", "name": "b", "type_": "u64"}
                ],
                "outputs": ["u64"]
            }
        }),
        json!({
            "udt_struct_v0": {
                "doc": "Record struct",
                "lib": "",
                "name": "Record",
                "fields": [
                    {"doc": "", "name": "id", "type_": "u64"},
                    {"doc": "", "name": "name", "type_": "string"}
                ]
            }
        }),
        json!({
            "udt_union_v0": {
                "doc": "Status enum",
                "lib": "",
                "name": "Status",
                "cases": [
                    {"void_v0": {"doc": "", "name": "Active"}},
                    {"void_v0": {"doc": "", "name": "Inactive"}}
                ]
            }
        }),
        json!({
            "udt_error_enum_v0": {
                "doc": "Contract errors",
                "lib": "",
                "name": "Error",
                "cases": [
                    {"doc": "", "name": "NotFound", "value": 1}
                ]
            }
        }),
    ]
}

#[test]
fn test_identical_spec() {
    let entries = sample_wasm_entries();
    let iface = parse_contract_interface(&entries).unwrap();
    let diff = diff_interfaces(&iface, &iface);

    assert_eq!(diff.analysis_source, AnalysisSource::WasmContractSpec);
    assert!(!diff.has_breaking_changes);
    assert!(diff.records.is_empty());

    let mut report = Vec::new();
    doctor_report::write_interface_diff(&mut report, &diff).unwrap();
    let report_str = String::from_utf8(report).unwrap();
    assert!(report_str.contains("Identical contract interface"));
    assert!(report_str.contains("Verdict: CHECKS_COMPLETED"));
}

#[test]
fn test_function_added() {
    let entries_before = sample_wasm_entries();
    let mut entries_after = sample_wasm_entries();
    entries_after.push(json!({
        "function_v0": {
            "doc": "",
            "name": "multiply",
            "inputs": [
                {"doc": "", "name": "x", "type_": "u64"},
                {"doc": "", "name": "y", "type_": "u64"}
            ],
            "outputs": ["u64"]
        }
    }));

    let before = parse_contract_interface(&entries_before).unwrap();
    let after = parse_contract_interface(&entries_after).unwrap();
    let diff = diff_interfaces(&before, &after);

    assert!(!diff.has_breaking_changes);
    let added = diff
        .records
        .iter()
        .find(|r| r.id == "FUNCTION_ADDED")
        .unwrap();
    assert_eq!(added.subject, "multiply");
    assert_eq!(added.classification, InterfaceClassification::NonBreaking);
}

#[test]
fn test_function_removed() {
    let entries_before = sample_wasm_entries();
    // After has no functions
    let entries_after: Vec<_> = sample_wasm_entries()
        .into_iter()
        .filter(|e| !e.as_object().unwrap().contains_key("function_v0"))
        .collect();

    let before = parse_contract_interface(&entries_before).unwrap();
    let after = parse_contract_interface(&entries_after).unwrap();
    let diff = diff_interfaces(&before, &after);

    assert!(diff.has_breaking_changes);
    let removed = diff
        .records
        .iter()
        .find(|r| r.id == "FUNCTION_REMOVED")
        .unwrap();
    assert_eq!(removed.subject, "add");
    assert_eq!(removed.classification, InterfaceClassification::Breaking);
}

#[test]
fn test_argument_type_changed() {
    let before = parse_contract_interface(&sample_wasm_entries()).unwrap();
    let after_entries = vec![json!({
        "function_v0": {
            "doc": "",
            "name": "add",
            "inputs": [
                {"doc": "", "name": "a", "type_": "u32"}, // changed from u64
                {"doc": "", "name": "b", "type_": "u64"}
            ],
            "outputs": ["u64"]
        }
    })];
    let after = parse_contract_interface(&after_entries).unwrap();
    let diff = diff_interfaces(&before, &after);

    assert!(diff.has_breaking_changes);
    let record = diff
        .records
        .iter()
        .find(|r| r.id == "FUNCTION_PARAMETER_TYPE_CHANGED")
        .unwrap();
    assert_eq!(record.classification, InterfaceClassification::Breaking);
    assert!(record.summary.contains("changed type from 'u64' to 'u32'"));
}

#[test]
fn test_return_type_changed() {
    let before = parse_contract_interface(&sample_wasm_entries()).unwrap();
    let after_entries = vec![json!({
        "function_v0": {
            "doc": "",
            "name": "add",
            "inputs": [
                {"doc": "", "name": "a", "type_": "u64"},
                {"doc": "", "name": "b", "type_": "u64"}
            ],
            "outputs": ["u32"] // changed from u64
        }
    })];
    let after = parse_contract_interface(&after_entries).unwrap();
    let diff = diff_interfaces(&before, &after);

    assert!(diff.has_breaking_changes);
    let record = diff
        .records
        .iter()
        .find(|r| r.id == "FUNCTION_RETURN_TYPE_CHANGED")
        .unwrap();
    assert_eq!(record.classification, InterfaceClassification::Breaking);
    assert!(record.summary.contains("changed from 'u64' to 'u32'"));
}

#[test]
fn test_struct_field_change() {
    let before = parse_contract_interface(&sample_wasm_entries()).unwrap();

    // 1. Field type changed -> BREAKING
    let after_changed_type = vec![json!({
        "udt_struct_v0": {
            "doc": "",
            "lib": "",
            "name": "Record",
            "fields": [
                {"doc": "", "name": "id", "type_": "u32"}, // u64 -> u32
                {"doc": "", "name": "name", "type_": "string"}
            ]
        }
    })];
    let diff1 = diff_interfaces(
        &before,
        &parse_contract_interface(&after_changed_type).unwrap(),
    );
    assert!(diff1.has_breaking_changes);
    let r1 = diff1
        .records
        .iter()
        .find(|r| r.id == "STRUCT_FIELD_TYPE_CHANGED")
        .unwrap();
    assert_eq!(r1.classification, InterfaceClassification::Breaking);

    // 2. Field removed -> BREAKING
    let after_field_removed = vec![json!({
        "udt_struct_v0": {
            "doc": "",
            "lib": "",
            "name": "Record",
            "fields": [
                {"doc": "", "name": "id", "type_": "u64"}
            ]
        }
    })];
    let diff2 = diff_interfaces(
        &before,
        &parse_contract_interface(&after_field_removed).unwrap(),
    );
    assert!(diff2.has_breaking_changes);
    let r2 = diff2
        .records
        .iter()
        .find(|r| r.id == "STRUCT_FIELD_REMOVED")
        .unwrap();
    assert_eq!(r2.classification, InterfaceClassification::Breaking);

    // 3. Optional field added -> REVIEW_REQUIRED
    let after_opt_added = vec![json!({
        "udt_struct_v0": {
            "doc": "",
            "lib": "",
            "name": "Record",
            "fields": [
                {"doc": "", "name": "id", "type_": "u64"},
                {"doc": "", "name": "name", "type_": "string"},
                {"doc": "", "name": "tag", "type_": {"option": {"value_type": "string"}}}
            ]
        }
    })];
    let diff3 = diff_interfaces(
        &before,
        &parse_contract_interface(&after_opt_added).unwrap(),
    );
    let r3 = diff3
        .records
        .iter()
        .find(|r| r.id == "STRUCT_FIELD_ADDED")
        .unwrap();
    assert_eq!(r3.classification, InterfaceClassification::ReviewRequired);
}

#[test]
fn test_enum_variant_change() {
    let before = parse_contract_interface(&sample_wasm_entries()).unwrap();

    // 1. Variant removed -> BREAKING
    let after_removed = vec![json!({
        "udt_union_v0": {
            "doc": "",
            "lib": "",
            "name": "Status",
            "cases": [
                {"void_v0": {"doc": "", "name": "Active"}}
            ]
        }
    })];
    let diff1 = diff_interfaces(&before, &parse_contract_interface(&after_removed).unwrap());
    assert!(diff1.has_breaking_changes);
    let r1 = diff1
        .records
        .iter()
        .find(|r| r.id == "ENUM_VARIANT_REMOVED")
        .unwrap();
    assert_eq!(r1.classification, InterfaceClassification::Breaking);

    // 2. Variant added -> REVIEW_REQUIRED
    let after_added = vec![json!({
        "udt_union_v0": {
            "doc": "",
            "lib": "",
            "name": "Status",
            "cases": [
                {"void_v0": {"doc": "", "name": "Active"}},
                {"void_v0": {"doc": "", "name": "Inactive"}},
                {"void_v0": {"doc": "", "name": "Pending"}}
            ]
        }
    })];
    let diff2 = diff_interfaces(&before, &parse_contract_interface(&after_added).unwrap());
    let r2 = diff2
        .records
        .iter()
        .find(|r| r.id == "ENUM_VARIANT_ADDED")
        .unwrap();
    assert_eq!(r2.classification, InterfaceClassification::ReviewRequired);
}

#[test]
fn test_output_ordering_deterministic() {
    let before = parse_contract_interface(&sample_wasm_entries()).unwrap();
    let after_entries = vec![
        json!({
            "function_v0": {
                "doc": "",
                "name": "new_fn",
                "inputs": [],
                "outputs": []
            }
        }),
        json!({
            "udt_struct_v0": {
                "doc": "",
                "lib": "",
                "name": "Record",
                "fields": [
                    {"doc": "", "name": "id", "type_": "string"}, // breaking
                    {"doc": "", "name": "name", "type_": "string"}
                ]
            }
        }),
    ];
    let after = parse_contract_interface(&after_entries).unwrap();

    let diff1 = diff_interfaces(&before, &after);
    let diff2 = diff_interfaces(&before, &after);

    assert_eq!(diff1.records, diff2.records);
    // Breaking changes must come first
    assert_eq!(
        diff1.records[0].classification,
        InterfaceClassification::Breaking
    );
}

#[test]
fn test_source_approximation_labeling() {
    let fixture_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/git-diff/before");
    let options = SourceOptions::default();
    let inv = inventory_interface_directory(&fixture_dir, &options).unwrap();
    let iface = interface_from_source(&inv);

    assert_eq!(iface.analysis_source, AnalysisSource::SourceApproximation);
    assert_eq!(iface.scope, INTERFACE_SOURCE_SCOPE);

    let diff = diff_interfaces(&iface, &iface);
    assert_eq!(diff.analysis_source, AnalysisSource::SourceApproximation);

    let mut report = Vec::new();
    doctor_report::write_interface_diff(&mut report, &diff).unwrap();
    let report_str = String::from_utf8(report).unwrap();
    assert!(report_str.contains("Source-derived approximation: not an exact on-ledger"));
}

#[test]
fn test_phase_gate_fixture_comparison() {
    // Known breaking fixture: fixtures/git-diff/before vs after
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/git-diff");
    let before_dir = root.join("before");
    let after_dir = root.join("after");

    let options = SourceOptions::default();
    let before_inv = inventory_interface_directory(&before_dir, &options).unwrap();
    let after_inv = inventory_interface_directory(&after_dir, &options).unwrap();

    let before_iface = interface_from_source(&before_inv);
    let after_iface = interface_from_source(&after_inv);

    let diff = diff_interfaces(&before_iface, &after_iface);

    // Phase gate: Known breaking fixture changes must NEVER be labeled non-breaking
    assert!(
        diff.has_breaking_changes,
        "known breaking fixture must have breaking changes"
    );

    // 1. 'removed' function removed must be BREAKING
    let removed_fn = diff
        .records
        .iter()
        .find(|r| r.subject == "removed")
        .unwrap();
    assert_eq!(removed_fn.id, "FUNCTION_REMOVED");
    assert_eq!(removed_fn.classification, InterfaceClassification::Breaking);

    // 2. 'update' return type changed (u32 -> u64) must be BREAKING
    let update_fn = diff.records.iter().find(|r| r.subject == "update").unwrap();
    assert_eq!(update_fn.id, "FUNCTION_RETURN_TYPE_CHANGED");
    assert_eq!(update_fn.classification, InterfaceClassification::Breaking);

    // 3. 'State' struct field 'count' type changed (u32 -> u64) must be BREAKING
    let state_field = diff
        .records
        .iter()
        .find(|r| r.subject.ends_with("count"))
        .unwrap();
    assert_eq!(state_field.id, "STRUCT_FIELD_TYPE_CHANGED");
    assert_eq!(
        state_field.classification,
        InterfaceClassification::Breaking
    );

    // 4. 'added' function added is NON_BREAKING
    let added_fn = diff.records.iter().find(|r| r.subject == "added").unwrap();
    assert_eq!(added_fn.id, "FUNCTION_ADDED");
    assert_eq!(
        added_fn.classification,
        InterfaceClassification::NonBreaking
    );
}
