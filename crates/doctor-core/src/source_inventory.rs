//! Interface-like source observations, not a compiled Wasm interface.
use crate::{storage::ContractTypeShape, Evidence};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFunction {
    pub identity: String,
    pub owner: String,
    pub signature: String,
    pub attributes: Vec<String>,
    pub evidence: Evidence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceType {
    pub identity: String,
    pub shape: ContractTypeShape,
    pub attributes: Vec<String>,
    pub evidence: Evidence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventField {
    pub name: String,
    pub type_name: String,
    pub attributes: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvent {
    pub identity: String,
    pub attributes: Vec<String>,
    pub shape: String,
    pub fields: Vec<EventField>,
    pub evidence: Evidence,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInventory {
    pub functions: Vec<SourceFunction>,
    pub types: Vec<SourceType>,
    pub events: Vec<SourceEvent>,
    pub uncertainties: Vec<Evidence>,
}
