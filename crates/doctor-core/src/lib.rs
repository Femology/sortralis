//! Sortralis domain models, validated configuration, and pure exit policy.
//! No repository scanning, command execution, or report rendering occurs here.

mod config;
pub mod git_diff;
mod models;
mod policy;
mod rule_id;
pub mod source_inventory;
pub mod storage;

pub use config::{Config, ConfigError, ConfigValidationError, CONFIG_FILE_NAME};
pub use models::*;
pub use policy::{evaluate_policy, exit_code, ExitCode, PolicyDecision, RunOutcome};
pub use rule_id::{InvalidRuleId, RuleId};

/// Version of the serialized report structure, independent of the product version.
pub const REPORT_SCHEMA_VERSION: &str = "1.0";

pub mod interface;
pub mod verification;
