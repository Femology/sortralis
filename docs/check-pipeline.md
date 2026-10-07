# Check pipeline (Phase 6)

`check [directory-or-Cargo.toml]` validates path and optional config, detects
rustc/cargo/stellar versions, obtains offline Cargo metadata, analyzes source,
runs configured tests, runs configured builds, groups findings, then applies
core policy. Reporting files, Wasm inspection, and comparison are not part of this phase.

Configuration is upgrade-doctor.toml beside the supplied manifest. If Cargo locates
a different workspace root, its configuration is loaded/revalidated before any
source analysis or test/build execution. Root configuration takes precedence;
member invocation does not guess workspace layout or run tasks in the member directory.

The commands are exactly `cargo test` and `stellar contract build`, with separate
arguments, null stdin, captured streams, and the authoritative workspace root.
They inherit Cargo's package/default-member behavior. Static exclusions apply to
source paths, not to package selection by the test/build commands. Commands may
create Cargo.lock/target artifacts, download dependencies, and execute Rust build
scripts/test code through Cargo. No arbitrary repository shell scripts are invoked.

## Verified build requirement

Installed Stellar CLI 27.0.0's `stellar contract build --help` confirms syntax and
workspace cdylib behavior. The exact SDK 28.0.0 migration guide says Stellar CLI
25.2.0 or newer is required for v28 Wasm builds:
https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs
The official release explanation independently confirms it:
https://stellar.org/blog/developers/soroban-rust-sdk-v28

SDK28_MIN_STELLAR_CLI centralizes the minimum; SDK28_BUILD_REFERENCE is included
in SDK28_STELLAR_BUILD_REQUIREMENT findings with captured CLI version evidence.
The comparison is inclusive; a 25.2.0 prerelease is conservatively below 25.2.0.
Passing the version requirement is not a build result or deployment guarantee.

Source context is established only from single major-constraining SDK requirements
(exact, caret, tilde, wildcard major). Broad/path-only/mixed requirements yield
SDK_CONTEXT_UNRESOLVED ManualReview instead of guessed SDK v28 analysis. No resolved
SDK version is claimed. A build still records what the actual tool returns.

## Evidence and outcomes

Every successful version/metadata/test/build command is explicitly recorded. Exact
bytes and OS arguments are retained in CheckResult.captures; serializable text
views use the runner's UTF-8 replacement convention. CLI writes captured test/build
stdout and all stderr bytes directly, never replacing failure diagnostics with a
generic message. Successful metadata JSON is retained but not dumped to the terminal.

Completed nonzero tests create TEST_FAILED (Test, Breaking); completed nonzero
builds create BUILD_FAILED (Build, Breaking). Both continue aggregation, and tests
failing do not suppress a build or source findings. Source findings are grouped
once per ID across packages. Source read/parse failures are visible ManualReview
findings and mark analysis incomplete (exit 3), while tasks can still run.

Default fail_on=[BREAKING] makes completed failed commands exit 1. An explicit
empty policy can allow exit 0 with warnings while failed statuses remain visible.
Missing/unrecognized required tooling, incompatible SDK v28 build tooling, failed
spawn, signal termination, timeout, and execution failures take precedence as exit 3.
Invalid paths/config/exclusions/timeouts exit 2; internal metadata/rule-ID errors
exit 4. No Soroban packages means explicit not-applicable tasks. Disabled tasks
are separate from success. No skipped/blocked command is fabricated in the command list.

Default timeouts: versions 5 s, metadata 30 s, each test/build 600 s. Library callers
can supply CheckTimeouts. Existing runner platform/output-buffer limitations apply;
this is not an OS sandbox or a security audit.

## Test classes

`tests/pipeline.rs` uses a clearly named command-runner Double for environment,
test and build outcomes. Metadata executes real offline Cargo. It does not run
real Stellar builds. Tests verify order, workspace directory, successful records,
both failure categories, raw stderr, surviving source findings, grouping, disabled
steps, config validation, tool incompatibility/missing tools, timeout and policy.
CLI default tests preserve help/explain/scan and check no-Soroban/input behavior.

`tests/real_tools.rs` has two explicitly ignored, opt-in tests, both using real
SystemRunner with no test double:

```bash
cargo test -p doctor-cli --test real_tools -- --ignored --test-threads=1 --nocapture
```

The healthy test requires real Cargo tests and Stellar build to exit 0. The failing
fixture test requires real Cargo exit 101, preserved intentional assertion output,
and a subsequent real Stellar build exit 0; it also invokes the real CLI to verify
exit 1 and visible raw failure diagnostics. Both copy fixtures before commands.
Explicit invocation fails if prerequisites are unavailable; it never silently skips.
Default CI does not install Stellar or compile these fixtures and leaves these two
tests ignored. Verification reports distinguish normal tests from opt-in results.
