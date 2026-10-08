//! SARIF 2.1.0 (Static Analysis Results Interchange Format) exporter.
use doctor_core::report::Report;
use doctor_core::Severity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, Write};

pub const SARIF_SCHEMA_URI: &str =
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json";
pub const SARIF_VERSION: &str = "2.1.0";
pub const TOOL_INFORMATION_URI: &str = "https://github.com/Sortralis/sortralis";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    pub short_description: SarifMultiformatMessage,
    pub full_description: SarifMultiformatMessage,
    pub help: SarifMultiformatMessage,
    pub help_uri: String,
    pub default_configuration: SarifRuleConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRuleConfiguration {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifMultiformatMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub rule_index: usize,
    pub level: String,
    pub message: SarifMessage,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRegion {
    pub start_line: u32,
}

fn severity_to_sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Breaking => "error",
        Severity::Warning => "warning",
        Severity::ManualReview => "warning",
        Severity::Info => "note",
    }
}

pub fn build_sarif(report: &Report) -> SarifLog {
    // Collect unique rules deterministically
    let mut rules_map: BTreeMap<String, SarifRule> = BTreeMap::new();

    for finding in &report.findings {
        let rule_id = finding.id.as_str().to_string();
        if !rules_map.contains_key(&rule_id) {
            let help_uri = format!(
                "https://github.com/Sortralis/sortralis/blob/main/docs/rules.md#{}",
                rule_id
            );
            rules_map.insert(
                rule_id.clone(),
                SarifRule {
                    id: rule_id,
                    name: finding.title.clone(),
                    short_description: SarifMultiformatMessage {
                        text: finding.title.clone(),
                    },
                    full_description: SarifMultiformatMessage {
                        text: finding.why_it_matters.clone(),
                    },
                    help: SarifMultiformatMessage {
                        text: finding.recommendation.clone(),
                    },
                    help_uri,
                    default_configuration: SarifRuleConfiguration {
                        level: severity_to_sarif_level(finding.severity).to_string(),
                    },
                },
            );
        }
    }

    let rules: Vec<SarifRule> = rules_map.into_values().collect();
    let rule_indices: BTreeMap<String, usize> = rules
        .iter()
        .enumerate()
        .map(|(idx, r)| (r.id.clone(), idx))
        .collect();

    let mut results = Vec::new();
    for finding in &report.findings {
        let rule_id = finding.id.as_str().to_string();
        let rule_index = rule_indices.get(&rule_id).copied().unwrap_or(0);
        let level = severity_to_sarif_level(finding.severity).to_string();

        let mut locations = Vec::new();
        for ev in &finding.evidence {
            if let Some(path) = &ev.path {
                let rel_path = path
                    .strip_prefix(&report.repository_path)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");

                let region = ev.line.map(|l| SarifRegion {
                    start_line: l.get(),
                });

                locations.push(SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation { uri: rel_path },
                        region,
                    },
                });
            }
        }

        results.push(SarifResult {
            rule_id,
            rule_index,
            level,
            message: SarifMessage {
                text: finding.summary.clone(),
            },
            locations,
        });
    }

    SarifLog {
        schema: SARIF_SCHEMA_URI.into(),
        version: SARIF_VERSION.into(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: report.tool_name.clone(),
                    version: report.tool_version.clone(),
                    information_uri: TOOL_INFORMATION_URI.into(),
                    rules,
                },
            },
            results,
        }],
    }
}

pub fn write_sarif_report(writer: &mut impl Write, report: &Report) -> io::Result<()> {
    let sarif = build_sarif(report);
    serde_json::to_writer_pretty(&mut *writer, &sarif).map_err(io::Error::other)?;
    writeln!(writer)?;
    Ok(())
}
