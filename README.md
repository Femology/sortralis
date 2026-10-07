# Soroban Upgrade Doctor

A local-first CLI being built to analyze Soroban contract upgrades before deployment.

**Status: pre-alpha, Phase 1 scaffold (`0.1.0-alpha.1`).** Only help and version
output are implemented. `scan`, `compare`, `check`, and `explain` are registered,
but each returns an explicit error and exit code 1. They do not analyze a repository,
run its code, or generate reports yet.

Passing Soroban Upgrade Doctor is not a security audit and does not prove that an upgrade is safe to deploy.

## Build and try

Install Rust using rustup, with Cargo, rustfmt, and Clippy. This scaffold is verified
with Rust/Cargo 1.96.0, edition 2021, and workspace resolver 2.
Stellar CLI and the Soroban SDK are not required for Phase 1.

From the repository root:

```bash
cargo build --workspace --locked
cargo run -p doctor-cli -- --help
cargo run -p doctor-cli -- --version
```

The binary is `target/debug/soroban-upgrade-doctor`. Its four analysis command
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

The six library crates establish boundaries only and expose no analyzer API.
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

The sole direct external dependency is clap, pinned to 4.6.7 with a committed
Cargo.lock. Its package metadata declares Rust 1.85 as the minimum; this project
has been tested only on Rust 1.96.0. See [dependency verification](docs/phase-1-notes.md).

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
This project is licensed under the [MIT license](LICENSE).
