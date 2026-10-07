//! Verified against soroban-sdk 28.0.0 src/env.rs and src/storage.rs.
//! https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/storage.rs
use crate::{collect, evidence, rules::bindings, ParsedSource, SourceError, SourceOptions};
use doctor_core::storage::*;
use quote::ToTokens;
use std::{collections::BTreeMap, fs, path::Path};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Expr, Fields, GenericArgument, Item, Pat, Type,
};

fn spelling<T: ToTokens>(item: &T) -> String {
    item.to_token_stream().to_string()
}
fn bare(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(e) => bare(&e.expr),
        Expr::Paren(e) => bare(&e.expr),
        Expr::Group(e) => bare(&e.expr),
        _ => expr,
    }
}
fn bare_type(ty: &Type) -> &Type {
    match ty {
        Type::Reference(t) => bare_type(&t.elem),
        Type::Paren(t) => bare_type(&t.elem),
        _ => ty,
    }
}
fn storage_like(expr: &Expr) -> bool {
    matches!(bare(expr), Expr::MethodCall(c) if c.method == "storage" || storage_like(&c.receiver))
}
fn fields(fields: &Fields) -> (String, Vec<StorageField>) {
    let shape = match fields {
        Fields::Named(_) => "named",
        Fields::Unnamed(_) => "tuple",
        Fields::Unit => "unit",
    };
    (
        shape.into(),
        fields
            .iter()
            .enumerate()
            .map(|(i, f)| StorageField {
                name: f
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| i.to_string()),
                type_name: spelling(&f.ty),
            })
            .collect(),
    )
}
#[derive(Default, Clone)]
struct Local {
    env: bool,
    storage: bool,
    durability: Option<Durability>,
    type_name: Option<String>,
    key: Option<StorageKey>,
}
struct Analyzer<'a> {
    source: &'a ParsedSource,
    names: BTreeMap<String, String>,
    module: Vec<String>,
    locals: BTreeMap<String, Local>,
    constants: BTreeMap<String, Local>,
    inventory: StorageInventory,
    error: Option<SourceError>,
}
impl Analyzer<'_> {
    fn identity(&self, name: &str) -> String {
        format!(
            "{}::{}{}",
            self.source.path.display(),
            if self.module.is_empty() {
                String::new()
            } else {
                format!("{}::", self.module.join("::"))
            },
            name
        )
    }
    fn sdk(&self, path: &syn::Path, name: &str) -> bool {
        let parts: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        match parts.as_slice() {
            [local] => self.names.get(local).is_some_and(|n| n == name),
            [namespace, item] => {
                item == name
                    && (self.source.sdk_crate_names.contains(namespace)
                        || self.names.get(namespace).is_some_and(|n| n == "sdk"))
            }
            _ => false,
        }
    }
    fn env_type(&self, ty: &Type) -> bool {
        matches!(bare_type(ty), Type::Path(t) if self.sdk(&t.path, "Env"))
    }
    fn variable(&self, expr: &Expr) -> Option<&Local> {
        let Expr::Path(p) = bare(expr) else {
            return None;
        };
        let name = p.path.get_ident()?.to_string();
        self.locals.get(&name).or_else(|| self.constants.get(&name))
    }
    fn is_env(&self, expr: &Expr) -> bool {
        if let Some(local) = self.variable(expr) {
            return local.env;
        }
        matches!(bare(expr), Expr::MethodCall(c) if c.method == "clone" && c.args.is_empty() && self.is_env(&c.receiver))
    }
    fn is_storage(&self, expr: &Expr) -> bool {
        if let Some(local) = self.variable(expr) {
            return local.storage;
        }
        matches!(bare(expr), Expr::MethodCall(c) if c.method == "storage" && c.args.is_empty() && self.is_env(&c.receiver))
    }
    fn durability(&self, expr: &Expr) -> Option<Durability> {
        if let Some(local) = self.variable(expr) {
            return local.durability;
        }
        let Expr::MethodCall(c) = bare(expr) else {
            return None;
        };
        if !c.args.is_empty() || !self.is_storage(&c.receiver) {
            return None;
        }
        match c.method.to_string().as_str() {
            "instance" => Some(Durability::Instance),
            "persistent" => Some(Durability::Persistent),
            "temporary" => Some(Durability::Temporary),
            _ => None,
        }
    }
    fn type_identity(&self, path: &syn::Path) -> Option<String> {
        let parts: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        // Only same-module and explicit crate/self paths. No last-name guessing.
        let candidate = match parts.as_slice() {
            [name] => self.identity(name),
            [first, rest @ ..] if first == "crate" => {
                format!("{}::{}", self.source.path.display(), rest.join("::"))
            }
            [first, rest @ ..] if first == "self" => self.identity(&rest.join("::")),
            _ => format!("{}::{}", self.source.path.display(), parts.join("::")),
        };
        self.inventory
            .contract_types
            .iter()
            .any(|t| t.identity == candidate)
            .then_some(candidate)
    }
    fn enum_key(&self, path: &syn::Path, args: &[&Expr]) -> Option<StorageKey> {
        if path.segments.len() < 2 {
            return None;
        }
        let mut type_path = syn::parse2::<syn::Path>(path.to_token_stream()).ok()?;
        let variant = type_path.segments.pop()?.ident.to_string();
        type_path.segments.pop_punct();
        let identity = self.type_identity(&type_path)?;
        let definition = self
            .inventory
            .contract_types
            .iter()
            .find(|t| t.identity == identity)?;
        let ContractTypeShape::Enum { variants } = &definition.shape else {
            return None;
        };
        if !variants.iter().any(|v| v.name == variant) {
            return None;
        }
        let args: Vec<_> = args.iter().map(|e| self.key(e)).collect();
        let dynamic = args.iter().any(|k| k.dynamic);
        let payload = if args.is_empty() {
            String::new()
        } else {
            format!(
                "({})",
                args.iter()
                    .map(|k| if k.dynamic {
                        "*".into()
                    } else {
                        k.identity.clone()
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        Some(StorageKey {
            identity: format!("enum:{identity}::{variant}{payload}"),
            expression: spelling(path),
            kind: KeyKind::EnumVariant,
            dynamic,
            contract_type: Some(identity),
        })
    }
    fn key(&self, expr: &Expr) -> StorageKey {
        let expr = bare(expr);
        if let Some(key) = self.variable(expr).and_then(|v| v.key.clone()) {
            return key;
        }
        let known = match expr {
            Expr::Lit(e) => match &e.lit {
                syn::Lit::Str(s) => Some((KeyKind::String, format!("string:{:?}", s.value()))),
                syn::Lit::Int(_) | syn::Lit::Bool(_) | syn::Lit::Char(_) => Some((
                    KeyKind::Primitive,
                    format!("primitive:{}", spelling(&e.lit)),
                )),
                _ => None,
            },
            Expr::Macro(m) if self.sdk(&m.mac.path, "symbol_short") => {
                syn::parse2::<syn::LitStr>(m.mac.tokens.clone())
                    .ok()
                    .map(|s| (KeyKind::Symbol, format!("symbol:{:?}", s.value())))
            }
            Expr::Call(c) => {
                if let Expr::Path(p) = bare(&c.func) {
                    if let Some(k) = self.enum_key(&p.path, &c.args.iter().collect::<Vec<_>>()) {
                        return k;
                    }
                    let Ok(mut path) = syn::parse2::<syn::Path>(p.path.to_token_stream()) else {
                        return self.unknown_key(expr);
                    };
                    let method = path.segments.pop().map(|s| s.ident.to_string());
                    path.segments.pop_punct();
                    if method.as_deref() == Some("new")
                        && c.args.len() == 2
                        && self.is_env(&c.args[0])
                    {
                        if let Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }) = bare(&c.args[1])
                        {
                            if self.sdk(&path, "Symbol") {
                                return StorageKey {
                                    identity: format!("symbol:{:?}", s.value()),
                                    expression: spelling(expr),
                                    kind: KeyKind::Symbol,
                                    dynamic: false,
                                    contract_type: None,
                                };
                            }
                            if self.sdk(&path, "String") {
                                return StorageKey {
                                    identity: format!("sdk-string:{:?}", s.value()),
                                    expression: spelling(expr),
                                    kind: KeyKind::String,
                                    dynamic: false,
                                    contract_type: None,
                                };
                            }
                        }
                    }
                }
                None
            }
            Expr::Path(p) => {
                if let Some(k) = self.enum_key(&p.path, &[]) {
                    return k;
                }
                None
            }
            Expr::Struct(s) => {
                if let Some(id) = self.type_identity(&s.path) {
                    return StorageKey {
                        identity: format!("struct:{id}"),
                        expression: spelling(expr),
                        kind: KeyKind::Unknown,
                        dynamic: true,
                        contract_type: Some(id),
                    };
                }
                None
            }
            _ => None,
        };
        if let Some((kind, identity)) = known {
            StorageKey {
                identity,
                expression: spelling(expr),
                kind,
                dynamic: false,
                contract_type: None,
            }
        } else {
            self.unknown_key(expr)
        }
    }
    fn unknown_key(&self, expr: &Expr) -> StorageKey {
        let contract_type = self
            .variable(expr)
            .and_then(|v| v.type_name.as_ref())
            .and_then(|ty| syn::parse_str::<syn::Path>(ty).ok())
            .and_then(|p| self.type_identity(&p));
        StorageKey {
            identity: format!("unknown:{}:{}", self.identity(""), spelling(expr)),
            expression: spelling(expr),
            kind: KeyKind::Unknown,
            dynamic: true,
            contract_type,
        }
    }
    fn value_type(&self, expr: &Expr) -> Option<String> {
        if let Some(t) = self.variable(expr).and_then(|v| v.type_name.clone()) {
            return Some(t);
        }
        match bare(expr) {
            Expr::Struct(e) => Some(spelling(&e.path)),
            Expr::Cast(e) => Some(spelling(bare_type(&e.ty))),
            Expr::Lit(e) => match &e.lit {
                syn::Lit::Bool(_) => Some("bool".into()),
                syn::Lit::Int(i) if !i.suffix().is_empty() => Some(i.suffix().into()),
                syn::Lit::Str(_) => Some("& str".into()),
                syn::Lit::Char(_) => Some("char".into()),
                _ => None,
            },
            Expr::Tuple(t) if t.elems.is_empty() => Some("()".into()),
            _ => None,
        }
    }
    fn location(
        &mut self,
        span: proc_macro2::Span,
        message: String,
    ) -> Option<doctor_core::Evidence> {
        match evidence(self.source, span, message) {
            Ok(e) => Some(e),
            Err(e) => {
                self.error = Some(e);
                None
            }
        }
    }
    fn uncertainty(&mut self, span: proc_macro2::Span, message: &str) {
        if let Some(e) = self.location(span, message.into()) {
            self.inventory.uncertainties.push(e);
        }
    }
    fn signature(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let parent = std::mem::take(&mut self.locals);
        for arg in &sig.inputs {
            if let syn::FnArg::Typed(arg) = arg {
                if let Pat::Ident(name) = arg.pat.as_ref() {
                    self.locals.insert(
                        name.ident.to_string(),
                        Local {
                            env: self.env_type(&arg.ty),
                            type_name: Some(spelling(bare_type(&arg.ty))),
                            ..Local::default()
                        },
                    );
                }
            }
        }
        self.visit_block(block);
        self.locals = parent;
    }
    fn shadow_pattern(&mut self, pat: &Pat) {
        let mut names = PatternNames::default();
        names.visit_pat(pat);
        for name in names.0 {
            self.locals.insert(name, Local::default());
        }
    }
    fn module_items(&mut self, items: &[Item], definitions: bool) {
        let parent = std::mem::replace(
            &mut self.names,
            bindings(items, &self.source.sdk_crate_names),
        );
        let parent_constants = std::mem::take(&mut self.constants);
        // Local declarations shadow imported SDK names.
        for item in items {
            let ident = match item {
                Item::Struct(s) => Some(&s.ident),
                Item::Enum(e) => Some(&e.ident),
                Item::Type(t) => Some(&t.ident),
                _ => None,
            };
            if let Some(ident) = ident {
                self.names.remove(&ident.to_string());
            }
        }
        if !definitions {
            // Resolve only direct constants, in bounded passes for references to
            // later declarations. Cycles remain unknown, never evaluated.
            for _ in 0..items.len() {
                for item in items {
                    if let Item::Const(c) = item {
                        let key = self.key(&c.expr);
                        self.constants.insert(
                            c.ident.to_string(),
                            Local {
                                type_name: Some(spelling(bare_type(&c.ty))),
                                key: Some(key),
                                ..Local::default()
                            },
                        );
                    }
                }
            }
        }
        for item in items {
            if let Item::Mod(m) = item {
                if let Some((_, items)) = &m.content {
                    self.module.push(m.ident.to_string());
                    self.module_items(items, definitions);
                    self.module.pop();
                }
                continue;
            }
            if definitions {
                let definition = match item {
                    Item::Struct(s)
                        if s.attrs.iter().any(|a| self.sdk(a.path(), "contracttype")) =>
                    {
                        let (shape, fields) = fields(&s.fields);
                        Some((&s.ident, ContractTypeShape::Struct { shape, fields }))
                    }
                    Item::Enum(e) if e.attrs.iter().any(|a| self.sdk(a.path(), "contracttype")) => {
                        Some((
                            &e.ident,
                            ContractTypeShape::Enum {
                                variants: e
                                    .variants
                                    .iter()
                                    .map(|v| {
                                        let (shape, fields) = fields(&v.fields);
                                        StorageVariant {
                                            name: v.ident.to_string(),
                                            fields,
                                            shape,
                                            discriminant: v
                                                .discriminant
                                                .as_ref()
                                                .map(|(_, e)| spelling(e)),
                                        }
                                    })
                                    .collect(),
                            },
                        ))
                    }
                    _ => None,
                };
                if let Some((ident, shape)) = definition {
                    if let Some(evidence) = self.location(ident.span(), "Explicit SDK contracttype declaration; storage association is tracked separately.".into()) {
                        self.inventory.contract_types.push(StorageContractType { identity: self.identity(&ident.to_string()), shape, used_as_key: false, used_as_value: false, evidence });
                    }
                }
            } else if !matches!(item, Item::Use(_)) {
                self.visit_item(item);
            }
        }
        self.names = parent;
        self.constants = parent_constants;
    }
}
impl<'ast> Visit<'ast> for Analyzer<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.signature(&item.sig, &item.block);
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.signature(&item.sig, &item.block);
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let parent = self.locals.clone();
        let names = self.names.clone();
        visit::visit_block(self, block);
        self.locals = parent;
        self.names = names;
    }
    fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
        self.visit_expr(&expr.expr);
        let parent = self.locals.clone();
        self.shadow_pattern(&expr.pat);
        self.visit_block(&expr.body);
        self.locals = parent;
    }
    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        self.visit_expr(&expr.expr);
        for arm in &expr.arms {
            let parent = self.locals.clone();
            self.shadow_pattern(&arm.pat);
            self.visit_pat(&arm.pat);
            self.visit_expr(&arm.body);
            self.locals = parent;
        }
    }
    fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
        self.visit_expr(&expr.expr);
        self.shadow_pattern(&expr.pat);
        self.uncertainty(
            expr.span(),
            "Conditional pattern bindings are conservatively unresolved; review storage aliases.",
        );
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.names.clear();
        self.uncertainty(
            item.span(),
            "Function-local imports are not resolved for SDK attributes/macros.",
        );
    }
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let parent = self.locals.clone();
        for pat in &closure.inputs {
            self.shadow_pattern(pat);
            match pat {
                Pat::Ident(p) => {
                    self.locals.insert(p.ident.to_string(), Local::default());
                }
                Pat::Type(t) => {
                    if let Pat::Ident(p) = t.pat.as_ref() {
                        self.locals.insert(
                            p.ident.to_string(),
                            Local {
                                env: self.env_type(&t.ty),
                                type_name: Some(spelling(bare_type(&t.ty))),
                                ..Local::default()
                            },
                        );
                    }
                }
                _ => self.uncertainty(
                    pat.span(),
                    "Closure pattern cannot be resolved; review captured storage bindings.",
                ),
            }
        }
        self.visit_expr(&closure.body);
        self.locals = parent;
    }
    fn visit_local(&mut self, local: &'ast syn::Local) {
        // Visit initializer before shadowing the outer variable.
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
        }
        let (pat, ty) = match &local.pat {
            Pat::Type(p) => (p.pat.as_ref(), Some(p.ty.as_ref())),
            p => (p, None),
        };
        if let Pat::Ident(p) = pat {
            let mut value = Local::default();
            if let Some(init) = &local.init {
                value.env = self.is_env(&init.expr);
                value.storage = self.is_storage(&init.expr);
                value.durability = self.durability(&init.expr);
                value.type_name = self.value_type(&init.expr);
                value.key = Some(self.key(&init.expr));
            }
            if let Some(ty) = ty {
                value.type_name = Some(spelling(bare_type(ty)));
                value.env = self.env_type(ty);
            }
            self.locals.insert(p.ident.to_string(), value);
        } else {
            self.shadow_pattern(pat);
            self.uncertainty(
                local.span(),
                "Destructured local binding is unresolved; review storage aliases and value types.",
            );
        }
    }
    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        visit::visit_expr_assign(self, assignment);
        if let Expr::Path(p) = bare(&assignment.left) {
            if let Some(name) = p.path.get_ident() {
                self.locals.insert(name.to_string(), Local::default());
            }
        }
        self.uncertainty(
            assignment.span(),
            "Assignment invalidates simple local inference; review subsequent storage keys/types.",
        );
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let operation = call.method.to_string();
        let supported = matches!(
            operation.as_str(),
            "has"
                | "get"
                | "set"
                | "remove"
                | "update"
                | "try_update"
                | "extend_ttl"
                | "extend_ttl_with_limits"
        );
        if let Some(durability) = self.durability(&call.receiver) {
            // Instance TTL is contract-wide and has no key, unlike persistent/temporary.
            if supported
                && !(durability == Durability::Instance && operation.starts_with("extend_ttl"))
            {
                if let Some(expr) = call.args.first() {
                    let mut key = self.key(expr);
                    key.expression = spelling(bare(expr));
                    let generics: Vec<_> = call
                        .turbofish
                        .iter()
                        .flat_map(|g| &g.args)
                        .filter_map(|g| match g {
                            GenericArgument::Type(t) => Some(t),
                            _ => None,
                        })
                        .collect();
                    let ty =
                        if matches!(operation.as_str(), "get" | "set" | "update" | "try_update") {
                            generics
                                .get(1)
                                .filter(|t| !matches!(t, Type::Infer(_)))
                                .map(|t| spelling(bare_type(t)))
                                .or_else(|| {
                                    if operation == "set" {
                                        call.args.get(1).and_then(|e| self.value_type(e))
                                    } else {
                                        None
                                    }
                                })
                        } else {
                            None
                        };
                    if let Some(id) = &key.contract_type {
                        if let Some(t) = self
                            .inventory
                            .contract_types
                            .iter_mut()
                            .find(|t| &t.identity == id)
                        {
                            t.used_as_key = true;
                        }
                    }
                    if let Some(ty) = &ty {
                        if let Ok(parsed) = syn::parse_str::<Type>(ty) {
                            let mut names = TypeNames::default();
                            names.visit_type(&parsed);
                            for path in names.paths {
                                if let Some(id) = syn::parse_str::<syn::Path>(&path)
                                    .ok()
                                    .and_then(|p| self.type_identity(&p))
                                {
                                    if let Some(t) = self
                                        .inventory
                                        .contract_types
                                        .iter_mut()
                                        .find(|t| t.identity == id)
                                    {
                                        t.used_as_value = true;
                                    }
                                } else if !known_value_type(&path) {
                                    self.uncertainty(call.method.span(), &format!("Stored type {path} has no resolved local contracttype definition; cross-file imports, aliases and external encodings require review."));
                                }
                            }
                        }
                    }
                    if let Some(evidence) = self.location(
                        call.method.span(),
                        format!(
                            "{durability:?} storage {operation}; key source: {}",
                            key.expression
                        ),
                    ) {
                        self.inventory.entries.push(StorageEntry {
                            key,
                            durability,
                            value_types: ty.into_iter().collect(),
                            operations: vec![operation],
                            evidence: vec![evidence],
                        });
                    }
                }
            } else if !supported {
                self.uncertainty(
                    call.method.span(),
                    "Storage method is outside the verified inventory method set.",
                );
            }
        } else if supported && storage_like(&call.receiver) {
            self.uncertainty(call.method.span(), "Storage-like receiver could not be bound to an SDK Env; no exact key/type is inferred.");
        }
        visit::visit_expr_method_call(self, call);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if !self.sdk(&mac.path, "symbol_short") {
            self.uncertainty(
                mac.span(),
                "Macro body is not expanded; it may contain storage calls or types.",
            );
        }
    }
}
#[derive(Default)]
struct TypeNames {
    paths: Vec<String>,
}
fn known_value_type(path: &str) -> bool {
    matches!(
        path,
        "bool"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "char"
            | "str"
    ) || matches!(
        path.split_whitespace().next(),
        Some(
            "Option"
                | "Result"
                | "Vec"
                | "Map"
                | "Bytes"
                | "BytesN"
                | "String"
                | "Symbol"
                | "Address"
        )
    )
}
#[derive(Default)]
struct PatternNames(Vec<String>);
impl<'ast> Visit<'ast> for PatternNames {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.push(pat.ident.to_string());
        visit::visit_pat_ident(self, pat);
    }
}
impl<'ast> Visit<'ast> for TypeNames {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        self.paths.push(spelling(&ty.path));
        visit::visit_type_path(self, ty);
    }
}
fn normalize(inv: &mut StorageInventory) {
    inv.entries.sort_by(|a, b| {
        (&a.key.identity, a.durability, &a.key.expression).cmp(&(
            &b.key.identity,
            b.durability,
            &b.key.expression,
        ))
    });
    let mut entries: Vec<StorageEntry> = Vec::new();
    for mut entry in std::mem::take(&mut inv.entries) {
        if let Some(last) = entries.last_mut().filter(|last| {
            last.key.identity == entry.key.identity && last.durability == entry.durability
        }) {
            // Preserve an unknown-value observation, rather than hiding it behind
            // a neighboring typed access. These remain separate entries.
            if last.value_types.is_empty() != entry.value_types.is_empty() {
                entries.push(entry);
                continue;
            }
            last.key.dynamic |= entry.key.dynamic;
            last.value_types.append(&mut entry.value_types);
            last.operations.append(&mut entry.operations);
            last.evidence.append(&mut entry.evidence);
            last.value_types.sort();
            last.value_types.dedup();
            last.operations.sort();
            last.operations.dedup();
            last.evidence.dedup();
        } else {
            entries.push(entry);
        }
    }
    inv.entries = entries;
    inv.contract_types
        .sort_by(|a, b| a.identity.cmp(&b.identity));
    inv.uncertainties
        .sort_by(|a, b| (&a.path, &a.line, &a.message).cmp(&(&b.path, &b.line, &b.message)));
    inv.uncertainties.dedup();
}

/// Inventory explicit calls in one parsed file using verified SDK v28 patterns.
pub fn inventory_storage_text(
    path: &Path,
    text: &str,
    options: &SourceOptions,
) -> Result<StorageInventory, SourceError> {
    options.validate()?;
    let source = ParsedSource::parse(path, text, &options.sdk_crate_names)?;
    let mut analyzer = Analyzer {
        source: &source,
        names: BTreeMap::new(),
        module: vec![],
        locals: BTreeMap::new(),
        constants: BTreeMap::new(),
        inventory: StorageInventory::default(),
        error: None,
    };
    analyzer.module_items(&source.syntax.items, true);
    analyzer.module_items(&source.syntax.items, false);
    if let Some(error) = analyzer.error {
        return Err(error);
    }
    // Propagate likely storage association through explicit contracttype fields.
    // Only same-module spellings are linked; this is not Rust name resolution.
    for _ in 0..analyzer.inventory.contract_types.len() {
        let mut links = Vec::new();
        for ty in &analyzer.inventory.contract_types {
            if !ty.used_as_key && !ty.used_as_value {
                continue;
            }
            let fields: Vec<_> = match &ty.shape {
                ContractTypeShape::Struct { fields, .. } => fields.iter().collect(),
                ContractTypeShape::Enum { variants } => {
                    variants.iter().flat_map(|v| &v.fields).collect()
                }
            };
            let prefix = ty
                .identity
                .rsplit_once("::")
                .map(|(prefix, _)| prefix)
                .unwrap_or("");
            for field in fields {
                if let Ok(parsed) = syn::parse_str::<Type>(&field.type_name) {
                    let mut names = TypeNames::default();
                    names.visit_type(&parsed);
                    for name in names.paths {
                        if let Ok(path) = syn::parse_str::<syn::Path>(&name) {
                            if let Some(name) = path.get_ident() {
                                links.push((
                                    format!("{prefix}::{name}"),
                                    ty.used_as_key,
                                    ty.used_as_value,
                                ));
                            }
                        }
                    }
                }
            }
        }
        for (name, key, value) in links {
            if let Some(ty) = analyzer
                .inventory
                .contract_types
                .iter_mut()
                .find(|ty| ty.identity == name)
            {
                ty.used_as_key |= key;
                ty.used_as_value |= value;
            }
        }
    }
    normalize(&mut analyzer.inventory);
    Ok(analyzer.inventory)
}
/// Read-only source inventory. Uses the source analyzer's exclusions and avoids
/// symlinks; relative paths keep identities comparable across different roots.
pub fn inventory_storage_directory(
    root: &Path,
    options: &SourceOptions,
) -> Result<StorageInventory, SourceError> {
    options.validate()?;
    let root = fs::canonicalize(root).map_err(|source| SourceError::Io {
        path: root.into(),
        source,
    })?;
    let mut paths = vec![];
    collect(&root, &root, options, &mut paths)?;
    paths.sort();
    let mut inventory = StorageInventory::default();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|source| SourceError::Io {
            path: path.clone(),
            source,
        })?;
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| SourceError::InvalidExclusion(path.clone()))?;
        let mut file = inventory_storage_text(relative, &text, options)?;
        inventory.entries.append(&mut file.entries);
        inventory.contract_types.append(&mut file.contract_types);
        inventory.uncertainties.append(&mut file.uncertainties);
    }
    normalize(&mut inventory);
    Ok(inventory)
}
