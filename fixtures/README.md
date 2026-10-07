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


`git-diff/before` and `git-diff/after` use verified SDK requirements 27.0.0 and
28.0.0. Tests commit them into temporary Git repositories and exercise the real
`sud` binary using offline, no-dependency Cargo metadata and syntax analysis.
Their build scripts deliberately fail compilation: comparison must never build
them. They cover function addition/removal/signature change, event/type changes,
stored value migration risk, and a newly applicable v28 export-argument finding.
They do not claim to be compilable healthy contracts.
