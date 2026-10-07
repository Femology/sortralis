use crate::{evidence, ParsedSource, SourceError, TargetContext};
use doctor_core::{Category, Evidence, Severity};
use std::collections::BTreeMap;
use syn::{
    visit::{self, Visit},
    Meta, Token, UseTree,
};

const MIGRATION: &str =
    "https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs";
#[derive(Debug)]
pub struct RuleDocumentation {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub supported_context: TargetContext,
    pub severity: Severity,
    pub category: Category,
    pub why_it_matters: &'static str,
    pub recommendation: &'static str,
    pub limitations: &'static str,
    pub references: &'static [&'static str],
}
pub trait SourceRule: Sync {
    fn documentation(&self) -> &'static RuleDocumentation;
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError>;
}
pub struct RuleRegistration {
    pub documentation: &'static RuleDocumentation,
    /// None means documentation-only/manual; it does not mean the check passed.
    pub analyzer: Option<&'static dyn SourceRule>,
}
const EXPORT_DOC: RuleDocumentation = RuleDocumentation {
    id: "SDK28_REMOVED_EXPORT_ARGUMENT",title: "Removed SDK v28 export argument",
    description: "A Soroban contracttype or contracterror attribute uses the removed export argument.",
    supported_context: TargetContext::Sdk28,severity: Severity::Breaking,category: Category::Source,
    why_it_matters: "SDK v28 rejects this argument during macro processing.",
    recommendation: "Remove export = ... from the attribute and rebuild with the supported SDK toolchain.",
    limitations: "Syntax only: scans explicit SDK-qualified attributes and direct SDK imports/aliases, including wildcard imports. No macro expansion, cfg evaluation, cfg_attr, cross-file reexports, or function-local import resolution. Conditional source is reported for review before applying changes.",
    references: &[MIGRATION],
};
const ACCOUNT_DOC: RuleDocumentation = RuleDocumentation {
    id: "CUSTOM_ACCOUNT_EXECUTABLE_REVIEW",title: "Review custom-account executable authorization",
    description: "An impl method named __check_auth requires Protocol 28 executable/deployment argument review.",
    supported_context: TargetContext::Sdk28,severity: Severity::ManualReview,category: Category::Auth,
    why_it_matters: "Deployment authorization contexts may contain ContractExecutable::ExternalRef. Older custom accounts cannot unpack that variant; v28 accounts must decide how to authorize it.",
    recommendation: "Review CreateContractHostFn and CreateContractWithCtorHostFn handling and executable match arms. The presence of __check_auth alone does not establish a defect.",
    limitations: "Detects implemented methods, including trait implementations, not free functions, calls, or trait declarations. No type checking or proof of exported account behavior; macro-generated methods are not expanded.",
    references: &[MIGRATION],
};
const EVENT_DOC: RuleDocumentation = RuleDocumentation {
    id: "SDK28_EVENT_SHAPE_REVIEW",title: "Manual event-shape review",
    description: "Currently manual: review consumers of map-format contract events when upgrading to SDK v28.",
    supported_context: TargetContext::Sdk28,severity: Severity::ManualReview,category: Category::Event,
    why_it_matters: "SDK v28 omits void-valued data fields in map-format events. Consumer expectations cannot be established from Rust syntax alone.",
    recommendation: "Review None/unit data fields and downstream decoders. Where every field must remain present, evaluate contractevent(sparse = false). This entry has no automatic detector and does not certify event compatibility.",
    limitations: "Manual only. Topic/data roles, format/opt-out arguments, type aliases and consumer expectations need semantic review. No synthetic finding is emitted.",
    references: &[MIGRATION],
};
pub struct ExportArgumentRule;
pub struct CustomAccountRule;
static EXPORT: ExportArgumentRule = ExportArgumentRule;
static ACCOUNT: CustomAccountRule = CustomAccountRule;
static REGISTRY: [RuleRegistration; 3] = [
    RuleRegistration {
        documentation: &EXPORT_DOC,
        analyzer: Some(&EXPORT),
    },
    RuleRegistration {
        documentation: &ACCOUNT_DOC,
        analyzer: Some(&ACCOUNT),
    },
    RuleRegistration {
        documentation: &EVENT_DOC,
        analyzer: None,
    },
];
pub fn registry() -> &'static [RuleRegistration] {
    &REGISTRY
}

// Track direct module-level imports, rather than equating every macro with the
// same final identifier to the SDK. This is deliberately not a Rust name resolver.
pub(crate) fn bindings(items: &[syn::Item], sdk_names: &[String]) -> BTreeMap<String, String> {
    fn imports(tree: &UseTree, prefix: &mut Vec<String>, output: &mut Vec<(Vec<String>, String)>) {
        match tree {
            UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                imports(&path.tree, prefix, output);
                prefix.pop();
            }
            UseTree::Name(name) => {
                let mut path = prefix.clone();
                path.push(name.ident.to_string());
                output.push((path, name.ident.to_string()));
            }
            UseTree::Rename(rename) => {
                let mut path = prefix.clone();
                path.push(rename.ident.to_string());
                output.push((path, rename.rename.to_string()));
            }
            UseTree::Group(group) => {
                for tree in &group.items {
                    imports(tree, prefix, output);
                }
            }
            UseTree::Glob(_) => {
                let mut path = prefix.clone();
                path.push("*".into());
                output.push((path, "*".into()));
            }
        }
    }
    let mut collected = Vec::new();
    for item in items {
        if let syn::Item::Use(item) = item {
            imports(&item.tree, &mut Vec::new(), &mut collected);
        }
    }
    let mut namespaces = sdk_names.to_vec();
    let mut map = BTreeMap::new();
    for item in items {
        if let syn::Item::ExternCrate(item) = item {
            if sdk_names.contains(&item.ident.to_string()) {
                if let Some((_, alias)) = &item.rename {
                    namespaces.push(alias.to_string());
                    map.insert(alias.to_string(), "sdk".into());
                }
            }
        }
    }
    let is_sdk = |path: &[String]| path.first().is_some_and(|name| namespaces.contains(name));
    let ambiguous_glob = collected
        .iter()
        .any(|(path, _)| path.last().is_some_and(|name| name == "*") && !is_sdk(path));
    if !ambiguous_glob
        && collected
            .iter()
            .any(|(path, _)| path.len() == 2 && is_sdk(path) && path[1] == "*")
    {
        for name in [
            "contracttype",
            "contracterror",
            "contractimpl",
            "contractevent",
            "Env",
            "Symbol",
            "String",
            "symbol_short",
        ] {
            map.insert(name.into(), name.into());
        }
    }
    // Explicit imports override globs in Rust, including non-SDK macros.
    for (path, alias) in collected {
        if path.last().is_some_and(|name| name == "*") {
            continue;
        }
        if is_sdk(&path) && path.len() == 1 {
            map.insert(alias, "sdk".into());
        } else if is_sdk(&path) && path.len() == 2 {
            map.insert(alias, path[1].clone());
        } else {
            map.remove(&alias);
        }
    }
    for item in items {
        if let syn::Item::Macro(item) = item {
            if let Some(ident) = &item.ident {
                map.remove(&ident.to_string());
            }
        }
    }
    map
}
struct ExportVisitor<'a> {
    source: &'a ParsedSource,
    names: BTreeMap<String, String>,
    locations: Vec<(proc_macro2::Span, String)>,
    error: Option<syn::Error>,
}
impl ExportVisitor<'_> {
    fn sdk_attribute(&self, path: &syn::Path) -> bool {
        let names: Vec<_> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let macro_name = match names.as_slice() {
            [name] => self.names.get(name).map(String::as_str),
            [namespace, name]
                if self.source.sdk_crate_names.contains(namespace)
                    || self
                        .names
                        .get(namespace)
                        .is_some_and(|value| value == "sdk") =>
            {
                Some(name.as_str())
            }
            _ => None,
        };
        matches!(macro_name, Some("contracttype" | "contracterror"))
    }
}
impl<'ast> Visit<'ast> for ExportVisitor<'_> {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if !self.sdk_attribute(attribute.path()) || !matches!(attribute.meta, Meta::List(_)) {
            return;
        }
        match attribute
            .parse_args_with(syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated)
        {
            Ok(arguments) => {
                for argument in arguments {
                    if let Meta::NameValue(value) = argument {
                        if value.path.is_ident("export") {
                            self.locations.push((value.path.segments[0].ident.span(),"Removed export argument on SDK contracttype/contracterror attribute".into()));
                        }
                    }
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        for attribute in &item.attrs {
            self.visit_attribute(attribute);
        }
        if let Some((_, items)) = &item.content {
            let parent = std::mem::replace(
                &mut self.names,
                bindings(items, &self.source.sdk_crate_names),
            );
            for item in items {
                self.visit_item(item);
            }
            self.names = parent;
        }
    }
    // Local scopes need name resolution; do not infer bindings from outer scopes.
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
    fn visit_impl_item_fn(&mut self, _: &'ast syn::ImplItemFn) {}
}
impl SourceRule for ExportArgumentRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &EXPORT_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = ExportVisitor {
            source,
            names: bindings(&source.syntax.items, &source.sdk_crate_names),
            locations: Vec::new(),
            error: None,
        };
        visitor.visit_file(&source.syntax);
        if let Some(error) = visitor.error {
            return Err(SourceError::Parse {
                path: source.path.clone(),
                source: error,
            });
        }
        visitor
            .locations
            .into_iter()
            .map(|(span, message)| evidence(source, span, message))
            .collect()
    }
}
struct AccountVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for AccountVisitor {
    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        if method.sig.ident == "__check_auth" {
            self.locations.push(method.sig.ident.span());
        }
    }
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
    fn visit_macro(&mut self, _: &'ast syn::Macro) {}
}
impl SourceRule for CustomAccountRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &ACCOUNT_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = AccountVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor.locations.into_iter().map(|span| evidence(source,span,"Implemented __check_auth method; review executable/deployment authorization, without assuming a defect".into())).collect()
    }
}
