# Fixtures

- `healthy-v28`: virtual workspace with two real SDK 28.0.0 contract libraries,
  renamed SDK dependencies, and passing arithmetic tests.
- `failing-tests`: SDK 28.0.0 contract with an intentionally failing test; contract
  build should still succeed. Do not change the failure to make this fixture pass.
- `workspace-inherited`: metadata-only nested member inheriting SDK 28.0.0.
- `non-soroban`: metadata-only cdylib Rust library without SDK dependencies.
- `source`: parser-only positive/negative migration patterns.
- `storage`: parser-only before/after storage snapshots. The baseline is
  `identical`; each other directory changes one schema observation, including
  a deliberately unrelated contracttype. See docs/storage-analysis.md.

Discovery tests remain offline metadata-only and verify no target modifications.
Phase 6 opt-in real-tool tests copy healthy-v28/failing-tests to temporary directories
before running commands, leaving tracked fixture directories unchanged.
The release overflow-checks setting was verified from the installed Stellar CLI's
actual diagnostic. SDK macro crate-name requirements were verified by real compilation.
See docs/check-pipeline.md for commands and distinction between real and doubled tests.
