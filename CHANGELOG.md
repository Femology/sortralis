# Changelog

## Unreleased — 0.1.0-alpha.1

- Rename the project and CLI to Sortralis.
- Add serializable findings, evidence, package and command records, verdicts,
  and report schema version 1.0 in doctor-core.
- Add validated optional TOML configuration and exact-severity blocking policy.
- Define and test all five core exit codes and explicit severity ordering.

- Establish a seven-crate Rust workspace with edition 2021 and resolver 2.
- Add help/version output and four command names with explicit Phase 1 errors.
- Add CLI integration tests and workspace lint configuration.
- Add a GitHub Actions quality workflow and contributor/security documentation.

Repository scanning, finding detection, CLI configuration wiring, subprocess
execution, Wasm inspection, comparison, and report generation remain unimplemented.
