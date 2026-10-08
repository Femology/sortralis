# End-to-End Fixture Matrix

This document defines the deterministic end-to-end fixture matrix for Sortralis, covering 15 scenarios. Each scenario specifies its purpose, verified CLI command, expected rule findings, exit codes, and Stellar CLI execution requirements.

---

## Fixture Scenarios

| # | Fixture Name | Purpose | CLI Command | Expected Rules | Expected Exit Code | Stellar CLI Required |
|---|--------------|---------|-------------|----------------|--------------------|----------------------|
| 1 | `p28-healthy` | Valid, clean SDK v28 contract with safe types and no deprecated APIs | `sortralis check fixtures/matrix/p28-healthy` | None | `0` (Completed) | No |
| 2 | `p27-legacy-upgrade-api` | Uses legacy `update_current_contract_wasm` API superseded in Protocol 28 | `sortralis check fixtures/matrix/p27-legacy-upgrade-api` | `P28-API-001` | `1` (PolicyFailed) | No |
| 3 | `p27-legacy-deploy-api` | Calls deprecated `DeployerWithAddress::deploy_v2` API | `sortralis check fixtures/matrix/p27-legacy-deploy-api` | `P28-DEPLOY-001` | `1` (PolicyFailed) | No |
| 4 | `p28-sparse-event-risk` | Contract event containing unit `()` void values subject to v28 sparse map omission | `sortralis check fixtures/matrix/p28-sparse-event-risk` | `P28-EVENT-001` | `0` (Completed / Review) | No |
| 5 | `p28-contracttrait-invalid` | Invalid `#[contracttrait]` macro application to non-trait item (struct/enum/fn) | `sortralis check fixtures/matrix/p28-contracttrait-invalid` | `P28-MACRO-001` | `1` (PolicyFailed) | No |
| 6 | `p28-internal-spec-reference` | Direct source reference to internal `__SPEC_XDR_` symbols or `ScSpecEntry` | `sortralis check fixtures/matrix/p28-internal-spec-reference` | `P28-SPEC-001` | `1` (PolicyFailed) | No |
| 7 | `upgrade-auth-direct` | Contract upgrade function containing explicit `admin.require_auth()` check | `sortralis check fixtures/matrix/upgrade-auth-direct` | None | `0` (Completed) | No |
| 8 | `upgrade-auth-none` | Contract upgrade function lacking any `require_auth()` authorization check | `sortralis check fixtures/matrix/upgrade-auth-none` | `P28-AUTH-001` | `0` (Completed / Review) | No |
| 9 | `storage-v1` vs `storage-v2` | Schema evolution where v2 adds a non-optional required field to stored state | `sortralis compare-source --before fixtures/matrix/storage-v1 --after fixtures/matrix/storage-v2 --json` | `STORAGE_ADDED_REQUIRED_FIELD` (`migration_risk: true`) | `0` (Completed) | No |
| 10 | `interface-v1` vs `interface-v2` | Contract interface evolution where v2 removes a public contract function | `sortralis compare --before fixtures/matrix/interface-v1 --after fixtures/matrix/interface-v2` | `FUNCTION_REMOVED` (`has_breaking_changes: true`) | `1` (PolicyFailed) | No |
| 11 | `no-soroban` | Non-Soroban repository or directory path lacking `Cargo.toml` | `sortralis check fixtures/matrix/no-soroban/src` | Clear user error (`no Cargo.toml`) | `2` (InvalidInput) | No |
| 12 | `malformed-rust` | Unparseable Rust source file with syntax errors | `sortralis check fixtures/matrix/malformed-rust` | `SOURCE_ANALYSIS_FAILED` | `3` (ExternalFailure) | No |
| 13 | `multiline-formatting` | Proves AST detection of `P28-API-001` is invariant to irregular line breaks and whitespace | `sortralis check fixtures/matrix/multiline-formatting` | `P28-API-001` | `1` (PolicyFailed) | No |
| 14 | `comments-and-strings` | Contains every watched symbol exclusively inside comments and string literals | `sortralis check fixtures/matrix/comments-and-strings` | None (Zero false positives) | `0` (Completed) | No |
| 15 | `multi-contract-workspace` | Multi-package Cargo workspace with multiple independent contracts | `sortralis check fixtures/matrix/multi-contract-workspace` | Deterministic contract inventory | `0` (Completed) | No |

---

## Detailed Scenario Specifications

### 1. `p28-healthy`
- **Purpose**: Verifies that a healthy Soroban contract targeting SDK v28 produces zero findings and passes the check pipeline without false positives.
- **Command**: `sortralis check fixtures/matrix/p28-healthy`
- **Expected Rules**: None (0 contract/P28 findings).
- **Expected Exit Code**: `0`.
- **Stellar CLI Required**: No.

### 2. `p27-legacy-upgrade-api`
- **Purpose**: Verifies detection of legacy `env.deployer().update_current_contract_wasm(...)` calls that must be migrated to Protocol 28 `ContractExecutable` mechanisms.
- **Command**: `sortralis check fixtures/matrix/p27-legacy-upgrade-api`
- **Expected Rules**: `P28-API-001` at exact source location.
- **Expected Exit Code**: `1` (Severity: Breaking).
- **Stellar CLI Required**: No.

### 3. `p27-legacy-deploy-api`
- **Purpose**: Verifies detection of deprecated `DeployerWithAddress::deploy_v2`, replaced in SDK v28 by `deploy_contract(ContractExecutable, constructor_args)`. Supported helpers such as `with_current_contract`, `with_address`, and `upload_contract_wasm` are intentionally not findings.
- **Command**: `sortralis check fixtures/matrix/p27-legacy-deploy-api`
- **Expected Rules**: `P28-DEPLOY-001`.
- **Expected Exit Code**: `1` (Severity: Breaking).
- **Stellar CLI Required**: No.

### 4. `p28-sparse-event-risk`
- **Purpose**: Identifies event definitions where unit/void fields (`()`) are emitted in map-format events and omitted under SDK v28 default sparse serialization.
- **Command**: `sortralis check fixtures/matrix/p28-sparse-event-risk`
- **Expected Rules**: `P28-EVENT-001`.
- **Expected Exit Code**: `0` (Severity: ManualReview).
- **Stellar CLI Required**: No.

### 5. `p28-contracttrait-invalid`
- **Purpose**: Proves macro finding is emitted only when AST analysis can prove invalid usage of `#[contracttrait]` on a non-trait item (e.g. struct).
- **Command**: `sortralis check fixtures/matrix/p28-contracttrait-invalid`
- **Expected Rules**: `P28-MACRO-001`.
- **Expected Exit Code**: `1` (Severity: Breaking).
- **Stellar CLI Required**: No.

### 6. `p28-internal-spec-reference`
- **Purpose**: Flags direct contract references to internal macro-generated spec symbols (`__SPEC_XDR_INPUT`) that change between toolchain versions.
- **Command**: `sortralis check fixtures/matrix/p28-internal-spec-reference`
- **Expected Rules**: `P28-SPEC-001`.
- **Expected Exit Code**: `1` (Severity: Breaking).
- **Stellar CLI Required**: No.

### 7. `upgrade-auth-direct`
- **Purpose**: Confirms that when a contract upgrade function verifies caller authorization via `admin.require_auth()`, no missing-auth warning is emitted.
- **Command**: `sortralis check fixtures/matrix/upgrade-auth-direct`
- **Expected Rules**: None.
- **Expected Exit Code**: `0`.
- **Stellar CLI Required**: No.

### 8. `upgrade-auth-none`
- **Purpose**: Flags upgrade functions where syntax analysis cannot see a direct or verified macro-based authorization signal. This is a manual-review signal, not proof that the function is exploitable; authorization may be delegated through a caller, trait, helper, or macro.
- **Command**: `sortralis check fixtures/matrix/upgrade-auth-none`
- **Expected Rules**: `P28-AUTH-001`.
- **Expected Exit Code**: `0` (Severity: ManualReview / REVIEW_REQUIRED).
- **Stellar CLI Required**: No.

### 9. `storage-v1` and `storage-v2`
- **Purpose**: Compares storage state between versions to detect schema migration risk when non-optional required fields are added to existing contract data structures.
- **Command**: `sortralis compare-source --before fixtures/matrix/storage-v1 --after fixtures/matrix/storage-v2 --json`
- **Expected Rules / Output**: `migration_risk: true`, `STORAGE_ADDED_REQUIRED_FIELD`.
- **Expected Exit Code**: `0`.
- **Stellar CLI Required**: No.

### 10. `interface-v1` and `interface-v2`
- **Purpose**: Compares public contract interfaces between versions to detect breaking public API changes (such as removed functions).
- **Command**: `sortralis compare --before fixtures/matrix/interface-v1 --after fixtures/matrix/interface-v2`
- **Expected Rules / Output**: `has_breaking_changes: true`, public function `remove_me` removed.
- **Expected Exit Code**: `1`.
- **Stellar CLI Required**: No.

### 11. `no-soroban`
- **Purpose**: Validates clear user error messaging when invoked against a non-cargo or non-contract directory.
- **Command**: `sortralis check fixtures/matrix/no-soroban/src`
- **Expected Rules**: Structured error: `no Cargo.toml at ...`.
- **Expected Exit Code**: `2` (InvalidInput).
- **Stellar CLI Required**: No.

### 12. `malformed-rust`
- **Purpose**: Confirms that unparseable Rust files trigger structured syntax analysis errors without panics or crashes.
- **Command**: `sortralis check fixtures/matrix/malformed-rust`
- **Expected Rules**: `SOURCE_ANALYSIS_FAILED` finding with parse location.
- **Expected Exit Code**: `3` (ExternalFailure).
- **Stellar CLI Required**: No.

### 13. `multiline-formatting`
- **Purpose**: Confirms that AST-based syntax analysis detects rules identically regardless of indentation, comments, or multi-line formatting of method invocations.
- **Command**: `sortralis check fixtures/matrix/multiline-formatting`
- **Expected Rules**: `P28-API-001`.
- **Expected Exit Code**: `1`.
- **Stellar CLI Required**: No.

### 14. `comments-and-strings`
- **Purpose**: Phase Gate verification ensuring zero false positives when watched symbols and keywords appear solely within comments or string literals.
- **Command**: `sortralis check fixtures/matrix/comments-and-strings`
- **Expected Rules**: None (0 findings).
- **Expected Exit Code**: `0`.
- **Stellar CLI Required**: No.

### 15. `multi-contract-workspace`
- **Purpose**: Verifies that workspaces with multiple contract crates produce deterministic ordering and consistent findings across repeated invocations.
- **Command**: `sortralis check fixtures/matrix/multi-contract-workspace`
- **Expected Rules**: None.
- **Expected Exit Code**: `0`.
- **Stellar CLI Required**: No.
