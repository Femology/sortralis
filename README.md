# Sortralis

A local-first CLI being built to analyze Soroban contract upgrades before deployment.

**Status: pre-alpha, Phase 5 source rules (`0.1.0-alpha.1`).** `scan` discovers
Cargo workspaces and likely Soroban contract packages. `scan --environment` detects
Rust, Cargo, and Stellar CLI versions. Full upgrade analysis, `compare`, and `check`
remain unfinished; `explain <RULE_ID>` documents registered source rules. No contract
builds, repository tests, or reports run automatically.

Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

## Build and try

Install Rust using rustup, with Cargo, rustfmt, and Clippy. This scaffold is verified
with Rust/Cargo 1.96.0, edition 2021, and workspace resolver 2.
Stellar CLI and the Soroban SDK are not required to build or test Sortralis.
Environment detection reports Stellar CLI as not installed when it is absent.

From the repository root:

```bash
cargo build --workspace --locked
cargo run -p doctor-cli -- --help
cargo run -p doctor-cli -- --version
```

The binary is `target/debug/sortralis`. To detect environment versions only:

```bash
cargo run -p doctor-cli -- scan --environment
```

An optional directory can be supplied: `scan ./contract-project --environment`.
This prints tool versions and explicitly states that full repository analysis is
not implemented. It exits 0 when all versions are detected and 3 for missing tools,
execution failures, or unrecognized version output. It applies no version threshold.
To discover repository packages and SDK requirements:

```bash
cargo run -p doctor-cli -- scan ./contract-project
```

Supply a directory containing `Cargo.toml` or a manifest path. A workspace member
is supported: Cargo identifies its workspace root and all members. Discovery does
not recursively search unrelated subdirectories or silently use an ancestor's
manifest when the supplied directory has none. It runs offline metadata with
`--no-deps`, reports **requested** SDK requirements (including inheritance and
aliases), and does not resolve versions, build contracts, or create a lockfile.
Normal SDK dependencies mark candidates; dev/build-only SDK dependencies do not.
Optional and target-specific dependencies are evidence, not proof of an active
contract. A non-Soroban Cargo project completes with no candidates.

Discovery exits 0 on completion, 2 for invalid paths/missing manifests, 3 for
Cargo execution failures (including malformed manifests), and 4 for invalid
metadata output. It has a 30-second timeout. Exit 0 means discovery completed;
it makes no upgrade compatibility claim. See [discovery details](docs/cargo-discovery.md).

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

The `doctor-core` library provides typed models, validated configuration, and pure
policy evaluation. `doctor-runner` executes explicitly requested programs with
separate arguments, timeout handling, concurrent output capture, and environment
detection. `doctor-cargo` discovers packages from Cargo metadata. `doctor-source` provides a parser-based rule engine; the remaining two libraries
establish boundaries only. See [runner behavior](docs/runner.md).
CLI integration tests live in `tests/cli/help.rs` and are explicitly registered
in `crates/doctor-cli/Cargo.toml`. Metadata-only Cargo fixtures live in `fixtures/`;
they are not contract build verification. The `scripts/` directory is reserved for later phases.

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

Direct dependencies are pinned: clap 4.6.7, serde 1.0.229, toml 1.1.6,
semver 1.0.28, cargo_metadata 0.23.1, syn 3.0.6, proc-macro2 1.0.107,
and nix 0.31.3 (Unix only).
serde_json 1.0.151 parses Cargo metadata and is also used in tests. Cargo.lock pins
the full graph.
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

## Source migration rules

```bash
cargo run -p doctor-cli -- explain SDK28_REMOVED_EXPORT_ARGUMENT
cargo run -p doctor-cli -- explain CUSTOM_ACCOUNT_EXECUTABLE_REVIEW
cargo run -p doctor-cli -- explain SDK28_EVENT_SHAPE_REVIEW
```

The source library accepts an explicit SDK v28 target context, parses Rust with
syn, groups findings by stable rule ID, and records exact file/line evidence.
It checks removed export arguments on resolved direct SDK attribute imports and
flags implemented `__check_auth` methods for manual review. Event-shape review
is documentation-only, with no automatic finding or claim of a completed check.
Unknown or missing explain IDs exit 2. `scan` continues to perform Cargo discovery;
source checks are exposed through the library pending later pipeline integration.
See [source rules and limitations](docs/source-rules.md).
