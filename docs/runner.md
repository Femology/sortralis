# Command runner — Phase 3

`doctor-runner::execute` takes a `CommandSpec` containing an OS-native program,
separate OS-native arguments, a working directory, and a nonzero timeout.
It uses `std::process::Command` directly, with null stdin and piped stdout/stderr.
It never constructs a shell command string or automatically runs repository scripts.

The returned `CapturedCommand` retains the exact request and raw stdout/stderr
bytes. Its core `CommandResult` includes program, args, directory, status, text
views, and elapsed milliseconds. Invalid UTF-8 is replaced in text views only;
original bytes are preserved. Capture runs concurrently to avoid full-pipe
deadlocks. Nonzero exit codes remain command results, with actual diagnostics.

Statuses distinguish exited-with-code, signal termination, timeout, failure to
start, and internal execution failure. Missing executables retain the OS error
kind; permissions errors are not mislabeled as missing binaries.
Invalid timeout/directory and pipe/wait/cleanup errors use typed errors.
Execution errors retain captured evidence.

## Timeouts and limitations

The deadline includes waiting for both process exit and output EOF. On Unix the
child starts in a separate process group, which is killed on timeout. The direct
child is reaped. A parent exiting while a descendant holds the pipes still reaches
the timeout. After termination the runner allows up to 100 ms per pipe to drain
before returning the byte snapshot already captured.

On other platforms only the direct child is killed. Descendants that deliberately
escape a Unix group can also survive. Blocking reader threads can remain until
those pipes close or produce another chunk; cancellation prevents continued
buffer growth after returning. This is timeout management, not a process sandbox.
Captured output is buffered in memory without a size cap in this phase.
Only Linux behavior has been verified.

## Environment detection

`detect_environment` requests exactly:

```text
rustc --version
cargo --version
stellar --version
```

The CLI exposes this through `scan --environment`, with five seconds per command.
Full repository scanning is not implemented. Missing tools, command failures, and
unrecognized versions have Toolchain findings with command evidence. All three
tools are attempted when a version command returns a recorded failure.

The parser reads a line naming the requested executable. It does not mistake
`stellar-xdr` or `xdr` for `stellar`. It handles whitespace, CRLF, optional
CLI/version words, a v prefix, and semver prerelease/build suffixes. Successful
stdout is preferred; stderr is also accepted. Nonzero commands are never accepted
as successful detection, even if they printed a plausible version.
No minimum version or compatibility conclusion is encoded.

The real outputs recorded on the development machine are checked into
`crates/doctor-runner/tests/fixtures/`. The Stellar output is:

```text
stellar 27.0.0 (5a7c5fe76530bf4248477ac812fc757146b98cc4)
stellar-xdr 27.0.0 (5262803470be965e42f80023d12fba12808c774a)
xdr (68fa1ac55692f68ad2a2ca549d0a283273554439)
```

These fixtures describe observed version output, not requirements.
Additional parser format tests are explicitly synthetic.

## Verification and dependencies

Execution tests compile a small test-only Rust child fixture using rustc, then
run its success, failure, sleep, output, and descendant modes. It is not a
production binary. The injection test passes metacharacters as one argument,
verifies exact echoed output, and confirms no marker file was created.

Verified package metadata: semver 1.0.28 (Rust minimum 1.68) and nix 0.31.3
(Rust minimum 1.69). nix's safe killpg API was inspected in the downloaded
version's source; no unsafe application code was added.

References:

- [std Command](https://doc.rust-lang.org/std/process/struct.Command.html)
- [Unix process_group](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#method.process_group)
- [nix](https://docs.rs/nix/0.31.3/nix/)
- [semver](https://docs.rs/semver/1.0.28/semver/)
