# Cargo discovery (Phase 4)

`doctor_cargo::analyze(path, timeout)` accepts a directory containing Cargo.toml
or the manifest itself. Cargo discovers the workspace and its members; no
`contracts/*` convention is assumed. A missing local manifest is a typed
`NoManifest` error, rather than accidental discovery of an unrelated ancestor.
A workspace member manifest identifies the whole workspace, including virtual
workspaces. Errors retain command arguments, status, raw stdout/stderr, and timing.

The runner executes separate arguments:

```text
cargo metadata --format-version 1 --no-deps --offline --manifest-path <absolute manifest>
```

No builds, tests, dependency resolution, downloads, or repository scripts are
requested. The CLI timeout is 30 seconds. Existing Cargo configuration still
influences metadata; this is not an OS sandbox. Phase 3 runner limitations apply.
Integration tests compare target file names and bytes before/after discovery
and check that Cargo.lock and target are not created in the fixtures.

## Detection evidence

Only workspace member packages are returned. A normal dependency whose Cargo
metadata `name` is `soroban-sdk` marks a likely contract candidate, even if renamed,
optional, target-specific, or lacking a cdylib target. Dev/build-only SDK uses
are excluded. A cdylib alone is insufficient. This intentionally may include SDK
helper libraries; source confirmation belongs to later phases.

Candidate records retain each SDK requirement, alias, optional flag and target,
plus crate types and manifest path evidence. Cargo expands workspace inheritance.
Requirements are Cargo/semver's normalized strings, not verbatim TOML. Multiple
requirements are retained separately, with a sorted unique summary in the core
package model. Evidence has no line number because metadata supplies none; an
inherited requirement's package manifest is evidence of the dependency use,
not a claim that the version literal occurs there. Workspace root is separately
available. `--no-deps` gives no resolved graph, so no resolved SDK version is claimed.

## Verified sources and fixtures

- [Cargo metadata schema](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html):
  format-version 1, workspace_members, packages, dependency name/req/kind/rename,
  target crate_types, absolute manifest paths, and null resolve with --no-deps.
- [cargo_metadata 0.23.1](https://docs.rs/cargo_metadata/0.23.1/cargo_metadata/):
  downloaded source inspected for Metadata::workspace_packages, DependencyKind::Normal,
  Dependency, Target and serde deserialization. Its Rust minimum is 1.86.0;
  this workspace is tested with 1.96.0.
- [Soroban SDK 28.0.0](https://docs.rs/soroban-sdk/28.0.0/soroban_sdk/):
  availability and manifest verified with `cargo info soroban-sdk@28.0.0`.
  No SDK APIs are used in this phase.

`fixtures/healthy-v28` is a virtual workspace with two renamed SDK dependencies;
`workspace-inherited` places its member outside a contracts directory and inherits
SDK 28.0.0 from the workspace. `non-soroban` deliberately has a cdylib but no SDK.
These are **metadata-only** fixtures with minimal library sources. The name
healthy-v28 describes its manifest shape, not build, protocol, or upgrade health.
Contract compilation and SDK transitive dependencies were not validated; tests
need no SDK dependency downloads. No source migration rules or compatibility
thresholds are implemented in Phase 4.
