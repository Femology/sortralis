# Changelog

## Phase 15 — documentation and contributor readiness

- Publish the Sortralis GitBook documentation site with getting-started, CLI, upgrade-analysis, Protocol 28, reports/CI, architecture, testing/evidence, contributor, security, and reference sections.
- Add structured GitHub bug and rule-request issue forms plus a pull request template.
- Cross-link the public documentation from the repository and align contributor guidance with the evidence-first, additive-history workflow.

## Phase 14 — real-world repository validation

- Add pinned source-only smoke tests for Stellar example repositories and OpenZeppelin Stellar Contracts.
- Fix a confirmed Protocol 28 deploy false positive by narrowing detection to deprecated `deploy_v2`.
- Recognize verified OpenZeppelin authorization macros while preserving manual review for unresolved caller/helper authorization.
- Keep real-world smoke evidence as an automated GitHub Actions regression gate.

## Phase 13 — end-to-end fixture matrix

- Add deterministic healthy/broken project fixtures covering Protocol 28 rules, auth, storage, interface changes, malformed input, comments/string false-positive protection, and multi-contract ordering.

## Phase 12 — unified reporting

- Add one normalized report model rendered to terminal, JSON, SARIF 2.1.0, and self-contained HTML.
- Add report-schema documentation and HTML escaping regression tests.

## Phase 11 — contract interface comparison

- Add normalized function/type/error/event models and interface diff classification.
- Prefer compiled Wasm contract specification data, with explicitly labeled source approximation fallback.
- Add `sortralis compare` with breaking-change exit semantics.


## Phase 10 — explicit verification runner

- Add `sud doctor PATH --verify [--json]` with ordered, structured command steps.
- Keep target default features/lint policy, with explicit Clippy/configuration skips.
- Bound pipe capture and redact likely secrets in command/output/finding evidence.
- Build each contract into a fresh private output directory; skip dependent inspection on failure.
- Add multi-contract, failure, output/redaction and real P28/compiler-failure gates.

## Phase 9 — Stellar adapter and local Wasm inspection

- Centralize verified Stellar version/build/local-info command construction.
- Add `sud wasm PATH [--json]` with captured evidence and typed failures.
- Preserve interface/meta/env-meta JSON and inspect the CLI-reported SHA-256 hash.
- Skip remote build attestations under local-only network policy.
- Record current CLI help/output and add injected-runner, missing-tool and real smoke tests.

## Phase 8 — safe Git diff

- Add `doctor-git` object snapshots, validated refs, raw dirty detection and explicit cleanup.
- Add `sud diff --from --to --repo [--json]` without switching the active worktree.
- Compare SDK requirements/lock records, explicit source interfaces/events/types,
  storage risks and rule findings with deterministic evidence.
- Add real Git/CLI gates preserving branch, HEAD, index and dirty sentinel bytes.


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
