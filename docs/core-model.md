# Core model — Phase 2

Sortralis retains the planned `doctor-*` crate names. `doctor-core` owns:

- `RuleId`: validated, case-preserving uppercase ASCII identifier. Segments
  contain letters/digits, are separated by a single underscore, and the ID starts
  with a letter. Invalid IDs are rejected even during JSON deserialization.
- `Finding`: ID, title, severity, category, summary, rationale, evidence,
  recommendation, and reference strings. No detector is implemented.
- `Evidence`: optional source path and one-based line, message, optional command
  record. Unknown paths/lines may be absent; a zero line is invalid.
- `CommandResult`: program, separate arguments, working directory, captured
  output, elapsed milliseconds, and tagged outcome. Outcomes distinguish exit
  code, timeout, and failure to start. These records do not execute anything.
- `DetectedPackage`: name, manifest path, optional requested SDK requirement.
  No package discovery is implemented.
- `AnalysisResult`: schema version, tool version, repository path, packages,
  findings, commands, and verdict. `REPORT_SCHEMA_VERSION` is `"1.0"`;
  callers populate that version when producing a report.
- `Config`: validated TOML parsing and optional-file loading. Omitted fields
  inherit defaults; unknown fields and unknown enum values are errors.
- `evaluate_policy`: exact severity membership and a conservative review verdict.
- `exit_code`: explicit run-outcome precedence, independent of severity ordering.

Serialized enum names use SCREAMING_SNAKE_CASE. Rule IDs serialize as strings.
Command statuses use a `kind` discriminator. Models contain no ANSI or terminal
layout. Collections supplied by analyzers are preserved; configuration's
`fail_on` uses a sorted set to deduplicate and serialize deterministically.

The CLI still only supports help/version. Core APIs neither infer command success
nor claim that an upgrade is safe. The caller must truthfully distinguish a
completed analysis from input, external execution, or internal failures.

## Verified dependencies

Exact versions and declared Rust requirements were inspected using `cargo info`:

| Package | Version | Declared Rust minimum | Role |
| --- | --- | --- | --- |
| serde | 1.0.229 | 1.56 | Typed serialization |
| toml | 1.1.6+spec-1.1.0 | 1.85 | Configuration parsing |
| serde_json | 1.0.151 | 1.71 | Test serialization round trips |

Sources: [serde](https://docs.rs/serde/1.0.229/serde/),
[toml](https://docs.rs/toml/1.1.6+spec-1.1.0/toml/),
[serde_json](https://docs.rs/serde_json/1.0.151/serde_json/).
The workspace is verified on Rust 1.96.0. No Stellar APIs were introduced.
