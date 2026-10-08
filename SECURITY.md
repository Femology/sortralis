# Security Policy

## Supported Versions

Currently, Sortralis is in a pre-alpha state. Security updates will only be applied to the latest `main` branch.

| Version | Supported          |
| ------- | ------------------ |
| v0.1.x  | :white_check_mark: |
| Pre-release | :white_check_mark: |

## Scope

The scope of this policy covers vulnerabilities within the Sortralis CLI and its crates (`doctor-*`).

Sortralis is a static analysis tool designed to aid developers in migrating to Protocol 28. Sortralis analyzes *your* source code and *your* Wasm binaries. If the tool fails to detect a migration issue, this is considered a bug or a missing feature, not a vulnerability in Sortralis itself.

## Explicit Unaudited Disclaimer

**Sortralis is NOT a security audit tool and passing its checks does NOT prove that an upgrade is safe to deploy.** 

The tool performs AST-level source inspection and basic Wasm scanning, which is inherently limited. It does not perform symbolic execution, formal verification, or dynamic runtime testing. It may produce false positives and false negatives. 

Always conduct comprehensive testing, independent security reviews, and manual inspection before deploying upgrades on a Soroban network.

## Reporting a Vulnerability

If you discover a vulnerability in the Sortralis tool itself (e.g., a vulnerability in how the tool handles malicious input files or dependencies), please reach out to the maintainers. 

*Reporting channel: [SECURITY_CONTACT_PLACEHOLDER]*
