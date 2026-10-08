# Sortralis

Sortralis is a local-first Rust CLI for analyzing Soroban contract upgrades before deployment.

It is designed to help maintainers detect known migration risks, inspect Cargo/Soroban project structure, run configured verification steps, compare storage and contract interfaces, inspect local Wasm artifacts, and emit CI-friendly reports.

**Status:** pre-alpha. Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

**Documentation:** https://oobayemi.gitbook.io/sortralis-docs/

## What Sortralis currently does

Sortralis currently supports:

- Cargo workspace and Soroban contract candidate discovery;
- Rust/Soroban source analysis with stable rule IDs and source evidence;
- Protocol 28 migration checks backed by fixtures and documented evidence;
- environment detection for Rust, Cargo, and Stellar CLI;
- configured test and Stellar contract build execution;
- local Wasm inspection through the Stellar CLI adapter;
- storage/source comparison;
- contract-interface comparison;
- safe committed Git-ref comparison without switching the active worktree;
- terminal, JSON, SARIF 2.1.0, and self-contained HTML reports;
- deterministic end-to-end fixture testing.

Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

## Build

The repository has been verified with Rust/Cargo 1.96.0, edition 2021, and workspace resolver 2.

From the repository root:

```bash
cargo build --workspace --locked
cargo run -p doctor-cli -- --help
cargo run -p doctor-cli -- --version
```

The main binary is `sortralis`. A short `sud` binary alias is also available for supported commands.

## Quick start

Analyze a Soroban project:

```bash
cargo run -p doctor-cli -- check ./path/to/project
```

Discover Cargo packages and Soroban candidates:

```bash
cargo run -p doctor-cli -- scan ./path/to/project
```

Inspect the local tool environment:

```bash
cargo run -p doctor-cli -- scan ./path/to/project --environment
```

Generate a SARIF report:

```bash
cargo run -p doctor-cli -- check ./path/to/project --format sarif --output results.sarif
```

Generate a self-contained HTML report:

```bash
cargo run -p doctor-cli -- check ./path/to/project --format html --output report.html
```

## Commands

- `scan` — discover Cargo packages and Soroban contract candidates; optional environment detection.
- `check` — run source checks and configured repository verification/build steps.
- `doctor` — perform project discovery or explicit ordered verification with `--verify`.
- `wasm` — inspect a local Wasm artifact through the verified Stellar CLI adapter.
- `compare` — compare normalized contract interfaces from Wasm artifacts or project directories.
- `compare-source` — compare storage observations from explicit source snapshots.
- `diff` — compare committed Git refs without switching or modifying the active worktree.
- `explain` — explain a registered rule ID.

Use `--help` on the CLI or an individual subcommand for the current argument surface.

## Reporting

`check` supports a unified report model rendered as:

- terminal;
- JSON;
- SARIF 2.1.0;
- self-contained offline HTML.

The JSON schema is documented in [docs/report-schema.md](docs/report-schema.md).

## Configuration and policy

The optional configuration file is `upgrade-doctor.toml`.

Default policy:

```toml
fail_on = ["BREAKING"]
run_tests = true
build_contracts = true
report_dir = "./doctor-reports"
exclude = ["target", "vendor"]
```

`fail_on` contains exact severities that block the run; it is not a generic “all warnings fail” threshold.

Core exit codes:

| Code | Meaning |
| --- | --- |
| 0 | Completed without configured blocking findings |
| 1 | Findings failed configured policy |
| 2 | Invalid arguments or configuration |
| 3 | Environment or external-tool execution failure |
| 4 | Internal tool/metadata/reporting failure |

## Protocol 28 rules

The current fixture matrix covers, among other cases:

- legacy contract-Wasm update usage;
- legacy deploy API usage;
- sparse-event review;
- invalid/suspicious contract-trait usage;
- internal generated spec symbol references;
- upgrade authorization review;
- storage migration differences;
- contract-interface breaking changes.

Rules are intentionally conservative. When Sortralis cannot prove a condition, it should report review-required/uncertain status instead of claiming safety.

See [docs/source-rules.md](docs/source-rules.md) and [docs/testing/fixture-matrix.md](docs/testing/fixture-matrix.md).

## Quality checks

Before contributing:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p doctor-cli -- --help
```

The GitHub Actions quality workflow runs the corresponding checks on pushes and pull requests.

## Real-world smoke testing

Phase 14 validates Sortralis against public Soroban repositories with different layouts and SDK generations. Results and exact pinned commits are recorded in [docs/testing/real-world-smoke-tests.md](docs/testing/real-world-smoke-tests.md).

Third-party smoke-test clones belong under the ignored `/smoke-tests/` directory and must not be committed into this repository.

## Architecture

| Crate | Responsibility |
| --- | --- |
| `doctor-cli` | CLI command routing and orchestration |
| `doctor-core` | Shared domain, policy, interface, verification, and report models |
| `doctor-cargo` | Cargo workspace/package discovery |
| `doctor-source` | Structured Rust source analysis and migration rules |
| `doctor-wasm` | Wasm/spec inspection and normalized interface comparison |
| `doctor-runner` | Safe external process execution and Stellar CLI adapter |
| `doctor-git` | Safe committed Git snapshot/ref comparison |
| `doctor-report` | Terminal, JSON, SARIF, and HTML rendering |

See [docs/architecture.md](docs/architecture.md) for a shorter architecture overview and the topic-specific files under `docs/` for implementation details.

## Limitations

- Sortralis is pre-alpha.
- It does not perform a security audit, formal verification, or economic analysis.
- It does not automatically modify production contracts or deploy upgrades.
- Source-derived interface analysis is explicitly labeled as an approximation when compiled Wasm specification data is unavailable.
- External build/test behavior depends on the target repository and local toolchain.
- Windows support is not claimed unless separately tested.

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

Licensed under the [MIT License](LICENSE).
