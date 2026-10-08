# Sortralis Report Schema (`v1.0`)

This document defines the machine-readable JSON report schema produced by Sortralis (`sortralis check --format json` and `sortralis doctor --format json`).

---

## Specification Overview

- **Schema Version**: `"1.0"` (represented in field `schema_version`).
- **Encoding**: UTF-8.
- **Field Naming**: Strict `snake_case`.
- **Determinism**: Arrays of findings, packages, and tools are sorted deterministically.
- **Nullability Policy**:
  - Required top-level structures (`schema_version`, `tool_name`, `tool_version`, `repository_path`, `verdict`, `exit_code`, `summary`, `packages`, `tools`, `steps`, `findings`, `command_results`, `disclaimer`) are always present and never omitted.
  - Optional scalar values (e.g. `soroban_sdk_requirement`, `duration_millis`, `exit_code` on skipped steps, source code `line`) use explicit JSON `null` when unknown or not applicable.
  - Arrays are always present; empty collections serialize as `[]` rather than `null`.

---

## Top-Level Schema Fields

| Field | Type | Description |
|---|---|---|
| `schema_version` | `string` | Version of the report schema (currently `"1.0"`). |
| `tool_name` | `string` | Name of the tool (`"sortralis"`). |
| `tool_version` | `string` | SemVer product version (e.g. `"0.1.0-alpha.1"`). |
| `repository_path` | `string` | Absolute or normalized path to the analyzed repository. |
| `verdict` | `string` | High-level upgrade verdict: `"READY_FOR_MANUAL_REVIEW"`, `"CHECKS_COMPLETED_WITH_WARNINGS"`, or `"NOT_READY"`. |
| `exit_code` | `integer` | Process exit status code (`0` for success/warning, `1` for breaking, `2` for invalid input, `3` for tool error). |
| `summary` | `object` | Aggregate counts of findings and verification steps. |
| `packages` | `array` | List of detected workspace Cargo packages and Soroban contract candidates. |
| `tools` | `array` | List of detected host toolchain utilities (`rustc`, `cargo`, `stellar`). |
| `steps` | `array` | Verification steps executed or evaluated (`Tests`, `Build`, etc.). |
| `findings` | `array` | Migration findings produced by static analyzers and verification steps. |
| `command_results` | `array` | Subprocess command execution evidence. |
| `disclaimer` | `string` | Standard non-audit disclaimer. |

---

## Object Definitions

### `ReportSummary`
- `total_findings` (`integer`): Total number of detected findings.
- `breaking_count` (`integer`): Count of findings with severity `BREAKING`.
- `warning_count` (`integer`): Count of findings with severity `WARNING`.
- `manual_review_count` (`integer`): Count of findings with severity `MANUAL_REVIEW`.
- `info_count` (`integer`): Count of findings with severity `INFO`.
- `total_steps` (`integer`): Total number of evaluated verification steps.
- `passed_steps` (`integer`): Count of steps that completed successfully.
- `failed_steps` (`integer`): Count of steps that failed.
- `other_steps` (`integer`): Count of skipped, disabled, or not-applicable steps.

### `DetectedPackage`
- `name` (`string`): Cargo package name.
- `manifest_path` (`string`): Path to package `Cargo.toml`.
- `soroban_sdk_requirement` (`string` or `null`): Declared `soroban-sdk` dependency requirement string.

### `ToolInfo`
- `name` (`string`): Program executable name (`rustc`, `cargo`, `stellar`).
- `version` (`string` or `null`): Parsed SemVer version string if detected.
- `status` (`string`): Detection status (`detected`, `not_installed`, `version_unrecognized`).

### `ReportStep`
- `name` (`string`): Step identifier (e.g. `"Tests"`, `"Build"`).
- `status` (`string`): Human-readable status description.
- `description` (`string` or `null`): Detailed step notes or skip reasons.
- `exit_code` (`integer` or `null`): Subprocess exit code if executed.
- `duration_millis` (`integer` or `null`): Execution time in milliseconds if executed.
- `command` (`string` or `null`): Formatted command line invocation if executed.

### `Finding`
- `id` (`string`): Validated uppercase ASCII rule ID (e.g. `"SOROBAN_STORAGE_KEY_CHANGED"`).
- `title` (`string`): Concise finding headline.
- `severity` (`string`): `"BREAKING"`, `"WARNING"`, `"MANUAL_REVIEW"`, or `"INFO"`.
- `category` (`string`): Finding domain category (`"SOURCE"`, `"STORAGE"`, `"INTERFACE"`, `"EVENT"`, `"AUTH"`, `"BUILD"`, etc.).
- `summary` (`string`): Summary of the specific violation.
- `why_it_matters` (`string`): Explanation of upgrade impact or behavioral change.
- `evidence` (`array`): Location and command evidence objects.
- `recommendation` (`string`): Remediation actions.
- `references` (`array` of `string`): Documentation links.

### `Evidence`
- `path` (`string` or `null`): Source file path relative to repository root.
- `line` (`integer` or `null`): One-based line number (never 0; `null` if file-level).
- `message` (`string`): Concrete code snippet or diagnostic message.
- `command` (`object` or `null`): Subprocess command result if applicable.

---

## Annotated JSON Example

```json
{
  "schema_version": "1.0",
  "tool_name": "sortralis",
  "tool_version": "0.1.0-alpha.1",
  "repository_path": "/workspace/my-contract",
  "verdict": "NOT_READY",
  "exit_code": 1,
  "summary": {
    "total_findings": 1,
    "breaking_count": 1,
    "warning_count": 0,
    "manual_review_count": 0,
    "info_count": 0,
    "total_steps": 2,
    "passed_steps": 1,
    "failed_steps": 0,
    "other_steps": 1
  },
  "packages": [
    {
      "name": "my-contract",
      "manifest_path": "Cargo.toml",
      "soroban_sdk_requirement": "28.0.0"
    }
  ],
  "tools": [
    {
      "name": "rustc",
      "version": "1.96.0",
      "status": "detected"
    },
    {
      "name": "stellar",
      "version": "27.0.0",
      "status": "detected"
    }
  ],
  "steps": [
    {
      "name": "Tests",
      "status": "passed (recorded exit 0)",
      "description": null,
      "exit_code": 0,
      "duration_millis": 120,
      "command": "cargo test"
    },
    {
      "name": "Build",
      "status": "not applicable: no Soroban packages",
      "description": null,
      "exit_code": null,
      "duration_millis": null,
      "command": null
    }
  ],
  "findings": [
    {
      "id": "SOROBAN_STORAGE_KEY_CHANGED",
      "title": "Storage key type or layout changed",
      "severity": "BREAKING",
      "category": "STORAGE",
      "summary": "Storage key enum variant modified",
      "why_it_matters": "Modifying key types alters storage ledger entries.",
      "evidence": [
        {
          "path": "src/lib.rs",
          "line": 42,
          "message": "enum DataKey modified",
          "command": null
        }
      ],
      "recommendation": "Preserve existing enum variant structure or implement migration.",
      "references": [
        "https://developers.stellar.org/docs/smart-contracts"
      ]
    }
  ],
  "command_results": [],
  "disclaimer": "Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy."
}
```
