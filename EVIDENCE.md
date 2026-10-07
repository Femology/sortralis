# Implementation evidence

## Phase 9: Stellar CLI adapter (2026-10-07)

Observed installed executable: `/home/dell/.local/bin/stellar`.
`stellar --version` returned CLI **27.0.0**, commit
`5a7c5fe76530bf4248477ac812fc757146b98cc4` (XDR 27.0.0).
No Phase 0 evidence file/help-command list existed in the inspected repository or
attached build plan. The full help inventory below was rerun before Phase 9 code
was written. Earlier Phase 6 `stellar contract build` syntax remains unchanged.
Help stdout is recorded in `fixtures/stellar-cli-27/` with trailing whitespace
removed; JSON, hash, version and diagnostic recordings retain observed output. These are observations,
not invented future-version or minimum-version requirements.

```bash
stellar --version
stellar --help
stellar contract --help
stellar contract build --help
stellar contract info --help
stellar contract info interface --help
stellar contract info meta --help
stellar contract info env-meta --help
stellar contract info build --help
stellar contract info hash --help
```

## Command dependencies

| Adapter operation | Verified command | Interpretation |
| --- | --- | --- |
| Version | `stellar --version` | Existing isolated version parser; XDR version lines excluded |
| Build contracts | `stellar contract build` | Existing check pipeline; cwd is discovered workspace root |
| Interface | `stellar contract info interface --wasm PATH --output json` | JSON array retained as opaque CLI entries |
| Metadata | `stellar contract info meta --wasm PATH --output json` | JSON array retained as opaque CLI entries |
| Environment metadata | `stellar contract info env-meta --wasm PATH --output json` | JSON array retained as opaque CLI entries |
| Build information | `stellar contract info build --wasm PATH` | **Not invoked**: can perform HTTP even for a local file; adapter returns network-policy error before spawning |
| Hash | `stellar contract info hash --wasm PATH` | One hexadecimal SHA-256 value; isolated exact-output parser |

Production command arguments are constructed only in
`crates/doctor-runner/src/stellar.rs`. Existing environment detection and the
check pipeline now use its version/build request constructors. Arguments are
separate OS strings and local paths are never interpolated into a shell.

JSON was chosen because the verified CLI offers it. No XDR type or Wasm
custom-section parser was invented. Parsers retain unknown JSON object fields.
A malformed JSON document, wrong top-level shape/nonobject entry, invalid hash
or unrecognized version is a typed error with captured evidence. The hash command
has no JSON output option; its exact observed text is fixture-tested.

## Verified source and network boundary

Exact-version official sources inspected:

- [Local source selection](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/shared.rs): `fetch` returns file bytes before network configuration/lookup when `--wasm` is present.
- [Interface JSON](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/interface.rs).
- [Metadata JSON and missing-section error](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/meta.rs).
- [Environment metadata JSON](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/env_meta.rs).
- [Build attestations](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/build.rs): `source_repo` can trigger a GitHub HTTP request. There is no verified offline option; this operation is deliberately unavailable under local-only policy.
- [Hash](https://github.com/stellar/stellar-cli/blob/v27.0.0/cmd/soroban-cli/src/commands/contract/info/hash.rs).

Inspection accepts a local regular file only. It never offers contract-id,
wasm-hash, RPC or network options. Build attestations are always explicitly
`SKIPPED_NETWORK_POLICY`, not a successful or completed inspection. Compiler/SDK
build metadata already embedded in the Wasm is retained in the metadata section.
The adapter must be reverified before changing this restriction.

## Recorded outputs and absent metadata

`interface.json`, `meta.json`, `env-meta.json` and `hash.txt` are actual outputs
from the installed SDK 28.0.0 package's `doctest_fixtures/contract.wasm`.
That bundled artifact's metadata reports SDK **21.0.1-preview.1** and protocol
**21**; packaging under SDK 28 does not imply it was compiled with SDK 28.
The Wasm binary is not redistributed. Parser tests use recorded output and
injected process results, clearly separated from real execution tests.

`absent-meta.stderr` and `absent-env-meta.stderr` were recorded from a local
minimal standard Wasm v1 module (eight-byte header, no sections). Both commands
exited **1**, with the final line:

```text
❌ error: no meta present in provided WASM file
```

Only this exact final diagnostic with exit 1 on meta/env-meta is classified as
absent. The original failed-command status and output remain in the report;
other nonzero commands remain errors. This text parser is isolated and tested.
Future diagnostic changes fail clearly rather than silently reporting success.

## Process runner and limits

Phase 3's existing runner already captures program/OS args/cwd, raw stdout/stderr,
status, elapsed time and timeout with typed errors. Its successful/nonzero/missing,
metacharacter, large-output and timeout tests are retained and rerun. Unix timeout
handling terminates the process group; the existing non-Unix fallback has weaker
descendant cleanup guarantees (see `docs/runner.md`). Wasm operations use 30-second
timeouts. File validation checks regular-file status and the standard v1 header;
full Wasm/spec validation is delegated to the real CLI. Inputs must remain stable
while inspection runs. No source or Wasm code is executed by inspection.

## Real execution versus test doubles

Default tests include a real version/hash smoke test if Stellar is installed;
absence prints an explicit skip message. Adapter/orchestration tests use injected
recorded results and are not claimed as real CLI execution. A separate opt-in
full real inspection copies `SUD_TEST_WASM` to a path with spaces and verifies
its bytes remain unchanged. It was executed locally against the observed SDK
bundled artifact; full Stellar builds are not part of Phase 9 inspection.
