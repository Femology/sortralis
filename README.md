# Sortralis

A local-first CLI being built to analyze Soroban contract upgrades before deployment.

**Status: pre-alpha, Phase 6 check pipeline (`0.1.0-alpha.1`).** `scan` discovers
Cargo workspaces and likely Soroban contract packages. `scan --environment` detects
Rust, Cargo, and Stellar CLI versions. `check` analyzes source and runs configured
repository tests and Stellar builds, with captured statuses and diagnostics.
`explain` documents registered source rules. Comparison, Wasm inspection, and
report files remain unfinished.

Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

## Build and try

Install Rust using rustup, with Cargo, rustfmt, and Clippy. This scaffold is verified
with Rust/Cargo 1.96.0, edition 2021, and workspace resolver 2.
Stellar CLI and the Soroban SDK are not required for the default workspace checks.
The opt-in real-tool integration tests require both.
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

| Crate | Responsibility |
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
detection. `doctor-cargo` discovers packages from Cargo metadata. `doctor-source`
provides a parser-based rule engine; the remaining two libraries establish boundaries
only. See [runner behavior](docs/runner.md).
CLI integration tests live in `tests/cli/help.rs` and are explicitly registered
in `crates/doctor-cli/Cargo.toml`. Discovery tests use offline fixture metadata;
opt-in check tests compile copies of healthy-v28 and failing-tests. The `scripts/`
directory is reserved for later phases.

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
in the build plan. `check` reads and validates it before running analysis/tasks. A missing optional file uses these defaults:

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

Failure outcomes take precedence over finding policy. `check` applies this policy
after aggregation; missing required tools, timeouts, and incomplete source analysis
exit 3. Completed nonzero tests/builds create Breaking findings and default to exit 1.
`compare` remains unfinished and returns 1.

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
`check` integrates source rules before running tests/builds.
See [source rules and limitations](docs/source-rules.md).

## Check the repository

```bash
cargo run -p doctor-cli -- check ./contract-project
```

`check` validates path/config, detects environment versions, discovers Cargo
packages, analyzes source, runs `cargo test`, runs `stellar contract build`, then
aggregates findings and applies exit policy. Commands run with separate arguments
from the discovered workspace root. Tests/builds can be disabled independently
in `upgrade-doctor.toml`. A member manifest is accepted; the discovered workspace
root configuration wins and is validated before source/tasks execute.

Failures retain raw stdout/stderr. Static findings remain visible after test/build
failures, and a failed test does not prevent the build attempt. Missing/unrecognized
required Stellar tooling blocks its build and exits 3. Disabled or inapplicable
steps are displayed explicitly; no command success is inferred. Default timeouts
are 5 seconds for versions, 30 for metadata, and 600 per test/build command.

SDK v28's verified build minimum is Stellar CLI 25.2.0. The pipeline records this
requirement and its source; meeting it does not establish actual build success.
Broad SDK constraints that cannot establish v28 are flagged for manual review,
with v28 source rules left unapplied. Event-shape review remains manual.

Default CI tests use a command-runner double for versions/test/build outcomes
and real offline Cargo metadata. Two opt-in tests in `crates/doctor-cli/tests/real_tools.rs`
run real Cargo tests and real Stellar builds in copies of the SDK v28 fixtures:

```bash
cargo test -p doctor-cli --test real_tools -- --ignored --test-threads=1 --nocapture
```

These require Stellar CLI, Rust's Wasm target, and SDK dependencies. There is no
silent skip when explicitly invoked. The failing-tests fixture intentionally
returns Cargo exit 101; its integration test expects that failure and a subsequent
successful build. `check` may create lockfiles/build artifacts and run Rust build
scripts and tests through Cargo. It does not independently run repository shell
scripts. No report directory is created yet. See [pipeline details](docs/check-pipeline.md).
