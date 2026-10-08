# v0.1.0 Release Readiness Checklist

This checklist is a release gate, not a claim that Sortralis proves contract safety.

## Product behavior

- [ ] CLI help contains no obsolete phase-only or “unimplemented” claims for shipped features.
- [ ] `sortralis --version` matches the intended release version.
- [ ] `sud --version` matches the intended release version.
- [ ] Core exit-code documentation matches implementation.
- [ ] Report schema version and product version are not conflated.
- [ ] Source approximation is labeled where exact Wasm specification data is unavailable.

## Quality

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features --locked`
- [ ] Linux Release Readiness job green.
- [ ] macOS Release Readiness job green.
- [ ] Windows Release Readiness job green.
- [ ] Phase 14 public-repository smoke workflow green after any source-rule change.

## Packaging

- [ ] Workspace remains `publish = false`.
- [ ] Release workflow tag/version guard verified.
- [ ] Locked release binaries build.
- [ ] Both `sortralis` and `sud` are included in archives.
- [ ] LICENSE and README are included.
- [ ] SHA-256 checksum file is produced for every archive.
- [ ] Linux x86_64 packaging defined.
- [ ] macOS arm64 packaging defined.
- [ ] macOS x86_64 packaging defined.
- [ ] Windows x86_64 packaging defined.
- [ ] Release publication waits for all platform builds.

## Documentation and project metadata

- [ ] README installation/status language is current.
- [ ] GitBook public docs are current.
- [ ] CHANGELOG contains the release summary.
- [ ] SECURITY policy matches release status.
- [ ] CONTRIBUTING quality commands are current.
- [ ] GitHub issue and PR templates are present.
- [ ] No local reference documents or third-party smoke clones are tracked.

## Release

- [ ] Workspace version changed from prerelease to the intended release.
- [ ] `Cargo.lock` workspace package versions match.
- [ ] Version bump is committed normally on `main`.
- [ ] Normal CI is green on the exact release commit.
- [ ] Release Readiness is green on the exact release commit.
- [ ] Annotated tag created without rewriting an existing tag.
- [ ] Tag pushed to origin.
- [ ] GitHub Release workflow succeeds.
- [ ] Published archives/checksums manually sanity-checked.

Do not check the final release section until the actual v0.1.0 version bump and tag are intentionally performed.


## Phase 16 verification evidence

The release-hardening matrix was exercised before any release tag was created.

Final verified GitHub Actions run:

- **Release Readiness:** `37820504361`
- Linux x86_64: passed release build, full workspace tests, `sortralis`/`sud` smoke tests.
- macOS arm64: passed release build, full workspace tests, `sortralis`/`sud` smoke tests.
- macOS x86_64 (`macos-15-intel`): passed release build, full workspace tests, `sortralis`/`sud` smoke tests.
- Windows x86_64: passed release build, full workspace tests, `sortralis.exe`/`sud.exe` smoke tests.

The matrix exposed and fixed real portability assumptions in test timing, macOS canonical temporary paths, synthetic Git fixture staging, and Git-for-Windows path handling.

This evidence establishes **release readiness**, not that v0.1.0 has been released. The workspace remains on its prerelease version until the version bump and tag are intentionally created.
