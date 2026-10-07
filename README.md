# Sortralis

A local-first CLI being built to analyze Soroban contract upgrades before deployment.

**Status: pre-alpha, Phase 2 core model (`0.1.0-alpha.1`).** Only help and version
output are implemented. `scan`, `compare`, `check`, and `explain` are registered,
but each returns an explicit error and exit code 1. They do not analyze a repository,
run its code, or generate reports yet.

Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

## Build and try

Install Rust using rustup, with Cargo, rustfmt, and Clippy. This scaffold is verified
with Rust/Cargo 1.96.0, edition 2021, and workspace resolver 2.
Stellar CLI and the Soroban SDK are not required for Phase 2.

From the repository root:

```bash
cargo build --workspace --locked
cargo run -p doctor-cli -- --help
cargo run -p doctor-cli -- --version
```

The binary is `target/debug/sortralis`. Its four analysis command
names are reserved for later phases; argument and configuration handling for those
commands is not implemented.

## Workspace

| Crate | Responsibility planned for later phases |
| --- | --- |
| doctor-cli | Commands and terminal entry point (the only binary) |
| doctor-core | Shared domain model and analysis coordination |
| doctor-cargo | Cargo project discovery |
| doctor-source | Rust source migration checks |
| doctor-wasm | Wasm interface extraction and comparison |
| doctor-runner | External command execution |
| doctor-report | Terminal, Markdown, and JSON reports |

The `doctor-core` library provides typed models, validated configuration loading, and pure policy evaluation. The other five libraries establish boundaries only. No analyzer or command runner is implemented.
CLI integration tests live in `tests/cli/help.rs` and are explicitly registered
in `crates/doctor-cli/Cargo.toml`. Reserved `fixtures/`, `docs/`, and `scripts/`
directories are retained for later phases.

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p doctor-cli -- --help
```

The GitHub Actions `quality` job in [ci.yml](.github/workflows/ci.yml) runs these
commands on pushes and pull requests. A workflow file is not evidence of a
successful hosted CI run.

Direct dependencies are pinned: clap 4.6.7, serde 1.0.229, and toml 1.1.6.
serde_json 1.0.151 is a test-only dependency. Cargo.lock pins the full graph.
This project has been tested on Rust 1.96.0. See the
[Phase 1 notes](docs/phase-1-notes.md) and [core model](docs/core-model.md).

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
This project is licensed under the [MIT license](LICENSE).

## Core configuration and policy

The optional configuration filename remains `upgrade-doctor.toml` as specified
in the build plan. The core library can read and validate it; CLI loading will be
wired in a later phase. A missing optional file uses these defaults:

```toml
fail_on = ["BREAKING"]
run_tests = true
build_contracts = true
report_dir = "./doctor-reports"
exclude = ["target", "vendor"]
```

Relative report paths will be interpreted relative to the target repository.
Reading this configuration does not create reports, execute tests, or build contracts.
Unknown fields, unknown severities, malformed TOML, empty report paths, and empty
exclusion entries are rejected with typed errors. Unreadable existing files are
errors rather than silent fallback. Severity names are case-sensitive:
`INFO`, `WARNING`, `MANUAL_REVIEW`, and `BREAKING`.

`fail_on` selects exact severities, rather than a threshold. An empty list
intentionally disables finding-based blocking; it does not suppress runtime failures.
Severity sort order is explicitly INFO, WARNING, MANUAL_REVIEW, BREAKING.
Nonblocking concerns still produce `CHECKS_COMPLETED_WITH_WARNINGS`; no concerns
produce `READY_FOR_MANUAL_REVIEW`. Blocking findings produce `NOT_READY`.
These verdicts are data values, not evidence that an analysis has run.

| Core exit code | Meaning |
| --- | --- |
| 0 | Completed without configured blocking findings |
| 1 | Findings failed policy |
| 2 | Invalid arguments or configuration |
| 3 | Environment or tool execution failure |
| 4 | Internal tool error |

Failure outcomes take precedence over finding policy. The current unfinished CLI
commands still return 1; the core policy is not yet connected to an analysis pipeline.
