//! Parser-based migration rules. Parsing is not compilation or macro expansion.
mod rules;
mod storage;
use doctor_core::{Evidence, Finding, InvalidRuleId, RuleId};
pub use rules::{
    registry, CustomAccountRule, ExportArgumentRule, RuleDocumentation, RuleRegistration,
    SourceRule,
};
use std::{
    error::Error,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};
pub use storage::{inventory_storage_directory, inventory_storage_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetContext {
    Sdk28,
    OtherSdk,
    Unknown,
}

pub struct ParsedSource {
    pub path: PathBuf,
    pub syntax: syn::File,
    pub sdk_crate_names: Vec<String>,
}
impl ParsedSource {
    pub fn parse(
        path: &Path,
        source: &str,
        sdk_crate_names: &[String],
    ) -> Result<Self, SourceError> {
        let syntax = syn::parse_file(source).map_err(|source| SourceError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            syntax,
            sdk_crate_names: sdk_crate_names.to_vec(),
        })
    }
}
#[derive(Debug, Clone)]
pub struct SourceOptions {
    /// Relative path prefixes; a single component excludes that name anywhere.
    pub exclude: Vec<PathBuf>,
    /// Rust crate identifiers, including Cargo dependency aliases if used.
    pub sdk_crate_names: Vec<String>,
}
impl Default for SourceOptions {
    fn default() -> Self {
        Self {
            exclude: Vec::new(),
            sdk_crate_names: vec!["soroban_sdk".into()],
        }
    }
}
impl SourceOptions {
    pub fn validate(&self) -> Result<(), SourceError> {
        for path in &self.exclude {
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(SourceError::InvalidExclusion(path.clone()));
            }
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum SourceError {
    Io { path: PathBuf, source: io::Error },
    Parse { path: PathBuf, source: syn::Error },
    InvalidExclusion(PathBuf),
    InvalidRuleId(InvalidRuleId),
    InvalidLocation { path: PathBuf, line: usize },
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Parse { path, source } => write!(
                f,
                "cannot parse {}:{}: {source}",
                path.display(),
                source.span().start().line
            ),
            Self::InvalidExclusion(path) => write!(
                f,
                "exclusion must be a nonempty relative path without parent traversal: {}",
                path.display()
            ),
            Self::InvalidRuleId(source) => write!(f, "{source}"),
            Self::InvalidLocation { path, line } => write!(
                f,
                "source location cannot be represented: {}:{line}",
                path.display()
            ),
        }
    }
}
impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::InvalidRuleId(source) => Some(source),
            _ => None,
        }
    }
}
pub(crate) fn evidence(
    source: &ParsedSource,
    span: proc_macro2::Span,
    message: String,
) -> Result<Evidence, SourceError> {
    let line = span.start().line;
    let line = u32::try_from(line)
        .ok()
        .and_then(std::num::NonZeroU32::new)
        .ok_or_else(|| SourceError::InvalidLocation {
            path: source.path.clone(),
            line,
        })?;
    Ok(Evidence {
        path: Some(source.path.clone()),
        line: Some(line),
        message,
        command: None,
    })
}
fn findings(sources: &[ParsedSource], context: TargetContext) -> Result<Vec<Finding>, SourceError> {
    let mut findings = Vec::new();
    for registration in registry() {
        let Some(rule) = registration.analyzer else {
            continue;
        };
        let doc = rule.documentation();
        let mut evidence = Vec::new();
        for source in sources {
            evidence.extend(rule.analyze(source, context)?);
        }
        evidence
            .sort_by(|a, b| (&a.path, &a.line, &a.message).cmp(&(&b.path, &b.line, &b.message)));
        evidence.dedup();
        if !evidence.is_empty() {
            findings.push(Finding {
                id: RuleId::new(doc.id).map_err(SourceError::InvalidRuleId)?,
                title: doc.title.into(),
                severity: doc.severity,
                category: doc.category,
                summary: doc.description.into(),
                why_it_matters: doc.why_it_matters.into(),
                evidence,
                recommendation: doc.recommendation.into(),
                references: doc
                    .references
                    .iter()
                    .map(|reference| (*reference).into())
                    .collect(),
            });
        }
    }
    Ok(findings)
}
pub fn analyze_text(
    path: &Path,
    text: &str,
    context: TargetContext,
) -> Result<Vec<Finding>, SourceError> {
    let source = ParsedSource::parse(path, text, &SourceOptions::default().sdk_crate_names)?;
    findings(&[source], context)
}
/// Analyze a package directory without following symlinks or writing files.
/// Source errors are explicit; an unreadable or malformed file is never silently
/// counted as a successful analysis. Findings are grouped once per rule.
pub fn analyze_directory(
    root: &Path,
    context: TargetContext,
    options: &SourceOptions,
) -> Result<Vec<Finding>, SourceError> {
    options.validate()?;
    let root = fs::canonicalize(root).map_err(|source| SourceError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut paths = Vec::new();
    collect(&root, &root, options, &mut paths)?;
    paths.sort();
    let mut sources = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|source| SourceError::Io {
            path: path.clone(),
            source,
        })?;
        sources.push(ParsedSource::parse(&path, &text, &options.sdk_crate_names)?);
    }
    findings(&sources, context)
}
fn collect(
    root: &Path,
    directory: &Path,
    options: &SourceOptions,
    paths: &mut Vec<PathBuf>,
) -> Result<(), SourceError> {
    let entries = fs::read_dir(directory).map_err(|source| SourceError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| SourceError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| SourceError::InvalidExclusion(path.clone()))?;
        let defaults = ["target", "vendor", "generated", ".git", "node_modules"];
        if relative
            .components()
            .any(|part| defaults.iter().any(|name| part.as_os_str() == *name))
            || options.exclude.iter().any(|exclude| {
                if exclude.components().count() == 1 {
                    relative
                        .components()
                        .any(|part| part.as_os_str() == exclude.as_os_str())
                } else {
                    relative.starts_with(exclude)
                }
            })
        {
            continue;
        }
        let kind = entry.file_type().map_err(|source| SourceError::Io {
            path: path.clone(),
            source,
        })?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect(root, &path, options, paths)?;
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
    Ok(())
}
