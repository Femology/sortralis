//! Verified SDK v28 lib.rs documents contractimpl public exports and contractevent
//! topic/data attributes. We preserve explicit syntax, without macro expansion.
use crate::{
    collect, evidence, rules::bindings, storage::fields, ParsedSource, SourceError, SourceOptions,
};
use doctor_core::{
    source_inventory::*,
    storage::{ContractTypeShape, StorageVariant},
};
use quote::ToTokens;
use std::{collections::BTreeMap, fs, path::Path};
use syn::{Item, Visibility};
fn spelling<T: ToTokens>(value: &T) -> String {
    value.to_token_stream().to_string()
}
fn attributes(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc"))
        .map(spelling)
        .collect()
}
fn sdk(
    path: &syn::Path,
    name: &str,
    names: &BTreeMap<String, String>,
    options: &SourceOptions,
) -> bool {
    let parts: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    match parts.as_slice() {
        [local] => names.get(local).is_some_and(|n| n == name),
        [namespace, item] => {
            item == name
                && (options.sdk_crate_names.contains(namespace)
                    || names.get(namespace).is_some_and(|n| n == "sdk"))
        }
        _ => false,
    }
}
fn analyze_items(
    source: &ParsedSource,
    items: &[Item],
    module: &str,
    options: &SourceOptions,
    inventory: &mut SourceInventory,
) -> Result<(), SourceError> {
    let names = bindings(items, &options.sdk_crate_names);
    let has = |attrs: &[syn::Attribute], name: &str| {
        attrs.iter().any(|a| sdk(a.path(), name, &names, options))
    };
    let identity = |name: &str| format!("{}::{module}{name}", source.path.display());
    for item in items {
        match item {
            Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    analyze_items(
                        source,
                        items,
                        &format!("{module}{}::", m.ident),
                        options,
                        inventory,
                    )?;
                }
            }
            Item::Impl(i) if has(&i.attrs, "contractimpl") => {
                for item in &i.items {
                    if let syn::ImplItem::Fn(f) = item {
                        if matches!(f.vis, Visibility::Public(_)) || i.trait_.is_some() {
                            let mut attrs = attributes(&i.attrs);
                            attrs.extend(attributes(&f.attrs));
                            inventory.functions.push(SourceFunction {
                                // SDK export names are function names alone, per
                                // exact v28 contractimpl docs; collisions stay visible.
                                identity: f.sig.ident.to_string(),
                                owner: spelling(&i.self_ty),
                                signature: spelling(&f.sig),
                                attributes: attrs,
                                evidence: evidence(
                                    source,
                                    f.sig.ident.span(),
                                    "Explicit contractimpl function; compiled exports may differ."
                                        .into(),
                                )?,
                            });
                        }
                    }
                }
                if i.trait_.is_some() {
                    inventory.uncertainties.push(evidence(source, i.impl_token.span,
                        "Trait/default method expansion is unresolved; interface-like inventory is incomplete.".into())?);
                }
            }
            Item::Struct(s) if has(&s.attrs, "contracttype") => {
                let (shape, fields) = fields(&s.fields);
                inventory.types.push(SourceType {
                    identity: identity(&s.ident.to_string()),
                    shape: ContractTypeShape::Struct { shape, fields },
                    attributes: attributes(&s.attrs),
                    evidence: evidence(
                        source,
                        s.ident.span(),
                        "Explicit contracttype source declaration.".into(),
                    )?,
                });
            }
            Item::Enum(e) if has(&e.attrs, "contracttype") => {
                inventory.types.push(SourceType {
                    identity: identity(&e.ident.to_string()),
                    shape: ContractTypeShape::Enum {
                        variants: e
                            .variants
                            .iter()
                            .map(|v| {
                                let (shape, fields) = fields(&v.fields);
                                StorageVariant {
                                    name: v.ident.to_string(),
                                    shape,
                                    fields,
                                    discriminant: v.discriminant.as_ref().map(|(_, e)| spelling(e)),
                                }
                            })
                            .collect(),
                    },
                    attributes: attributes(&e.attrs),
                    evidence: evidence(
                        source,
                        e.ident.span(),
                        "Explicit contracttype enum source declaration.".into(),
                    )?,
                });
            }
            Item::Struct(s) if has(&s.attrs, "contractevent") => {
                let (shape, _) = fields(&s.fields);
                inventory.events.push(SourceEvent { identity: identity(&s.ident.to_string()),
                    attributes: attributes(&s.attrs), shape,
                    fields: s.fields.iter().enumerate().map(|(index,f)| EventField {
                        name: f.ident.as_ref().map(ToString::to_string).unwrap_or_else(|| index.to_string()),
                        type_name: spelling(&f.ty), attributes: attributes(&f.attrs),
                    }).collect(),
                    evidence: evidence(source, s.ident.span(), "Explicit event declaration; field topic attributes and macro arguments retained as source syntax.".into())? });
            }
            Item::Macro(m) => inventory.uncertainties.push(evidence(
                source,
                m.mac.path.segments[0].ident.span(),
                "Item macro is not expanded; functions/types/events may be generated.".into(),
            )?),
            _ => {}
        }
    }
    Ok(())
}
pub fn inventory_interface_text(
    path: &Path,
    text: &str,
    options: &SourceOptions,
) -> Result<SourceInventory, SourceError> {
    options.validate()?;
    let source = ParsedSource::parse(path, text, &options.sdk_crate_names)?;
    let mut inventory = SourceInventory::default();
    analyze_items(&source, &source.syntax.items, "", options, &mut inventory)?;
    Ok(inventory)
}
pub fn inventory_interface_directory(
    root: &Path,
    options: &SourceOptions,
) -> Result<SourceInventory, SourceError> {
    options.validate()?;
    let root = fs::canonicalize(root).map_err(|source| SourceError::Io {
        path: root.into(),
        source,
    })?;
    let mut paths = Vec::new();
    collect(&root, &root, options, &mut paths)?;
    paths.sort();
    let mut result = SourceInventory::default();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|source| SourceError::Io {
            path: path.clone(),
            source,
        })?;
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| SourceError::InvalidExclusion(path.clone()))?;
        let mut inventory = inventory_interface_text(relative, &text, options)?;
        result.functions.append(&mut inventory.functions);
        result.types.append(&mut inventory.types);
        result.events.append(&mut inventory.events);
        result.uncertainties.append(&mut inventory.uncertainties);
    }
    result.functions.sort_by(|a, b| {
        (&a.identity, &a.owner, &a.signature).cmp(&(&b.identity, &b.owner, &b.signature))
    });
    result.types.sort_by(|a, b| a.identity.cmp(&b.identity));
    result.events.sort_by(|a, b| a.identity.cmp(&b.identity));
    Ok(result)
}
