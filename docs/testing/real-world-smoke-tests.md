# Real-World Smoke Tests

For Phase 14, we perform real-world smoke tests on public Soroban contract repositories to validate the rules and fix obvious false positives.

## Selected Repositories

| Repository | URL | Commit SHA | Type |
| --- | --- | --- | --- |
| `soroban-examples` | https://github.com/stellar/soroban-examples | `03d42aa6b973dcf3a453a99d0c6a6e8d25a196e2` | Workspace (multiple examples) |
| `soroban-example-dapp` | https://github.com/stellar/soroban-example-dapp | `07504b922bc75a48e5220711aea2cb4962f90367` | Simple Contract / dApp |
| `stellar-contracts` | https://github.com/OpenZeppelin/stellar-contracts | `ce5c5688a514adc28693cbbafddd7eeb6011d2c8` | Workspace (OpenZeppelin library) |

## Methodology

1. Run source scan on each repository.
2. Verify when the environment supports it.
3. Perform WASM inspection when the build succeeds.
4. Manually review every warning/error produced.
5. Classify each as:
   - true positive
   - useful review signal
   - false positive
   - tool/environment failure
6. Fix clear false positives in Upgrade Doctor.

## Current Progress (Phase 14 - In Progress)

### 1. `soroban-example-dapp`
- **Source Scan / Check**: Ran `sortralis check` against `abundance` and `crowdfund` contracts (`soroban-sdk ^20.0.0-rc2`).
- **Wasm Build**: Contract build succeeded and produced valid `.wasm` artifacts (`abundance_token.wasm`, `soroban_crowdfund_contract.wasm`).
- **Tests**: Recorded test abort (exit 101/SIGABRT) caused by pre-v28 SDK linker/testutils mismatch under current host rustc toolchain (classified as environment/toolchain signal, not a rule false positive).
- **Source Rules**: Zero false positives reported.

### 2. `soroban-examples`
- **Setup**: Cloned at commit `03d42aa6b973dcf3a453a99d0c6a6e8d25a196e2` (`soroban-sdk 28.0.0`).
- **Status**: Ready for full `verify` and individual contract scans.

### 3. `stellar-contracts`
- **Setup**: Cloned OpenZeppelin repository at commit `ce5c5688a514adc28693cbbafddd7eeb6011d2c8` (`soroban-sdk 28.0.0`, 39 member packages).
- **Status**: Workspace ready for inspection.
