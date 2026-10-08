//! Normalized contract interface models, diff classification, and comparison records.
use serde::{Deserialize, Serialize};
use std::fmt;

pub const INTERFACE_SCHEMA_VERSION: &str = "1.0";
pub const INTERFACE_WASM_SCOPE: &str =
    "Verified Wasm contract specification extracted via Stellar CLI adapter.";
pub const INTERFACE_SOURCE_SCOPE: &str =
    "Source-derived approximation: syntax analysis without compiled Wasm spec, macro expansion, or full type resolution.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InterfaceClassification {
    NonBreaking,
    ReviewRequired,
    Breaking,
}

impl fmt::Display for InterfaceClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonBreaking => write!(f, "NON_BREAKING"),
            Self::ReviewRequired => write!(f, "REVIEW_REQUIRED"),
            Self::Breaking => write!(f, "BREAKING"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisSource {
    WasmContractSpec,
    SourceApproximation,
}

impl fmt::Display for AnalysisSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WasmContractSpec => write!(f, "wasm_contract_spec"),
            Self::SourceApproximation => write!(f, "source_approximation"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedParameter {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedFunction {
    pub name: String,
    pub doc: String,
    pub parameters: Vec<NormalizedParameter>,
    pub return_type: Option<String>,
}

impl NormalizedFunction {
    pub fn signature_display(&self) -> String {
        let params: Vec<_> = self
            .parameters
            .iter()
            .map(|p| format!("{}: {}", p.name, p.type_name))
            .collect();
        let ret = match &self.return_type {
            Some(r) => format!(" -> {r}"),
            None => String::new(),
        };
        format!("fn {}({}){}", self.name, params.join(", "), ret)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedField {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedVariant {
    pub name: String,
    pub discriminant: Option<u32>,
    pub fields: Vec<NormalizedField>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizedTypeKind {
    Struct { fields: Vec<NormalizedField> },
    Enum { variants: Vec<NormalizedVariant> },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedType {
    pub name: String,
    pub doc: String,
    pub kind: NormalizedTypeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedErrorCase {
    pub name: String,
    pub value: u32,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedError {
    pub name: String,
    pub doc: String,
    pub cases: Vec<NormalizedErrorCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub name: String,
    pub doc: String,
    pub topics: Vec<String>,
    pub params: Vec<NormalizedParameter>,
    pub data_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInterface {
    pub analysis_source: AnalysisSource,
    pub scope: String,
    pub functions: Vec<NormalizedFunction>,
    pub types: Vec<NormalizedType>,
    pub errors: Vec<NormalizedError>,
    pub events: Vec<NormalizedEvent>,
}

impl Default for ContractInterface {
    fn default() -> Self {
        Self {
            analysis_source: AnalysisSource::WasmContractSpec,
            scope: INTERFACE_WASM_SCOPE.into(),
            functions: Vec::new(),
            types: Vec::new(),
            errors: Vec::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDiffRecord {
    pub id: String,
    pub subject: String,
    pub classification: InterfaceClassification,
    pub summary: String,
    pub before_evidence: Option<String>,
    pub after_evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDiff {
    pub schema_version: String,
    pub scope: String,
    pub analysis_source: AnalysisSource,
    pub before: ContractInterface,
    pub after: ContractInterface,
    pub records: Vec<InterfaceDiffRecord>,
    pub has_breaking_changes: bool,
}
