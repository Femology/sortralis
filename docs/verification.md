# Explicit verification — Phase 10

```bash
sud doctor . --verify
sud doctor ./contract-project --verify --json
sud doctor ./contract-project --verify --skip-clippy
```

Without `--verify`, `doctor` performs the existing read-only Cargo discovery
summary, without tests, Clippy, builds or inspection. Verification explicitly
executes the target project's Cargo test/build workflow, including code and build
scripts normally invoked by those tools. It is not a process sandbox.

## Ordering and project-compatible defaults

Preflight validates the path, obtains offline/no-dependency Cargo metadata and
validates configuration at Cargo's discovered workspace root. Metadata failure
is a structured report with all dependent steps skipped. Invalid paths/missing
manifests/configuration or a zero timeout are invalid-input errors.

The ordered steps then run from that workspace root:

1. `cargo fmt --all -- --check`.
2. `cargo test --workspace`, unless `run_tests=false`.
3. `cargo clippy --workspace --all-targets`, unless explicitly `--skip-clippy`.
4. For each discovered Soroban package with a cdylib target:
   `stellar contract build --package NAME --out-dir PRIVATE_FRESH_DIRECTORY`.
5. After that package builds successfully and the expected regular Wasm exists,
   run the Phase 9 local inspector: CLI version, interface JSON, meta JSON,
   env-meta JSON and hash. Remote build attestations remain skipped.

No command enables `--all-features`, changes default features, adds feature
names, or overrides the target's lint policy. Clippy does not add `-D warnings`;
project-configured deny/forbid levels still apply. Use `--skip-clippy` when the
component or the check is inappropriate, and the report records the omission.
Sortralis's own required checks continue using `--all-features` and `-D warnings`.
Target Cargo configuration/toolchain settings are respected; future customization
of individual feature sets/profiles is outside this phase.

A format/test/Clippy failure does not prevent the other independent checks or
contract builds from providing useful evidence. A package's failed, timed-out or
missing-tool build prevents its inspection, including partial/stale artifacts.
A first package's failure does not prevent another package's independent build.
Success without the expected artifact is itself a failed finding, then inspection
is skipped. Disabled builds (`build_contracts=false`) and non-Soroban/no-cdylib
workspaces explicitly skip contract build/inspection; Rust checks still run.
`fail_on` does not waive verification failures: any failed required command exits
1; verification is distinct from `check`'s finding-based configuration policy.

## Artifact discovery and temporary lifecycle

Artifact output paths come from the verified Stellar CLI 27.0.0 `--out-dir`
convention. Exact official build source copies
`package_name.replace('-', '_') + ".wasm"` into that directory. Each package is
built into a separate new directory inside an OS-generated private `tempfile`
directory. The root is uniquely named `sortralis-verify-*`. No target-directory
layout guessing or filesystem-wide search is used. Missing files, symlink files
or symlink package output directories are rejected. Only artifacts belonging to
a successfully completed invocation are inspected.

Temporary artifact copies are consumed before explicit cleanup on normal success
and target failure. Reported inspection command paths describe those temporary
inputs; the copies are no longer present after verification returns. Cargo and
Stellar's usual workspace/target build outputs and Cargo.lock may be created or
updated by the explicitly requested tests/builds. Doctor does not delete those.
Cleanup failure is a structured execution failure. Drop provides best-effort
fallback; process kill/abort/power loss may leave an unregistered temporary folder.
This is not a sandbox for adversarial project code or concurrent filesystem edits.

## Structured evidence and output limits

Every executed step records its name, sanitized command/argument vector and
escaped display, duration in milliseconds, actual exit code when available,
status, bounded stdout/stderr, truncation flags and generated failure finding.
Skipped steps have no executed command/exit code and explain their dependency or
configuration reason. Nonzero commands retain their actual diagnostics; a finding
never replaces stderr. JSON command stdout/stderr are sanitized text views; raw
unredacted verification output is not persisted in the report.

Both pipes are drained concurrently to EOF even after the retention cap is
reached. Production task capture retains at most 64 KiB per stream. Metadata and
machine-readable inspection commands retain at most 8 MiB per stream so parsing
is not limited to the terminal-display cap. Exceeding that JSON limit fails
clearly. Every displayed/stored stream in verification evidence is at most
64 KiB. Truncation keeps complete prefix lines and emits `[OUTPUT TRUNCATED]`;
this discards a partial trailing token/secret at the cut boundary. No claim is
made that all diagnostics fit in a truncated report.

Redaction recognizes likely sensitive assignments/headers, API/GitHub/JWT token
patterns, Stellar secret-seed-shaped strings, URI credentials and private-key PEM
blocks. Secret flag values are masked before command display. G/C/M public Stellar
identifier patterns and normal network passphrases are preserved. Detection is
heuristic; arbitrary secrets or unusual encodings cannot be guaranteed detectable.
The synthetic fixture uses fake credentials/identifier-shaped strings, not real
keys. Terminal evidence escapes control characters and arguments; displayed
commands are descriptions, never shell interpolation.

Task timeouts default to 600 seconds; metadata and inspection use 30 seconds.
The existing Unix process-group timeout behavior is reused. Only Linux was
verified; the non-Unix fallback kills the direct child only. Cancellation and
cleanup guarantees remain those described in [runner.md](runner.md).

## Status and exits

Statuses: PASSED, FAILED, MISSING_TOOL, TIMED_OUT, EXECUTION_ERROR, SKIPPED.
Successful execution is explicitly recorded, never inferred from a file.

- 0: all configured executed steps completed without failures; skipped checks
  remain visible and are not claimed as passed.
- 1: target verification command, artifact validation or inspection failed.
- 2: invalid path/arguments/configuration/options.
- 3: missing executable, timeout, runner/discovery execution error or cleanup failure.
- 4: internal domain-model/rule-ID error.

Exit 3 takes precedence over completed nonzero command findings. Build failures
are normal report data and exit 1; they do not panic or crash Doctor. An inspection
command that exits 0 with malformed output is followed by a structured parsing
failure. Verified metadata-absent diagnostics retain actual exit 1 and are
explicitly skipped rather than mislabeled as a failed required section.

## Tests and real-tool gate

Default tests inject a process runner and verify passing/failing commands,
missing executables, timeouts/runner errors, fresh multi-contract outputs,
independent continuation, dependency skips, missing artifacts, redaction and
serialization. The runner's real child fixture emits large data on both pipes to
prove bounded retention with real exit status. CLI tests exercise structured
missing-tool output and non-verifying discovery without producing build outputs.

```bash
cargo test -p doctor-cli --test verification
cargo test -p doctor-runner --test redaction
cargo test -p doctor-runner --test execution bounded_capture
cargo test -p doctor-cli --test verification -- --ignored --nocapture
```

The last command is opt-in: it performs real verification of healthy-v28 (two
SDK 28.0.0 contracts) and drives a real compiler failure fixture through the
actual `sud` binary. It requires cached/downloadable SDK dependencies, rustfmt,
Clippy, Stellar CLI and the appropriate Wasm target. Injected tests are never
claimed as real CLI/build execution. Required tools were present for the local
real P28 gate; see the phase verification report for exact command/results.

Verification provides evidence for manual review. It is not a security audit and
never establishes guaranteed deployment safety. No later phase is implemented.
