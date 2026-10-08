# Contributing to Sortralis

Sortralis is a pre-alpha local-first CLI for Soroban upgrade analysis. Keep changes narrowly scoped, preserve working behavior, and verify Stellar/Soroban-specific assumptions before implementing them.

## Local setup

Clone the repository:

```bash
git clone https://github.com/Femology/sortralis.git
cd sortralis
```

Use the repository's verified Rust toolchain and lockfile rather than assuming compatibility with every future stable release.

## Required quality checks

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p doctor-cli -- --help
```

If formatting fails, run `cargo fmt --all`, inspect the diff, and rerun the checks.

Do not claim that tests passed unless you actually ran them.

## Architecture

The workspace is split by responsibility:

- `doctor-core`: shared domain, policy, interface, verification, and report models;
- `doctor-cli`: CLI routing/orchestration;
- `doctor-cargo`: Cargo workspace discovery;
- `doctor-source`: structured Rust parsing and source rules;
- `doctor-wasm`: Wasm/spec inspection and interface comparison;
- `doctor-runner`: safe subprocess execution and Stellar CLI integration;
- `doctor-git`: safe committed-ref comparison;
- `doctor-report`: terminal, JSON, SARIF, and HTML rendering.

## Adding or changing a rule

Before implementing a Stellar/Soroban-specific rule:

1. verify the behavior against authoritative Stellar documentation, exact crate source/docs, or the installed verified CLI;
2. give the rule a stable machine-readable ID;
3. provide concrete evidence and remediation;
4. add positive and negative tests/fixtures;
5. ensure comments and string literals do not create false positives;
6. use manual-review semantics when the analyzer cannot prove a break.

Do not invent protocol behavior because it “sounds risky.”

## Fixtures

Every detection rule should have meaningful positive and negative coverage.

The end-to-end matrix lives in `fixtures/matrix/`; expected commands, rule IDs, and exit codes are documented in [docs/testing/fixture-matrix.md](docs/testing/fixture-matrix.md).

Do not weaken a legitimate failing test simply to make CI green.

## Process and command safety

Production code should:

- use typed errors;
- avoid `todo!()` and `unimplemented!()` in production paths;
- avoid shell command construction by string concatenation;
- pass subprocess arguments separately;
- preserve raw external failure evidence;
- avoid destructive Git operations against analyzed repositories.

Tests may use `unwrap()` where the expected success is explicit and local.

## Git discipline

- Stage only files belonging to the current logical change.
- Do **not** use `git add .`.
- Use conventional commit messages.
- Keep shared history additive.
- Do not amend already-shared commits.
- Do not force-push collaborative branches.
- If a pushed change needs a fix, add a new normal commit.

## Documentation and claims

Keep README/help text aligned with implemented behavior.

Never claim that Sortralis proves an upgrade is secure or safe to deploy. Passing Sortralis is not a security audit.
