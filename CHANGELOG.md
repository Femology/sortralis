# Changelog

## Unreleased — 0.1.0-alpha.1

- Add direct command execution with timeouts, raw output bytes, exit statuses,
  captured failures, and Unix process-group cleanup.
- Detect Rust, Cargo, and Stellar CLI versions without compatibility thresholds.
- Add environment-only `scan --environment` and structured missing-tool findings.
- Test literal metacharacter arguments, large output, invalid UTF-8, and timeout
  handling for descendants retaining output pipes.

- Rename the project and CLI to Sortralis.
- Add serializable findings, evidence, package and command records, verdicts,
  and report schema version 1.0 in doctor-core.
- Add validated optional TOML configuration and exact-severity blocking policy.
- Define and test all five core exit codes and explicit severity ordering.

- Establish a seven-crate Rust workspace with edition 2021 and resolver 2.
- Add help/version output and four command names with explicit Phase 1 errors.
- Add CLI integration tests and workspace lint configuration.
- Add a GitHub Actions quality workflow and contributor/security documentation.

Repository scanning, source rule detection, CLI configuration wiring, contract
build/test orchestration, Wasm inspection, comparison, and report generation
remain unimplemented.
