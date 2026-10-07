# Changelog

## Unreleased — 0.1.0-alpha.1

- Add normalized source storage inventories and conservative before/after diffs,
  with key/durability/value/contracttype evidence and explicit uncertainty.
- Add compare-source for human-readable or JSON storage diff records, parser-only
  fixtures and a conservative classification gate. No exact ledger schema claim.

- Implement check configuration/environment/Cargo/source/test/build aggregation
  and exit policy, preserving raw diagnostics and explicit command results.
- Centralize the verified SDK v28 Stellar CLI build requirement.
- Add an injectable runner for environment/discovery and orchestration tests.
- Upgrade healthy-v28 to compilable contract sources, add failing-tests, and
  provide separate opt-in real Cargo/Stellar integration tests.
- Resolve SDK extern-crate aliases during source rule import recognition.

- Add parser-based SDK v28 export-argument and custom-account review rules with
  grouped findings, stable IDs, and precise file/line evidence.
- Register an explicit manual-only event-shape review and implement `explain`.
- Add read-only source traversal with default/custom exclusions, symlink skipping,
  and typed parse/read errors; verify positive/negative source fixtures.

- Discover Cargo workspaces and likely Soroban contract packages with offline,
  non-resolving metadata, inherited SDK requirements, and path evidence.
- Add real Cargo discovery summaries to `scan`, retaining environment-only mode.
- Add metadata-only SDK 28.0.0 fixtures and read-only/error-path tests.

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

Wasm inspection, comparison, expanded source rules, and report generation remain
unimplemented.
