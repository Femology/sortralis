//! Contract interface normalization and deterministic before/after diff engine.
use crate::WasmError;
use doctor_core::{interface::*, source_inventory::SourceInventory, storage::ContractTypeShape};
use quote::ToTokens;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Format an ScSpecTypeDef JSON value into a canonical readable type representation.
pub fn format_spec_type(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Object(obj) => {
            if let Some(udt) = obj.get("udt") {
                if let Some(name) = udt.get("name").and_then(|n| n.as_str()) {
                    return name.to_string();
                }
            }
            if let Some(opt) = obj.get("option") {
                if let Some(val_ty) = opt.get("value_type") {
                    return format!("Option<{}>", format_spec_type(val_ty));
                }
            }
            if let Some(res) = obj.get("result") {
                let ok = res
                    .get("ok_type")
                    .map(format_spec_type)
                    .unwrap_or_else(|| "()".into());
                let err = res
                    .get("error_type")
                    .map(format_spec_type)
                    .unwrap_or_else(|| "Error".into());
                return format!("Result<{}, {}>", ok, err);
            }
            if let Some(vec) = obj.get("vec") {
                if let Some(elem) = vec.get("element_type") {
                    return format!("Vec<{}>", format_spec_type(elem));
                }
            }
            if let Some(map) = obj.get("map") {
                let k = map
                    .get("key_type")
                    .map(format_spec_type)
                    .unwrap_or_else(|| "()".into());
                let v = map
                    .get("value_type")
                    .map(format_spec_type)
                    .unwrap_or_else(|| "()".into());
                return format!("Map<{}, {}>", k, v);
            }
            if let Some(tuple) = obj.get("tuple") {
                if let Some(types) = tuple.get("type_").and_then(|t| t.as_array()) {
                    let items: Vec<_> = types.iter().map(format_spec_type).collect();
                    return format!("({})", items.join(", "));
                }
            }
            serde_json::to_string(obj).unwrap_or_else(|_| "unknown".into())
        }
        _ => "unknown".into(),
    }
}

/// Parse verified Stellar CLI `interface` JSON entries into a normalized `ContractInterface`.
pub fn parse_contract_interface(entries: &[Value]) -> Result<ContractInterface, WasmError> {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut errors = Vec::new();
    let mut events = Vec::new();

    for entry in entries {
        let obj = entry.as_object().ok_or_else(|| {
            WasmError::InvalidInterface("expected a JSON object for contract spec entry".into())
        })?;

        // SCSpecFunctionV0
        if let Some(f_val) = obj.get("function_v0") {
            let name = f_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = f_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut parameters = Vec::new();
            if let Some(inputs) = f_val.get("inputs").and_then(|i| i.as_array()) {
                for input in inputs {
                    let p_name = input
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let p_type = input
                        .get("type_")
                        .map(format_spec_type)
                        .unwrap_or_else(|| "unknown".into());
                    parameters.push(NormalizedParameter {
                        name: p_name,
                        type_name: p_type,
                    });
                }
            }

            let return_type = match f_val.get("outputs").and_then(|o| o.as_array()) {
                Some(outputs) if !outputs.is_empty() => {
                    let first = format_spec_type(&outputs[0]);
                    if first == "void" {
                        None
                    } else if outputs.len() == 1 {
                        Some(first)
                    } else {
                        let formatted: Vec<_> = outputs.iter().map(format_spec_type).collect();
                        Some(format!("({})", formatted.join(", ")))
                    }
                }
                _ => None,
            };

            functions.push(NormalizedFunction {
                name,
                doc,
                parameters,
                return_type,
            });
        }
        // SCSpecUDTStructV0
        else if let Some(s_val) = obj.get("udt_struct_v0").or_else(|| obj.get("struct_v0")) {
            let name = s_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = s_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut fields = Vec::new();
            if let Some(field_arr) = s_val.get("fields").and_then(|f| f.as_array()) {
                for field in field_arr {
                    let f_name = field
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let f_type = field
                        .get("type_")
                        .map(format_spec_type)
                        .unwrap_or_else(|| "unknown".into());
                    fields.push(NormalizedField {
                        name: f_name,
                        type_name: f_type,
                    });
                }
            }

            types.push(NormalizedType {
                name,
                doc,
                kind: NormalizedTypeKind::Struct { fields },
            });
        }
        // SCSpecUDTUnionV0 / SCSpecUDTEnumV0
        else if let Some(u_val) = obj.get("udt_union_v0").or_else(|| obj.get("union_v0")) {
            let name = u_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = u_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut variants = Vec::new();
            if let Some(cases) = u_val.get("cases").and_then(|c| c.as_array()) {
                for case in cases {
                    if let Some(void_case) = case.get("void_v0") {
                        let v_name = void_case
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default()
                            .to_string();
                        variants.push(NormalizedVariant {
                            name: v_name,
                            discriminant: None,
                            fields: Vec::new(),
                        });
                    } else if let Some(tuple_case) = case.get("tuple_v0") {
                        let v_name = tuple_case
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let mut fields = Vec::new();
                        if let Some(t_types) = tuple_case.get("type_").and_then(|t| t.as_array()) {
                            for (idx, ty) in t_types.iter().enumerate() {
                                fields.push(NormalizedField {
                                    name: format!("_{idx}"),
                                    type_name: format_spec_type(ty),
                                });
                            }
                        }
                        variants.push(NormalizedVariant {
                            name: v_name,
                            discriminant: None,
                            fields,
                        });
                    }
                }
            }

            types.push(NormalizedType {
                name,
                doc,
                kind: NormalizedTypeKind::Enum { variants },
            });
        } else if let Some(e_val) = obj.get("udt_enum_v0").or_else(|| obj.get("enum_v0")) {
            let name = e_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = e_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut variants = Vec::new();
            if let Some(cases) = e_val.get("cases").and_then(|c| c.as_array()) {
                for case in cases {
                    let v_name = case
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let disc = case.get("value").and_then(|v| v.as_u64()).map(|v| v as u32);
                    variants.push(NormalizedVariant {
                        name: v_name,
                        discriminant: disc,
                        fields: Vec::new(),
                    });
                }
            }

            types.push(NormalizedType {
                name,
                doc,
                kind: NormalizedTypeKind::Enum { variants },
            });
        }
        // SCSpecUDTErrorEnumV0
        else if let Some(err_val) = obj
            .get("udt_error_enum_v0")
            .or_else(|| obj.get("error_enum_v0"))
        {
            let name = err_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = err_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut cases = Vec::new();
            if let Some(case_arr) = err_val.get("cases").and_then(|c| c.as_array()) {
                for case in case_arr {
                    let c_name = case
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let value = case.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let c_doc = case
                        .get("doc")
                        .and_then(|d| d.as_str())
                        .unwrap_or_default()
                        .to_string();
                    cases.push(NormalizedErrorCase {
                        name: c_name,
                        value,
                        doc: c_doc,
                    });
                }
            }

            cases.sort_by(|a, b| (a.value, &a.name).cmp(&(b.value, &b.name)));
            errors.push(NormalizedError { name, doc, cases });
        }
        // SCSpecEventV0
        else if let Some(ev_val) = obj.get("event_v0") {
            let name = ev_val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            let doc = ev_val
                .get("doc")
                .and_then(|d| d.as_str())
                .unwrap_or_default()
                .to_string();

            let mut topics = Vec::new();
            if let Some(t_arr) = ev_val.get("prefix_topics").and_then(|t| t.as_array()) {
                for t in t_arr {
                    if let Some(s) = t.as_str() {
                        topics.push(s.to_string());
                    }
                }
            }

            let mut params = Vec::new();
            if let Some(p_arr) = ev_val.get("params").and_then(|p| p.as_array()) {
                for p in p_arr {
                    let p_name = p
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let p_type = p
                        .get("type_")
                        .map(format_spec_type)
                        .unwrap_or_else(|| "unknown".into());
                    params.push(NormalizedParameter {
                        name: p_name,
                        type_name: p_type,
                    });
                }
            }

            let data_format = ev_val
                .get("data_format")
                .and_then(|d| d.as_str())
                .map(ToString::to_string);

            events.push(NormalizedEvent {
                name,
                doc,
                topics,
                params,
                data_format,
            });
        }
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    types.sort_by(|a, b| a.name.cmp(&b.name));
    errors.sort_by(|a, b| a.name.cmp(&b.name));
    events.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ContractInterface {
        analysis_source: AnalysisSource::WasmContractSpec,
        scope: INTERFACE_WASM_SCOPE.into(),
        functions,
        types,
        errors,
        events,
    })
}

/// Convert source syntax observations into a normalized `ContractInterface` labeled as `source_approximation`.
pub fn interface_from_source(inventory: &SourceInventory) -> ContractInterface {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut events = Vec::new();

    for f in &inventory.functions {
        let mut parameters = Vec::new();
        let mut return_type = None;

        if let Ok(sig) = syn::parse_str::<syn::Signature>(&f.signature) {
            for input in sig.inputs {
                if let syn::FnArg::Typed(pat) = input {
                    let param_name = pat.pat.to_token_stream().to_string();
                    let param_type = pat.ty.to_token_stream().to_string();
                    // Host Env parameter is omitted from external contract interface
                    if param_type == "Env" || param_type.ends_with(":: Env") {
                        continue;
                    }
                    parameters.push(NormalizedParameter {
                        name: param_name,
                        type_name: param_type,
                    });
                }
            }
            match sig.output {
                syn::ReturnType::Default => {}
                syn::ReturnType::Type(_, ty) => {
                    let ret = ty.to_token_stream().to_string();
                    return_type = Some(ret);
                }
            }
        }

        functions.push(NormalizedFunction {
            name: f.identity.clone(),
            doc: String::new(),
            parameters,
            return_type,
        });
    }

    for t in &inventory.types {
        let clean_name = t
            .identity
            .split("::")
            .last()
            .unwrap_or(&t.identity)
            .to_string();
        match &t.shape {
            ContractTypeShape::Struct { fields, .. } => {
                let norm_fields = fields
                    .iter()
                    .map(|f| NormalizedField {
                        name: f.name.clone(),
                        type_name: f.type_name.clone(),
                    })
                    .collect();
                types.push(NormalizedType {
                    name: clean_name,
                    doc: String::new(),
                    kind: NormalizedTypeKind::Struct {
                        fields: norm_fields,
                    },
                });
            }
            ContractTypeShape::Enum { variants } => {
                let norm_variants = variants
                    .iter()
                    .map(|v| NormalizedVariant {
                        name: v.name.clone(),
                        discriminant: v
                            .discriminant
                            .as_deref()
                            .and_then(|d| d.parse::<u32>().ok()),
                        fields: v
                            .fields
                            .iter()
                            .map(|f| NormalizedField {
                                name: f.name.clone(),
                                type_name: f.type_name.clone(),
                            })
                            .collect(),
                    })
                    .collect();
                types.push(NormalizedType {
                    name: clean_name,
                    doc: String::new(),
                    kind: NormalizedTypeKind::Enum {
                        variants: norm_variants,
                    },
                });
            }
        }
    }

    for ev in &inventory.events {
        let clean_name = ev
            .identity
            .split("::")
            .last()
            .unwrap_or(&ev.identity)
            .to_string();
        let params = ev
            .fields
            .iter()
            .map(|f| NormalizedParameter {
                name: f.name.clone(),
                type_name: f.type_name.clone(),
            })
            .collect();
        events.push(NormalizedEvent {
            name: clean_name,
            doc: String::new(),
            topics: Vec::new(),
            params,
            data_format: None,
        });
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    types.sort_by(|a, b| a.name.cmp(&b.name));
    events.sort_by(|a, b| a.name.cmp(&b.name));

    ContractInterface {
        analysis_source: AnalysisSource::SourceApproximation,
        scope: INTERFACE_SOURCE_SCOPE.into(),
        functions,
        types,
        errors: Vec::new(),
        events,
    }
}

/// Deterministic before/after comparison of two normalized contract interfaces.
pub fn diff_interfaces(before: &ContractInterface, after: &ContractInterface) -> InterfaceDiff {
    let analysis_source = if before.analysis_source == AnalysisSource::SourceApproximation
        || after.analysis_source == AnalysisSource::SourceApproximation
    {
        AnalysisSource::SourceApproximation
    } else {
        AnalysisSource::WasmContractSpec
    };

    let scope = match analysis_source {
        AnalysisSource::WasmContractSpec => INTERFACE_WASM_SCOPE.to_string(),
        AnalysisSource::SourceApproximation => INTERFACE_SOURCE_SCOPE.to_string(),
    };

    let mut records = Vec::new();

    // 1. Compare functions
    let before_fns: BTreeMap<&str, &NormalizedFunction> = before
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();
    let after_fns: BTreeMap<&str, &NormalizedFunction> = after
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();

    let all_fn_names: BTreeSet<&str> = before_fns.keys().chain(after_fns.keys()).copied().collect();

    for name in all_fn_names {
        match (before_fns.get(name), after_fns.get(name)) {
            (Some(b), None) => {
                records.push(InterfaceDiffRecord {
                    id: "FUNCTION_REMOVED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::Breaking,
                    summary: format!(
                        "Public function '{name}' was removed from the contract interface."
                    ),
                    before_evidence: Some(b.signature_display()),
                    after_evidence: None,
                });
            }
            (None, Some(a)) => {
                records.push(InterfaceDiffRecord {
                    id: "FUNCTION_ADDED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::NonBreaking,
                    summary: format!(
                        "New public function '{name}' was added to the contract interface."
                    ),
                    before_evidence: None,
                    after_evidence: Some(a.signature_display()),
                });
            }
            (Some(b), Some(a)) => {
                // Return type comparison
                if b.return_type != a.return_type {
                    let old_ret = b.return_type.as_deref().unwrap_or("()");
                    let new_ret = a.return_type.as_deref().unwrap_or("()");
                    records.push(InterfaceDiffRecord {
                        id: "FUNCTION_RETURN_TYPE_CHANGED".into(),
                        subject: name.to_string(),
                        classification: InterfaceClassification::Breaking,
                        summary: format!("Return type of function '{name}' changed from '{old_ret}' to '{new_ret}'."),
                        before_evidence: Some(b.signature_display()),
                        after_evidence: Some(a.signature_display()),
                    });
                }

                // Parameter comparisons
                if b.parameters.len() != a.parameters.len() {
                    if a.parameters.len() > b.parameters.len() {
                        records.push(InterfaceDiffRecord {
                            id: "FUNCTION_PARAMETER_ADDED".into(),
                            subject: name.to_string(),
                            classification: InterfaceClassification::Breaking,
                            summary: format!("Function '{name}' added parameter(s). Expected {} arguments, now expects {}.", b.parameters.len(), a.parameters.len()),
                            before_evidence: Some(b.signature_display()),
                            after_evidence: Some(a.signature_display()),
                        });
                    } else {
                        records.push(InterfaceDiffRecord {
                            id: "FUNCTION_PARAMETER_REMOVED".into(),
                            subject: name.to_string(),
                            classification: InterfaceClassification::Breaking,
                            summary: format!("Function '{name}' removed parameter(s). Expected {} arguments, now expects {}.", b.parameters.len(), a.parameters.len()),
                            before_evidence: Some(b.signature_display()),
                            after_evidence: Some(a.signature_display()),
                        });
                    }
                } else {
                    // Check for reordering: same parameter (name, type) pairs but different sequence
                    let b_multiset: BTreeSet<(&str, &str)> = b
                        .parameters
                        .iter()
                        .map(|p| (p.name.as_str(), p.type_name.as_str()))
                        .collect();
                    let a_multiset: BTreeSet<(&str, &str)> = a
                        .parameters
                        .iter()
                        .map(|p| (p.name.as_str(), p.type_name.as_str()))
                        .collect();

                    if b_multiset == a_multiset && b.parameters != a.parameters {
                        records.push(InterfaceDiffRecord {
                            id: "FUNCTION_PARAMETER_REORDERED".into(),
                            subject: name.to_string(),
                            classification: InterfaceClassification::Breaking,
                            summary: format!("Parameters for function '{name}' were reordered."),
                            before_evidence: Some(b.signature_display()),
                            after_evidence: Some(a.signature_display()),
                        });
                    } else {
                        // Compare index-by-index
                        for (i, (bp, ap)) in b.parameters.iter().zip(&a.parameters).enumerate() {
                            if bp.type_name != ap.type_name {
                                records.push(InterfaceDiffRecord {
                                    id: "FUNCTION_PARAMETER_TYPE_CHANGED".into(),
                                    subject: format!("{name}::{}", bp.name),
                                    classification: InterfaceClassification::Breaking,
                                    summary: format!("Parameter '{}' (index {i}) in function '{name}' changed type from '{}' to '{}'.", bp.name, bp.type_name, ap.type_name),
                                    before_evidence: Some(format!("{}: {}", bp.name, bp.type_name)),
                                    after_evidence: Some(format!("{}: {}", ap.name, ap.type_name)),
                                });
                            } else if bp.name != ap.name {
                                records.push(InterfaceDiffRecord {
                                    id: "FUNCTION_PARAMETER_RENAMED".into(),
                                    subject: format!("{name}::{}", bp.name),
                                    classification: InterfaceClassification::ReviewRequired,
                                    summary: format!("Parameter at index {i} in function '{name}' was renamed from '{}' to '{}'. Callers using named parameters may be affected.", bp.name, ap.name),
                                    before_evidence: Some(format!("{}: {}", bp.name, bp.type_name)),
                                    after_evidence: Some(format!("{}: {}", ap.name, ap.type_name)),
                                });
                            }
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }

    // 2. Compare public types (Structs and Enums)
    let before_types: BTreeMap<&str, &NormalizedType> =
        before.types.iter().map(|t| (t.name.as_str(), t)).collect();
    let after_types: BTreeMap<&str, &NormalizedType> =
        after.types.iter().map(|t| (t.name.as_str(), t)).collect();

    let all_type_names: BTreeSet<&str> = before_types
        .keys()
        .chain(after_types.keys())
        .copied()
        .collect();

    for name in all_type_names {
        match (before_types.get(name), after_types.get(name)) {
            (Some(_), None) => {
                records.push(InterfaceDiffRecord {
                    id: "TYPE_REMOVED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::Breaking,
                    summary: format!("Public contract type '{name}' was removed."),
                    before_evidence: Some(format!("type {name}")),
                    after_evidence: None,
                });
            }
            (None, Some(_)) => {
                records.push(InterfaceDiffRecord {
                    id: "TYPE_ADDED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::NonBreaking,
                    summary: format!("Public contract type '{name}' was added."),
                    before_evidence: None,
                    after_evidence: Some(format!("type {name}")),
                });
            }
            (Some(bt), Some(at)) => match (&bt.kind, &at.kind) {
                (
                    NormalizedTypeKind::Struct { fields: bf },
                    NormalizedTypeKind::Struct { fields: af },
                ) => {
                    let bf_map: BTreeMap<&str, &NormalizedField> =
                        bf.iter().map(|f| (f.name.as_str(), f)).collect();
                    let af_map: BTreeMap<&str, &NormalizedField> =
                        af.iter().map(|f| (f.name.as_str(), f)).collect();
                    let all_field_names: BTreeSet<&str> =
                        bf_map.keys().chain(af_map.keys()).copied().collect();

                    for fname in all_field_names {
                        match (bf_map.get(fname), af_map.get(fname)) {
                            (Some(old_f), None) => {
                                records.push(InterfaceDiffRecord {
                                    id: "STRUCT_FIELD_REMOVED".into(),
                                    subject: format!("{name}::{fname}"),
                                    classification: InterfaceClassification::Breaking,
                                    summary: format!(
                                        "Field '{fname}' ({}) was removed from struct '{name}'.",
                                        old_f.type_name
                                    ),
                                    before_evidence: Some(format!("{fname}: {}", old_f.type_name)),
                                    after_evidence: None,
                                });
                            }
                            (None, Some(new_f)) => {
                                let is_opt = new_f.type_name.starts_with("Option<");
                                let classification = if is_opt {
                                    InterfaceClassification::ReviewRequired
                                } else {
                                    InterfaceClassification::Breaking
                                };
                                let label = if is_opt { "Optional field" } else { "Field" };
                                records.push(InterfaceDiffRecord {
                                    id: "STRUCT_FIELD_ADDED".into(),
                                    subject: format!("{name}::{fname}"),
                                    classification,
                                    summary: format!(
                                        "{label} '{fname}' ({}) was added to struct '{name}'.",
                                        new_f.type_name
                                    ),
                                    before_evidence: None,
                                    after_evidence: Some(format!("{fname}: {}", new_f.type_name)),
                                });
                            }
                            (Some(old_f), Some(new_f)) => {
                                if old_f.type_name != new_f.type_name {
                                    records.push(InterfaceDiffRecord {
                                            id: "STRUCT_FIELD_TYPE_CHANGED".into(),
                                            subject: format!("{name}::{fname}"),
                                            classification: InterfaceClassification::Breaking,
                                            summary: format!("Field '{fname}' in struct '{name}' changed type from '{}' to '{}'.", old_f.type_name, new_f.type_name),
                                            before_evidence: Some(format!("{fname}: {}", old_f.type_name)),
                                            after_evidence: Some(format!("{fname}: {}", new_f.type_name)),
                                        });
                                }
                            }
                            (None, None) => {}
                        }
                    }
                }
                (
                    NormalizedTypeKind::Enum { variants: bv },
                    NormalizedTypeKind::Enum { variants: av },
                ) => {
                    let bv_map: BTreeMap<&str, &NormalizedVariant> =
                        bv.iter().map(|v| (v.name.as_str(), v)).collect();
                    let av_map: BTreeMap<&str, &NormalizedVariant> =
                        av.iter().map(|v| (v.name.as_str(), v)).collect();
                    let all_var_names: BTreeSet<&str> =
                        bv_map.keys().chain(av_map.keys()).copied().collect();

                    for vname in all_var_names {
                        match (bv_map.get(vname), av_map.get(vname)) {
                            (Some(_), None) => {
                                records.push(InterfaceDiffRecord {
                                    id: "ENUM_VARIANT_REMOVED".into(),
                                    subject: format!("{name}::{vname}"),
                                    classification: InterfaceClassification::Breaking,
                                    summary: format!(
                                        "Variant '{vname}' was removed from enum '{name}'."
                                    ),
                                    before_evidence: Some(format!("variant {vname}")),
                                    after_evidence: None,
                                });
                            }
                            (None, Some(_)) => {
                                records.push(InterfaceDiffRecord {
                                        id: "ENUM_VARIANT_ADDED".into(),
                                        subject: format!("{name}::{vname}"),
                                        classification: InterfaceClassification::ReviewRequired,
                                        summary: format!("Variant '{vname}' was added to enum '{name}'. Review client match exhaustiveness."),
                                        before_evidence: None,
                                        after_evidence: Some(format!("variant {vname}")),
                                    });
                            }
                            (Some(ov), Some(nv)) => {
                                if ov.discriminant != nv.discriminant || ov.fields != nv.fields {
                                    records.push(InterfaceDiffRecord {
                                            id: "ENUM_VARIANT_CHANGED".into(),
                                            subject: format!("{name}::{vname}"),
                                            classification: InterfaceClassification::Breaking,
                                            summary: format!("Variant '{vname}' in enum '{name}' changed fields or discriminant representation."),
                                            before_evidence: Some(format!("{:?} {:?}", ov.discriminant, ov.fields)),
                                            after_evidence: Some(format!("{:?} {:?}", nv.discriminant, nv.fields)),
                                        });
                                }
                            }
                            (None, None) => {}
                        }
                    }
                }
                _ => {
                    records.push(InterfaceDiffRecord {
                        id: "TYPE_KIND_CHANGED".into(),
                        subject: name.to_string(),
                        classification: InterfaceClassification::Breaking,
                        summary: format!(
                            "Public contract type '{name}' changed kind (struct vs enum)."
                        ),
                        before_evidence: Some(format!("type {name}")),
                        after_evidence: Some(format!("type {name}")),
                    });
                }
            },
            (None, None) => {}
        }
    }

    // 3. Compare errors
    let before_errs: BTreeMap<&str, &NormalizedError> =
        before.errors.iter().map(|e| (e.name.as_str(), e)).collect();
    let after_errs: BTreeMap<&str, &NormalizedError> =
        after.errors.iter().map(|e| (e.name.as_str(), e)).collect();

    let all_err_names: BTreeSet<&str> = before_errs
        .keys()
        .chain(after_errs.keys())
        .copied()
        .collect();

    for name in all_err_names {
        match (before_errs.get(name), after_errs.get(name)) {
            (Some(_), None) => {
                records.push(InterfaceDiffRecord {
                    id: "ERROR_TYPE_REMOVED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::Breaking,
                    summary: format!("Error definition '{name}' was removed."),
                    before_evidence: Some(format!("error {name}")),
                    after_evidence: None,
                });
            }
            (None, Some(_)) => {
                records.push(InterfaceDiffRecord {
                    id: "ERROR_TYPE_ADDED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::NonBreaking,
                    summary: format!("Error definition '{name}' was added."),
                    before_evidence: None,
                    after_evidence: Some(format!("error {name}")),
                });
            }
            (Some(be), Some(ae)) => {
                let be_cases: BTreeMap<&str, &NormalizedErrorCase> =
                    be.cases.iter().map(|c| (c.name.as_str(), c)).collect();
                let ae_cases: BTreeMap<&str, &NormalizedErrorCase> =
                    ae.cases.iter().map(|c| (c.name.as_str(), c)).collect();

                let all_case_names: BTreeSet<&str> =
                    be_cases.keys().chain(ae_cases.keys()).copied().collect();

                for cname in all_case_names {
                    match (be_cases.get(cname), ae_cases.get(cname)) {
                        (Some(oc), None) => {
                            records.push(InterfaceDiffRecord {
                                id: "ERROR_CASE_REMOVED".into(),
                                subject: format!("{name}::{cname}"),
                                classification: InterfaceClassification::Breaking,
                                summary: format!(
                                    "Error case '{cname}' ({}) was removed from error '{name}'.",
                                    oc.value
                                ),
                                before_evidence: Some(format!("{cname} = {}", oc.value)),
                                after_evidence: None,
                            });
                        }
                        (None, Some(nc)) => {
                            records.push(InterfaceDiffRecord {
                                id: "ERROR_CASE_ADDED".into(),
                                subject: format!("{name}::{cname}"),
                                classification: InterfaceClassification::NonBreaking,
                                summary: format!(
                                    "Error case '{cname}' ({}) was added to error '{name}'.",
                                    nc.value
                                ),
                                before_evidence: None,
                                after_evidence: Some(format!("{cname} = {}", nc.value)),
                            });
                        }
                        (Some(oc), Some(nc)) => {
                            if oc.value != nc.value {
                                records.push(InterfaceDiffRecord {
                                    id: "ERROR_CASE_VALUE_CHANGED".into(),
                                    subject: format!("{name}::{cname}"),
                                    classification: InterfaceClassification::Breaking,
                                    summary: format!("Error case '{cname}' in error '{name}' changed value from {} to {}.", oc.value, nc.value),
                                    before_evidence: Some(format!("{cname} = {}", oc.value)),
                                    after_evidence: Some(format!("{cname} = {}", nc.value)),
                                });
                            }
                        }
                        (None, None) => {}
                    }
                }
            }
            (None, None) => {}
        }
    }

    // 4. Compare events
    let before_evs: BTreeMap<&str, &NormalizedEvent> =
        before.events.iter().map(|e| (e.name.as_str(), e)).collect();
    let after_evs: BTreeMap<&str, &NormalizedEvent> =
        after.events.iter().map(|e| (e.name.as_str(), e)).collect();

    let all_ev_names: BTreeSet<&str> = before_evs.keys().chain(after_evs.keys()).copied().collect();

    for name in all_ev_names {
        match (before_evs.get(name), after_evs.get(name)) {
            (Some(_), None) => {
                records.push(InterfaceDiffRecord {
                    id: "EVENT_REMOVED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::ReviewRequired,
                    summary: format!("Event '{name}' was removed."),
                    before_evidence: Some(format!("event {name}")),
                    after_evidence: None,
                });
            }
            (None, Some(_)) => {
                records.push(InterfaceDiffRecord {
                    id: "EVENT_ADDED".into(),
                    subject: name.to_string(),
                    classification: InterfaceClassification::NonBreaking,
                    summary: format!("Event '{name}' was added."),
                    before_evidence: None,
                    after_evidence: Some(format!("event {name}")),
                });
            }
            (Some(be), Some(ae)) => {
                if be.topics != ae.topics || be.params != ae.params {
                    records.push(InterfaceDiffRecord {
                        id: "EVENT_SHAPE_CHANGED".into(),
                        subject: name.to_string(),
                        classification: InterfaceClassification::Breaking,
                        summary: format!("Event '{name}' parameter types, counts, or topic configuration changed."),
                        before_evidence: Some(format!("topics: {:?}, params: {:?}", be.topics, be.params)),
                        after_evidence: Some(format!("topics: {:?}, params: {:?}", ae.topics, ae.params)),
                    });
                }
            }
            (None, None) => {}
        }
    }

    // Deterministic sorting: Breaking first, then ReviewRequired, then NonBreaking; then ID, then Subject
    records.sort_by(|a, b| {
        b.classification
            .cmp(&a.classification)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.subject.cmp(&b.subject))
    });

    let has_breaking_changes = records
        .iter()
        .any(|r| r.classification == InterfaceClassification::Breaking);

    InterfaceDiff {
        schema_version: INTERFACE_SCHEMA_VERSION.into(),
        scope,
        analysis_source,
        before: before.clone(),
        after: after.clone(),
        records,
        has_breaking_changes,
    }
}
