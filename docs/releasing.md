# Releasing Sortralis

Sortralis releases are distributed as GitHub Release binaries. The workspace is intentionally configured with `publish = false`; Phase 16 does not publish the internal `doctor-*` crates to crates.io.

## Release prerequisites

Before creating a release tag:

1. Ensure `main` is clean and all normal CI jobs are green.
2. Ensure **Release Readiness** is green on Linux, macOS, and Windows.
3. Ensure the Phase 14 pinned real-world source smoke workflow is green when source rules changed.
4. Review `CHANGELOG.md`, `README.md`, GitBook docs, and `SECURITY.md` for the release.
5. Confirm the workspace version in `Cargo.toml` is the exact version you intend to tag.
6. Confirm `Cargo.lock` records the same workspace package version.
7. Confirm:
   - `sortralis --version`
   - `sortralis --help`
   - `sud --version`
   - `sud --help`

## Version and tag contract

The release workflow accepts tags matching `v*`, but it performs a strict guard before building:

```text
Git tag:   v<workspace.package.version>
Cargo:     <workspace.package.version>
```

For example, the final v0.1.0 release requires:

```text
Cargo.toml: 0.1.0
Git tag:    v0.1.0
```

A mismatched tag fails before release assets are built.

## Preparing v0.1.0

Update the workspace version:

```toml
[workspace.package]
version = "0.1.0"
```

Then let Cargo update local workspace package records in `Cargo.lock` using a normal locked/unlocked metadata/build command as appropriate for the version-only change, inspect the diff, and commit the version/changelog update normally.

Do not amend or force-push a shared release-preparation commit.

## Required local gates

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --locked -p doctor-cli --bins
cargo run -p doctor-cli -- --version
cargo run -p doctor-cli -- --help
```

The GitHub **Release Readiness** workflow independently builds and tests on:

- Ubuntu x86_64
- current GitHub-hosted macOS
- Windows x86_64

The tag release additionally packages:

- Linux x86_64
- macOS arm64
- macOS x86_64
- Windows x86_64

## Creating the release

After the version-preparation commit is on `main` and all gates are green:

```bash
git switch main
git pull --ff-only
git tag -a v0.1.0 -m "Sortralis v0.1.0"
git push origin v0.1.0
```

Pushing the tag starts `.github/workflows/release.yml`.

The workflow:

1. validates the tag against the workspace version;
2. builds locked release binaries per platform;
3. smoke-tests `sortralis` and `sud`;
4. packages binaries with `LICENSE` and `README.md`;
5. creates SHA-256 checksum sidecars;
6. collects all platform assets;
7. publishes a GitHub Release only after every build succeeds;
8. uses `gh release create --verify-tag` so release publication cannot silently create an unexpected tag.

## Release assets

Expected archive families:

```text
sortralis-v<VERSION>-linux-x86_64.tar.gz
sortralis-v<VERSION>-macos-arm64.tar.gz
sortralis-v<VERSION>-macos-x86_64.tar.gz
sortralis-v<VERSION>-windows-x86_64.zip
```

Each archive has a matching `.sha256` file.

Users should verify the checksum before extracting an archive.

## Failed release workflow

If a platform build or validation step fails:

- do not move, replace, or force-update the tag;
- fix the problem on `main` with a new normal commit;
- bump to a new prerelease/version if the failed tag has already been publicly distributed;
- create a new tag for the corrected release.

Never force-push or rewrite an already shared release tag.

## Post-release verification

After GitHub reports the release as published:

1. confirm all eight assets exist (four archives + four checksum files);
2. download at least one archive and verify its checksum;
3. run the packaged binary's `--version` and `--help`;
4. verify the GitHub Release version matches the workspace version;
5. verify the public GitBook installation/release guidance is still accurate;
6. record the release in `CHANGELOG.md`.

## Security note

Release binaries are build artifacts, not a security certification. Sortralis remains subject to the limitations documented in `SECURITY.md` and the public docs.
