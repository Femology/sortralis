# Stellar Wave repository application brief

## Project

**Sortralis** is a local-first Rust CLI for evidence-based Soroban contract upgrade analysis.

It helps Stellar/Soroban maintainers inspect upgrade risk before deployment by combining Cargo/Soroban discovery, source migration checks, authorization review, storage comparison, committed Git revision comparison, local Wasm inspection, normalized contract-interface comparison, configured verification, and CI-friendly reporting.

Sortralis is intentionally conservative: passing the tool is not a security audit and does not prove an upgrade is safe to deploy.

## Why Sortralis fits the Stellar Wave Program

Sortralis is directly focused on maintainability and upgrade safety in the Soroban ecosystem. It gives contract maintainers a repeatable local workflow for catching known migration risks, reviewing storage/interface changes, and recording evidence before deployment.

The project is written in Rust and is tested against real public Soroban repositories. It is designed for ongoing community maintenance rather than a one-off demo.

## Current release

- Stable release: **v0.1.0**
- License: MIT
- Primary language: Rust
- Public documentation: https://oobayemi.gitbook.io/sortralis-docs/
- Supported release targets:
  - Linux x86_64
  - macOS arm64
  - macOS x86_64
  - Windows x86_64

The release pipeline builds, tests, smoke-tests, packages, and emits SHA-256 checksum sidecars for every supported platform.

## Maintainer readiness

The repository includes:

- contributor documentation;
- security policy;
- pull request template;
- structured issue forms;
- deterministic end-to-end fixtures;
- pinned real-world Soroban smoke tests;
- Linux/macOS/Windows release-readiness CI;
- public GitBook documentation;
- a scoped contributor backlog with acceptance criteria.

## Initial contributor backlog

The first Wave-ready backlog includes:

1. `sortralis init` starter configuration generation.
2. Machine-readable JSON Schema for report schema v1.0.
3. GitHub Actions SARIF/code-scanning integration example.
4. Checksum-verifying release install helpers.
5. A `rules` command for discovering registered rule IDs.
6. Explicit `--no-color` CLI support while preserving `NO_COLOR`.

These issues are intentionally scoped so contributors can complete meaningful work within a short Wave cycle.

## Suggested initial complexity

Final complexity should be assigned in the Drips maintainer dashboard.

- Issue #6 — `--no-color`: **Trivial**
- Issue #3 — SARIF GitHub Actions example: **Trivial**
- Issue #2 — JSON Schema v1.0: **Medium**
- Issue #5 — `rules` command: **Medium**
- Issue #1 — `sortralis init`: **Medium**
- Issue #4 — checksum-verifying install helpers: **High**

Adjust complexity if implementation review shows the scope materially differs.

## Maintainer commitment

During an active Wave, maintainers should review applications promptly, assign contributors early enough for implementation and review, give actionable code-review feedback, and merge only work that satisfies the repository's quality gates.

Required quality gates remain:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

## Application summary

Sortralis is a production-oriented maintainer tool for the Stellar/Soroban ecosystem with a public v0.1.0 release, cross-platform CI, real-world validation, contributor documentation, and a ready technical backlog. The goal of joining Stellar Wave is to grow the contributor base around practical Soroban upgrade tooling and accelerate maintenance work that directly benefits contract developers and maintainers.
