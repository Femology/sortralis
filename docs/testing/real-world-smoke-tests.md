# Real-World Smoke Tests

Phase 14 validates Sortralis against pinned public Soroban repositories with different layouts and SDK generations. Every reported finding is manually classified as a true positive, useful review signal, false positive, or tool/environment signal. Confirmed false positives receive regression coverage before the phase is closed.

## Selected repositories

| Repository | Commit SHA | Layout / reason selected |
| --- | --- | --- |
| `stellar/soroban-example-dapp` | `07504b922bc75a48e5220711aea2cb4962f90367` | Older SDK 20 dApp with a root Cargo workspace and multiple contracts |
| `stellar/soroban-examples` | `03d42aa6b973dcf3a453a99d0c6a6e8d25a196e2` | SDK 28 examples repository; smoke target is the `workspace/` Cargo workspace |
| `OpenZeppelin/stellar-contracts` | `ce5c5688a514adc28693cbbafddd7eeb6011d2c8` | Large SDK 28 production-quality workspace with many contracts and reusable libraries |

The exact SHAs are intentionally pinned so later reruns analyze the same source.

## Methodology

1. Build Sortralis from the repository under test.
2. Clone each public repository at the exact recorded commit.
3. Run source-only `sortralis check --format json` sequentially with target-repository tests and contract builds disabled.
4. Preserve JSON, stderr, exit code, and rule IDs as GitHub Actions artifacts.
5. Manually inspect every reported source rule against the pinned source.
6. Verify Stellar/Soroban API claims against authoritative SDK v28 source before changing a detector.
7. Fix confirmed false positives with regression tests.
8. Rerun the full Sortralis CI suite and all three pinned public-repo smoke tests.

The source-only cloud workflow is [`.github/workflows/phase14-smoke.yml`](../../.github/workflows/phase14-smoke.yml). Third-party clones live under the ignored `/smoke-tests/` directory and are never committed.

## Automated smoke evidence

### Initial cloud run

Workflow run: `37810383996`

| Repository | Exit | Verdict | Findings | Breaking | Warning | Manual review |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| `soroban-example-dapp` | 0 | `CHECKS_COMPLETED_WITH_WARNINGS` | 1 | 0 | 1 | 0 |
| `soroban-examples` | 0 | `CHECKS_COMPLETED_WITH_WARNINGS` | 1 | 0 | 1 | 0 |
| `stellar-contracts` | 1 | `NOT_READY` | 4 | 1 | 1 | 2 |

The initial OpenZeppelin run exposed two over-broad rules.

### False-positive investigation and fixes

#### `P28-DEPLOY-001`

Initial behavior incorrectly treated `with_current_contract`, `with_address`, and `upload_contract_wasm` as deprecated Protocol 28 deployment APIs.

This was a **confirmed false positive**.

Verification against `soroban-sdk v28.0.0` showed those APIs remain supported. The v28 migration guidance specifically replaces deprecated `DeployerWithAddress::deploy_v2` with `deploy_contract`, and deprecated `update_current_contract_wasm` with `update_current_contract`.

The rule was narrowed to syntax-level detection of `deploy_v2`. It intentionally does not flag the generic method name `deploy` because Sortralis does not have Rust type resolution and doing so would create broad false positives.

Regression coverage now verifies that the modern pattern

```rust
env.deployer()
    .with_current_contract(salt)
    .deploy_contract(ContractExecutable::Wasm(wasm_hash), ());
```

does not trigger `P28-DEPLOY-001`.

#### `P28-AUTH-001`

Initial behavior required a literal `require_auth()` / `require_auth_for_args()` call in each upgrade function. This produced false alarms on OpenZeppelin functions protected by authorization procedural macros.

Pinned OpenZeppelin macro source was inspected and verified:

- `only_role` enforces role membership and authorization;
- `only_owner` enforces owner authorization;
- `only_admin` enforces admin authorization;
- `only_any_role` enforces role membership and authorization;
- `has_role` and `has_any_role` do **not** enforce authorization.

Sortralis now recognizes the four verified authorization attributes only when they resolve to `stellar_macros`. Lookalike local attributes and the non-authorizing `has_role` / `has_any_role` attributes do not suppress review.

The rule wording was also made conservative. A remaining finding now means no direct or verified macro-based authorization signal is visible and the caller/trait/helper/macro path requires human review; it no longer claims an unauthenticated takeover has been proven.

## Final cloud rerun

Workflow run: `37811362188`

Full Sortralis CI run: `37811362287` — **success**.

| Repository | Exit | Verdict | Findings | Breaking | Warning | Manual review |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| `soroban-example-dapp` | 0 | `CHECKS_COMPLETED_WITH_WARNINGS` | 1 | 0 | 1 | 0 |
| `soroban-examples` | 0 | `CHECKS_COMPLETED_WITH_WARNINGS` | 1 | 0 | 1 | 0 |
| `stellar-contracts` | 0 | `CHECKS_COMPLETED_WITH_WARNINGS` | 3 | 0 | 1 | 2 |

### Final finding classification

#### `soroban-example-dapp`

- `STELLAR_NOT_INSTALLED` — **tool/environment signal**.
  - Expected in the source-only cloud workflow because Stellar CLI is intentionally not installed.
  - Not a source-rule false positive.

A previous local full `check` run on this older SDK 20 project successfully produced Wasm artifacts, while its test step aborted with Cargo exit 101 / SIGABRT under the current host toolchain. The exact root cause has not been proven, so that remains an environment/toolchain failure under investigation rather than a Sortralis rule finding.

#### `soroban-examples`

- `STELLAR_NOT_INSTALLED` — **tool/environment signal**.
  - Expected in the source-only cloud workflow.
  - No Protocol 28 source-rule findings were produced for the pinned `workspace/` example.

#### `OpenZeppelin/stellar-contracts`

- `CUSTOM_ACCOUNT_EXECUTABLE_REVIEW` — **useful review signal**.
  - Triggered on implemented `__check_auth` methods.
  - SDK v28 migration guidance explicitly requires custom accounts to review the new `ContractExecutable::ExternalRef` authorization context.
  - Sortralis does not claim these implementations are defective.

- `P28-AUTH-001` — **useful manual-review signal**.
  - The only remaining evidence is `packages/contract-utils/src/upgradeable/storage.rs::upgrade`.
  - That low-level helper intentionally contains no authorization and its own documentation requires callers/admin entry points to enforce authorization.
  - Syntax-only analysis cannot prove every caller path, so manual review is the correct classification.

- `STELLAR_NOT_INSTALLED` — **tool/environment signal**.
  - Expected in the source-only cloud workflow.

The earlier blocking `P28-DEPLOY-001` finding disappeared after the verified rule correction. OpenZeppelin now exits 0 in the source-only smoke workflow.

## Phase 14 verdict

**Phase 14 real-world source smoke testing is complete.**

Evidence established:

- three public Soroban repositories with different layouts and SDK generations were tested at exact pinned commits;
- every reported source finding was manually classified;
- a real Protocol 28 deploy false positive was found and fixed;
- macro-based authorization false alarms were reduced using verified OpenZeppelin macro semantics;
- regression tests protect both fixes;
- the remaining findings are conservative manual-review or environment signals;
- the final public-repo smoke workflow passes;
- the full Sortralis CI workflow passes.

This phase does not claim that all target repositories were fully built and tested in GitHub Actions. The automated Phase 14 workflow is intentionally source-only to keep it deterministic, resource-bounded, and safe from arbitrary third-party build/test execution.
