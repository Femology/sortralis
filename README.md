# Soroban Upgrade Doctor (sortralis)

Pre-alpha CLI for analyzing Soroban contract upgrades and migrating to Protocol 28.

## Problem

Protocol 28 introduces significant architectural changes to Soroban contracts, including updates to deployment APIs, events, authentication, and execution models. These changes make manual upgrades tedious and error-prone. Attempting to deploy legacy code to a Protocol 28 network can result in unexpected failures, security gaps, or broken interfaces. 

## What Doctor Checks

Sortralis performs static analysis and checks for:
- **Legacy API Usage**: Detects deprecated SDK v28 calls like `update_current_contract_wasm`.
- **Protocol 28 Migration Patterns**: Identifies where `ContractExecutable` mechanisms should be used.
- **Event Shape Risks**: Checks for structural changes to events that could break indexing.
- **Authentication Changes**: Inspects explicit and implicit authorization patterns.
- **Storage Mutations**: Flags unsafe or unintended storage state transitions between snapshots.
- **Custom Account Executables**: Prompts review for custom account authorization mechanisms.
- **Removed Export Arguments**: Identifies functions relying on removed Protocol 28 exports.

## Installation

```bash
cargo install --path .
# or if published
# cargo install sortralis
```

## 60-Second Quick Start

Analyze your Soroban smart contract workspace:

```bash
sortralis check path/to/your/project
```

## Example Output

```text
Workspace: /path/to/project
Summary: 1 breaking, 0 warning, 0 manual review, 0 info; 2 steps (0 passed, 0 failed, 2 other)
Packages: my-contract (SDK: =28.0.0)
Tools: cargo 1.96.0, rustc 1.96.0, stellar 27.0.0
Tests: disabled by configuration
Build: disabled by configuration

Findings:
[BREAKING] P28-API-001: The contract calls update_current_contract_wasm, which is deprecated in SDK v28 in favor of Protocol 28 contract executable migration patterns.
  Location: /path/to/project/src/lib.rs:11
  Evidence: Legacy update_current_contract_wasm call detected; upgrade mechanism changed in SDK v28
  Why it matters: Protocol 28 updates the contract deployment and executable architecture (ContractExecutable). Calling update_current_contract_wasm directly requires migration.
  Remediation: Migrate contract upgrade logic to the supported Protocol 28 ContractExecutable mechanisms.
  Reference: https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs
Command: rustc --version (Exited { code: 0 })
rustc 1.96.0 (ac68faa20 2026-05-25)
...
Verdict: NotReady; exit 1
Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.
```

## Commands

- `doctor`: Discover source packages, or explicitly execute ordered project verification.
- `scan`: Discover Cargo packages and Soroban candidates.
- `wasm`: Inspect a local Wasm through Stellar CLI, with no network artifact lookup.
- `compare`: Compare contract interfaces between two Wasm artifacts or project directories.
- `diff`: Compare committed Git source snapshots without switching the active worktree.
- `compare-source`: Compare storage observations in explicit before/after source snapshots.
- `check`: Run source checks, configured repository tests, and Stellar contract builds.
- `explain`: Explain a registered source migration rule.

## Exit Codes

- `0`: Success / Ready
- `1`: NotReady (Breaking changes or warnings found)
- `2`: Path or manifest error (Usage error)
- `3`: Generic execution error
- `4`: Serialization or metadata parsing error

## Supported Versions

- **Soroban SDK**: `v28.0.0`
- **Rust/Cargo**: Compatible with latest stable releases

## Limitations

- Passing Sortralis is **not a security audit** and does not guarantee that an upgrade is safe to deploy.
- Sortralis does not perform formal verification or dynamic symbolic execution.
- Windows environments are not officially tested or supported at this time.
- Does not automatically migrate or rewrite your source code.

## CI/SARIF Integration

Sortralis supports unified terminal, JSON, HTML, and SARIF output formats for seamless CI/CD integration:

```bash
sortralis check --format sarif --output results.sarif .
```

You can then upload `results.sarif` to GitHub Advanced Security or other SARIF-compatible scanning tools.

## Architecture Summary

Sortralis is built with a modular crate architecture:
- `doctor-core`: Core primitives, data structures, and the reporting schema.
- `doctor-source`: Source-level Rust parsing (using `syn`) and rule evaluation.
- `doctor-wasm`: Inspects compiled Wasm artifacts.
- `doctor-runner`: Executes shell commands and orchestrates tests/builds.
- `doctor-cli`: Command-line interface and subcommands.

Read more in [Architecture](docs/architecture.md).

## Contributing

We welcome contributions! Please review our [Contributing Guide](CONTRIBUTING.md) to understand how to run tests, add rules, and structure PRs.

## Security

Please read our [Security Policy](SECURITY.md) for details on supported versions and how to report vulnerabilities.

## License

MIT License. See [LICENSE](LICENSE) for more information.

## Roadmap

- [ ] Interactive migration hints (fix suggestions)
- [ ] Direct IDE integration via LSP
- [ ] Automated git branch management for test setups
- [ ] Network-based state comparison for storage rules
