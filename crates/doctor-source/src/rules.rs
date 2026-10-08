use crate::{evidence, ParsedSource, SourceError, TargetContext};
use doctor_core::{Category, Evidence, Severity};
use std::collections::BTreeMap;
use syn::{
    spanned::Spanned,
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
const LEGACY_UPGRADE_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-API-001",
    title: "Legacy update_current_contract_wasm API detected",
    description: "The contract calls update_current_contract_wasm, which is deprecated in SDK v28 in favor of Protocol 28 contract executable migration patterns.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::Breaking,
    category: Category::Source,
    why_it_matters: "Protocol 28 updates the contract deployment and executable architecture (ContractExecutable). Calling update_current_contract_wasm directly requires migration.",
    recommendation: "Migrate contract upgrade logic to the supported Protocol 28 ContractExecutable mechanisms.",
    limitations: "AST method-call inspection on syntax nodes; does not evaluate runtime control flow or macros.",
    references: &[MIGRATION],
};
const LEGACY_DEPLOY_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-DEPLOY-001",
    title: "Deprecated deploy_v2 API detected",
    description: "The contract calls DeployerWithAddress::deploy_v2, which is deprecated in SDK v28 in favor of deploy_contract with ContractExecutable.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::Breaking,
    category: Category::Source,
    why_it_matters: "SDK v28 deprecates deploy_v2 and replaces it with deploy_contract, which accepts ContractExecutable.",
    recommendation: "Replace deploy_v2 with deploy_contract and wrap the Wasm hash in ContractExecutable::Wasm.",
    limitations: "Syntax-only AST method-call inspection. It intentionally does not flag with_current_contract, with_address, or upload_contract_wasm because those remain supported in SDK v28. It also does not flag a generic method named deploy because type resolution is unavailable and that would create broad false positives.",
    references: &[MIGRATION],
};
const SPARSE_EVENT_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-EVENT-001",
    title: "Sparse event risk detected",
    description: "Contract event or publish call contains unit/void () values that SDK v28 omits in map-format events.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::ManualReview,
    category: Category::Event,
    why_it_matters: "SDK v28 omits void-valued data fields in map-format events, potentially breaking downstream decoders expecting complete fields.",
    recommendation: "Review None/unit event fields or use contractevent(sparse = false) where field presence is strictly required.",
    limitations: "AST inspection on struct definitions and event publish calls.",
    references: &[MIGRATION],
};
const CONTRACTTRAIT_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-MACRO-001",
    title: "Invalid contracttrait macro usage",
    description: "The #[contracttrait] attribute is applied to a non-trait item.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::Breaking,
    category: Category::Source,
    why_it_matters: "Contract traits can only be declared on trait definitions; applying #[contracttrait] to structs, enums, or functions is invalid.",
    recommendation: "Apply #[contracttrait] only to trait declarations.",
    limitations: "AST attribute check on non-trait item syntax nodes.",
    references: &[MIGRATION],
};
const INTERNAL_SPEC_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-SPEC-001",
    title: "Direct reference to internal spec symbol",
    description: "Contract code references internal __SPEC_XDR_ or ScSpecEntry symbols directly.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::Breaking,
    category: Category::Source,
    why_it_matters:
        "Internal spec symbols are macro-generated and change layout/format between SDK versions.",
    recommendation:
        "Avoid referencing internal spec symbols directly; use standard SDK contract interfaces.",
    limitations: "AST identifier inspection.",
    references: &[MIGRATION],
};
const UPGRADE_AUTH_DOC: RuleDocumentation = RuleDocumentation {
    id: "P28-AUTH-001",
    title: "Upgrade authorization requires review",
    description: "No direct or verified macro-based authorization signal is visible on an upgrade function.",
    supported_context: TargetContext::Sdk28,
    severity: Severity::ManualReview,
    category: Category::Auth,
    why_it_matters: "Upgrade entry points must be authorization-protected, but syntax-only analysis cannot always prove authorization delegated through helpers, traits, or macros.",
    recommendation: "Verify that the upgrade entry point or its trusted caller enforces authorization before changing contract executable code.",
    limitations: "AST check for require_auth / require_auth_for_args and explicitly resolved stellar_macros authorization attributes (only_role, only_owner, only_admin, only_any_role). It does not expand arbitrary macros or prove cross-function/caller authorization, so unresolved cases remain manual review.",
    references: &[MIGRATION],
};

pub struct ExportArgumentRule;
pub struct CustomAccountRule;
pub struct LegacyUpgradeRule;
pub struct LegacyDeployRule;
pub struct SparseEventRule;
pub struct ContractTraitRule;
pub struct InternalSpecRule;
pub struct UpgradeAuthRule;

static EXPORT: ExportArgumentRule = ExportArgumentRule;
static ACCOUNT: CustomAccountRule = CustomAccountRule;
static LEGACY_UPGRADE: LegacyUpgradeRule = LegacyUpgradeRule;
static LEGACY_DEPLOY: LegacyDeployRule = LegacyDeployRule;
static SPARSE_EVENT: SparseEventRule = SparseEventRule;
static CONTRACT_TRAIT: ContractTraitRule = ContractTraitRule;
static INTERNAL_SPEC: InternalSpecRule = InternalSpecRule;
static UPGRADE_AUTH: UpgradeAuthRule = UpgradeAuthRule;

static REGISTRY: [RuleRegistration; 9] = [
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
    RuleRegistration {
        documentation: &LEGACY_UPGRADE_DOC,
        analyzer: Some(&LEGACY_UPGRADE),
    },
    RuleRegistration {
        documentation: &LEGACY_DEPLOY_DOC,
        analyzer: Some(&LEGACY_DEPLOY),
    },
    RuleRegistration {
        documentation: &SPARSE_EVENT_DOC,
        analyzer: Some(&SPARSE_EVENT),
    },
    RuleRegistration {
        documentation: &CONTRACTTRAIT_DOC,
        analyzer: Some(&CONTRACT_TRAIT),
    },
    RuleRegistration {
        documentation: &INTERNAL_SPEC_DOC,
        analyzer: Some(&INTERNAL_SPEC),
    },
    RuleRegistration {
        documentation: &UPGRADE_AUTH_DOC,
        analyzer: Some(&UPGRADE_AUTH),
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

struct LegacyUpgradeVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for LegacyUpgradeVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "update_current_contract_wasm" {
            self.locations.push(call.method.span());
        }
        visit::visit_expr_method_call(self, call);
    }
}
impl SourceRule for LegacyUpgradeRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &LEGACY_UPGRADE_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = LegacyUpgradeVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "Legacy update_current_contract_wasm call detected; upgrade mechanism changed in SDK v28".into(),
                )
            })
            .collect()
    }
}

struct LegacyDeployVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for LegacyDeployVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "deploy_v2" {
            self.locations.push(call.method.span());
        }
        visit::visit_expr_method_call(self, call);
    }
}
impl SourceRule for LegacyDeployRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &LEGACY_DEPLOY_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = LegacyDeployVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "Deprecated deploy_v2 call detected; SDK v28 replaces it with deploy_contract(ContractExecutable, constructor_args)".into(),
                )
            })
            .collect()
    }
}

fn is_or_contains_unit(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Tuple(t) => t.elems.is_empty() || t.elems.iter().any(is_or_contains_unit),
        _ => false,
    }
}

struct SparseEventVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for SparseEventVisitor {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let is_event = item.attrs.iter().any(|attr| {
            attr.path().is_ident("contractevent")
                || attr
                    .path()
                    .segments
                    .iter()
                    .any(|seg| seg.ident == "contractevent")
        });
        let has_sparse_false = item.attrs.iter().any(|attr| {
            if let syn::Meta::List(list) = &attr.meta {
                let s = list.tokens.to_string();
                s.contains("sparse = false") || s.contains("sparse=false")
            } else {
                false
            }
        });
        if is_event && !has_sparse_false {
            for field in &item.fields {
                if let syn::Type::Tuple(tuple) = &field.ty {
                    if tuple.elems.is_empty() {
                        self.locations.push(field.span());
                    }
                }
            }
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "publish" {
            for arg in &call.args {
                if is_or_contains_unit(arg) {
                    self.locations.push(call.method.span());
                }
            }
        }
        visit::visit_expr_method_call(self, call);
    }
}
impl SourceRule for SparseEventRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &SPARSE_EVENT_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = SparseEventVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "Event definition or publish call contains unit/void () values subject to v28 sparse map omission".into(),
                )
            })
            .collect()
    }
}

struct ContractTraitVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for ContractTraitVisitor {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        for attr in &item.attrs {
            if attr.path().is_ident("contracttrait")
                || attr
                    .path()
                    .segments
                    .iter()
                    .any(|s| s.ident == "contracttrait")
            {
                self.locations.push(attr.span());
            }
        }
        visit::visit_item_struct(self, item);
    }
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        for attr in &item.attrs {
            if attr.path().is_ident("contracttrait")
                || attr
                    .path()
                    .segments
                    .iter()
                    .any(|s| s.ident == "contracttrait")
            {
                self.locations.push(attr.span());
            }
        }
        visit::visit_item_enum(self, item);
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        for attr in &item.attrs {
            if attr.path().is_ident("contracttrait")
                || attr
                    .path()
                    .segments
                    .iter()
                    .any(|s| s.ident == "contracttrait")
            {
                self.locations.push(attr.span());
            }
        }
        visit::visit_item_fn(self, item);
    }
}
impl SourceRule for ContractTraitRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &CONTRACTTRAIT_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = ContractTraitVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "#[contracttrait] applied to non-trait item; contract traits must be declared on trait definitions".into(),
                )
            })
            .collect()
    }
}

struct InternalSpecVisitor {
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for InternalSpecVisitor {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        let name = ident.to_string();
        if name.starts_with("__SPEC_XDR_")
            || name.starts_with("__spec_xdr_")
            || name == "ScSpecEntry"
        {
            self.locations.push(ident.span());
        }
    }
}
impl SourceRule for InternalSpecRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &INTERNAL_SPEC_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let mut visitor = InternalSpecVisitor {
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "Direct reference to internal spec symbol detected; spec generation is managed by SDK macros".into(),
                )
            })
            .collect()
    }
}

fn contains_require_auth(block: &syn::Block) -> bool {
    struct AuthFinder {
        found: bool,
    }
    impl<'ast> Visit<'ast> for AuthFinder {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            if call.method == "require_auth" || call.method == "require_auth_for_args" {
                self.found = true;
            }
            visit::visit_expr_method_call(self, call);
        }
    }
    let mut finder = AuthFinder { found: false };
    finder.visit_block(block);
    finder.found
}

fn is_upgrade_fn(name: &str, block: &syn::Block) -> bool {
    if name == "upgrade" || name == "upgrade_contract" || name == "update_contract" {
        return true;
    }
    struct UpgradeCallFinder {
        found: bool,
    }
    impl<'ast> Visit<'ast> for UpgradeCallFinder {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            if call.method == "update_current_contract_wasm" {
                self.found = true;
            }
            visit::visit_expr_method_call(self, call);
        }
    }
    let mut finder = UpgradeCallFinder { found: false };
    finder.visit_block(block);
    finder.found
}

fn verified_auth_attribute(attrs: &[syn::Attribute], names: &BTreeMap<String, String>) -> bool {
    const AUTH_MACROS: [&str; 4] = ["only_role", "only_owner", "only_admin", "only_any_role"];

    attrs.iter().any(|attr| {
        let segments: Vec<_> = attr
            .path()
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();

        match segments.as_slice() {
            [name] => names
                .get(name)
                .is_some_and(|resolved| AUTH_MACROS.contains(&resolved.as_str())),
            [namespace, name] => {
                AUTH_MACROS.contains(&name.as_str())
                    && (namespace == "stellar_macros"
                        || names
                            .get(namespace)
                            .is_some_and(|resolved| resolved == "sdk"))
            }
            _ => false,
        }
    })
}

struct UpgradeAuthVisitor {
    names: BTreeMap<String, String>,
    locations: Vec<proc_macro2::Span>,
}
impl<'ast> Visit<'ast> for UpgradeAuthVisitor {
    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        let name = method.sig.ident.to_string();
        if is_upgrade_fn(&name, &method.block)
            && !contains_require_auth(&method.block)
            && !verified_auth_attribute(&method.attrs, &self.names)
        {
            self.locations.push(method.sig.ident.span());
        }
        visit::visit_impl_item_fn(self, method);
    }
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        let name = func.sig.ident.to_string();
        if is_upgrade_fn(&name, &func.block)
            && !contains_require_auth(&func.block)
            && !verified_auth_attribute(&func.attrs, &self.names)
        {
            self.locations.push(func.sig.ident.span());
        }
        visit::visit_item_fn(self, func);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if let Some((_, items)) = &item.content {
            let macro_crates = [String::from("stellar_macros")];
            let parent = std::mem::replace(&mut self.names, bindings(items, &macro_crates));
            for item in items {
                self.visit_item(item);
            }
            self.names = parent;
        }
    }
}
impl SourceRule for UpgradeAuthRule {
    fn documentation(&self) -> &'static RuleDocumentation {
        &UPGRADE_AUTH_DOC
    }
    fn analyze(
        &self,
        source: &ParsedSource,
        context: TargetContext,
    ) -> Result<Vec<Evidence>, SourceError> {
        if context != self.documentation().supported_context {
            return Ok(Vec::new());
        }
        let macro_crates = [String::from("stellar_macros")];
        let mut visitor = UpgradeAuthVisitor {
            names: bindings(&source.syntax.items, &macro_crates),
            locations: Vec::new(),
        };
        visit::visit_file(&mut visitor, &source.syntax);
        visitor
            .locations
            .into_iter()
            .map(|span| {
                evidence(
                    source,
                    span,
                    "No direct or verified macro-based authorization signal found on this upgrade function; review caller, trait, helper, or macro authorization".into(),
                )
            })
            .collect()
    }
}
