use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// Stable uppercase ASCII identifier. No normalization or case conversion occurs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RuleId(String);

impl RuleId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidRuleId> {
        let value = value.into();
        let starts_with_letter = value.bytes().next().is_some_and(|b| b.is_ascii_uppercase());
        let valid_segments = value.split(['_', '-']).all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        });
        if starts_with_letter && valid_segments {
            Ok(Self(value))
        } else {
            Err(InvalidRuleId { value })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RuleId {
    type Error = InvalidRuleId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RuleId> for String {
    fn from(id: RuleId) -> Self {
        id.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidRuleId {
    pub value: String,
}

impl fmt::Display for InvalidRuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid rule ID {:?}: use uppercase ASCII segments separated by underscores, starting with a letter", self.value)
    }
}

impl Error for InvalidRuleId {}
