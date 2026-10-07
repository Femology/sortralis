# Contributing

This repository currently implements Phase 1 only. Check the existing source and
Cargo manifests before making changes; preserve working functionality and keep
changes scoped to the agreed phase.

Use Rust/Cargo 1.96.0 with rustfmt and Clippy to match the verified environment.
From the repository root, run before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p doctor-cli -- --help
```

If formatting fails, run `cargo fmt --all`, inspect the diff, and rerun the checks.
Describe the actual commands and results in the PR, including any limitations.
Add meaningful tests with behavior changes. Tests may unwrap expected successes;
production code must use typed errors. Do not weaken tests to hide a failure.

Verify dependency versions and Stellar/Soroban APIs against authoritative sources
before using them. Do not add invented protocol rules, placeholder success paths,
or claims that this tool proves upgrade safety.

Stage specific files rather than using `git add .`. When committing, use
conventional commit messages and preserve history.
