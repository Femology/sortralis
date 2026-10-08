# Contributing to Sortralis

Thank you for your interest in contributing to Sortralis! 

## Local Setup

1. Ensure you have Rust and Cargo installed (latest stable is recommended).
2. Clone the repository: `git clone https://github.com/stellar/sortralis.git`
3. Install dependencies and build the workspace: `cargo build`

## Test Commands

To run tests across all crates:
```bash
cargo test
```

For formatting and linting:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Architecture

Sortralis uses a modular crate architecture:
- `doctor-core`: Foundational structs and reporting types.
- `doctor-cli`: The CLI interface parsing and command routing.
- `doctor-source`: Uses `syn` for parsing Rust source code and evaluating AST rules.
- `doctor-wasm`: Extracts information from compiled Soroban WebAssembly binaries.
- `doctor-runner`: Spawns sub-commands and discovers Cargo manifests.

## How to Add a Rule

1. Define a struct implementing the `Rule` trait in `crates/doctor-source/src/rules.rs`.
2. Provide metadata such as the Rule ID (e.g. `P28-API-002`), title, severity, and remediation.
3. Implement `fn analyze(...)` utilizing `syn::visit` to scan the AST for violations.
4. Add the rule to the `registry()` function in `rules.rs` to register it with the engine.

## Mandatory Positive/Negative Fixtures

Every new rule **must** include corresponding fixtures in `fixtures/matrix/`:
- **Positive Fixtures**: Source code that intentionally violates the rule (the tool must catch it).
- **Negative Fixtures**: Healthy source code that mimics the pattern correctly (the tool must pass cleanly, 0 false positives).

Test your fixtures explicitly:
```bash
cargo run --bin sortralis -- check fixtures/matrix/your-new-fixture
```

## How to Add a Report Field Safely

1. Navigate to `crates/doctor-core/src/report.rs`.
2. Add your field to the corresponding struct (e.g., `ReportParams`, `Report`).
3. Ensure backwards compatibility: if modifying SARIF or JSON models, make the new field optional (`Option<T>`) to avoid breaking existing integrations.
4. Update terminal output logic in `crates/doctor-report/src/lib.rs` if the field needs human-readable representation.

## Commit and PR Expectations

- **Commit Messages**: Follow [Conventional Commits](https://www.conventionalcommits.org/). e.g., `feat(rules): add P28-API-002`, `fix(cli): resolve formatting issue`.
- **Additive History**: Do not rewrite shared history. No force pushes to collaborative branches, and no `git commit --amend` once code is shared. Add new commits instead.
- **PR Description**: Detail why the change is made. If a rule is added, link to the Soroban SDK migration guide.
