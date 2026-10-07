# Metadata-only fixtures

- `healthy-v28`: virtual workspace, two SDK 28.0.0 contract candidates with aliases.
- `workspace-inherited`: nested member inheriting SDK 28.0.0.
- `non-soroban`: cdylib Rust library without an SDK dependency.

SDK 28.0.0 was verified from its published crate. These fixtures exercise Cargo
metadata only, not contract builds. See `docs/cargo-discovery.md` for limitations.
