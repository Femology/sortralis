# Soroban Upgrade Doctor — Product Requirements Document (PRD)

**Status:** Build-ready specification  
**Primary audience:** Stellar/Soroban smart-contract developers and maintainers  
**Initial product type:** Open-source command-line developer tool  
**Repository model:** One repository  
**Target network/tooling context:** Stellar Protocol 28 / Soroban Rust SDK v28

---

## 1. Product in one sentence

**Soroban Upgrade Doctor is a CLI that checks a Soroban smart-contract project before an upgrade and tells the developer what may break, why it may break, and what to fix before deploying.**

---

## 2. The problem in simple English

Soroban contracts are software. Software changes over time.

A team may:

- upgrade its `soroban-sdk` version;
- change a contract function;
- add or remove a field from stored data;
- change an event;
- change authorization logic;
- rebuild a contract with a newer Stellar toolchain;
- replace the Wasm code of an already-deployed contract.

Any of those changes can break a live contract or the applications that depend on it.

Protocol 28 makes some kinds of upgrades easier, especially contract data evolution and fleet upgrades, but developers still have to understand whether their **specific codebase** is safe to move forward.

Today, a developer often has to manually inspect:

- `Cargo.toml`;
- Rust source files;
- old and new contract specs;
- build output;
- tests;
- Wasm metadata;
- events;
- storage structures;
- upgrade functions;
- SDK migration notes.

That is slow and easy to get wrong.

Soroban Upgrade Doctor turns that review into one repeatable command.

---

## 3. Who this is for

### Primary user

A Soroban smart-contract developer who is preparing to:

- upgrade to a newer Soroban SDK;
- upgrade deployed Wasm;
- change public contract interfaces;
- modify stored contract data;
- prepare a release;
- review an upgrade PR.

### Secondary users

- protocol maintainers;
- auditors doing an initial compatibility pass;
- open-source contributors;
- teams maintaining multiple Soroban contracts;
- CI pipelines that should block unsafe upgrades.

---

## 4. The core user story

> “I changed my Soroban contract. Before I deploy the new version, I want one command that tells me whether the project builds, whether tests pass, whether the public interface changed, whether storage changes look migration-safe, whether known v28 migration problems exist, and what I need to review.”

---

## 5. What the product will do

The initial CLI will support four major workflows.

### 5.1 `doctor scan`

Analyze one Soroban repository.

Example:

```bash
soroban-upgrade-doctor scan .
```

It should inspect:

- repository structure;
- Cargo workspace;
- `soroban-sdk` dependency;
- Rust edition;
- Stellar CLI availability/version;
- relevant source-code patterns;
- contract build result;
- test result;
- Wasm metadata;
- contract interface/spec;
- obvious Protocol 28 migration issues.

It produces:

- terminal summary;
- detailed Markdown report;
- machine-readable JSON report.

---

### 5.2 `doctor compare`

Compare a previous contract Wasm with a candidate upgraded Wasm.

Example:

```bash
soroban-upgrade-doctor compare \
  --before ./artifacts/v1.wasm \
  --after ./artifacts/v2.wasm
```

It should detect differences in:

- public functions;
- function parameters;
- return types;
- public structs/types represented in the contract spec;
- errors;
- events when available from the contract specification;
- contract metadata.

It should classify findings as:

- INFO;
- WARNING;
- BREAKING;
- MANUAL_REVIEW.

---

### 5.3 `doctor check`

Run the complete upgrade-readiness gate.

Example:

```bash
soroban-upgrade-doctor check .
```

This should combine:

1. environment checks;
2. repository analysis;
3. source analysis;
4. `cargo test`;
5. `stellar contract build`;
6. Wasm inspection;
7. rule evaluation.

Exit code should be non-zero if configured blocking findings exist.

This command is intended for CI.

---

### 5.4 `doctor explain`

Explain a rule or finding.

Example:

```bash
soroban-upgrade-doctor explain SDK28_EXPORT_ARGUMENT
```

The output should say:

- what was detected;
- why it matters;
- evidence;
- what the developer should do;
- links/references stored in the rule metadata.

---

# 6. What “upgrade safe” means in this product

The tool must **not** claim that a contract is mathematically or economically safe.

“Upgrade ready” means:

- required toolchain checks passed;
- project builds;
- tests passed;
- no known blocking migration rule fired;
- public interface changes were reported;
- relevant storage changes were reported;
- any finding requiring human judgment was clearly marked.

The tool must always state:

> Passing Soroban Upgrade Doctor is not a security audit and does not prove that an upgrade is safe to deploy.

---

# 7. Why Protocol 28 matters

The first release focuses on Protocol 28 because it introduced important builder-facing changes.

Soroban Rust SDK v28 includes, among other changes:

- migration-friendly unpacking for named `#[contracttype]` structs;
- removal of the old `export` argument on `contracttype` and `contracterror`;
- changed event publishing behavior;
- executable-reference support;
- newer build expectations.

The tool should encode checks around documented v28 migration concerns, but each rule must reference a known source or a test fixture. No rule should be invented simply because it “sounds risky.”

---

# 8. MVP scope

The MVP is a **local CLI**.

It must include:

- Rust CLI;
- deterministic rule engine;
- Cargo project discovery;
- SDK-version detection;
- Stellar CLI detection;
- static source rules;
- test runner;
- contract builder;
- Wasm/spec inspection;
- before/after interface comparison;
- Markdown report;
- JSON report;
- CI-friendly exit codes;
- fixture projects;
- unit tests;
- integration tests;
- GitHub Actions CI;
- excellent README and CONTRIBUTING guide.

---

# 9. Explicitly out of scope for v0.1.0

Do not build these in the first release:

- hosted SaaS;
- user accounts;
- database;
- blockchain wallet;
- token;
- smart contract owned by Upgrade Doctor;
- automatic mainnet deployment;
- automated modification of production contracts;
- automatic migration transactions;
- AI-generated security conclusions;
- vulnerability scanner claiming audit coverage;
- GUI/dashboard;
- GitHub App;
- background monitoring.

These can be future issues.

---

# 10. Product principles

## 10.1 Evidence before advice

Every finding must contain concrete evidence.

Bad:

> “Your storage may be unsafe.”

Good:

> “`State` changed from `{ count: u32 }` to `{ count: u32, label: String }`. A newly added non-optional field requires manual migration review. File: `contracts/counter/src/state.rs`, line 14.”

---

## 10.2 No guessing

If the tool cannot prove something, classify it as `MANUAL_REVIEW`.

It must not invent:

- a contract ID;
- a network state;
- a migration requirement;
- a source line;
- a function signature;
- a successful test result.

---

## 10.3 Local first

The main analysis should work without sending source code to an external server.

---

## 10.4 Deterministic results

The same repository state and tool versions should produce the same findings.

---

## 10.5 Useful failure messages

If an external command fails, show:

- command;
- exit code;
- relevant stderr/stdout;
- recommended next step.

Never replace the real error with “Something went wrong.”

---

# 11. Proposed technical architecture

Use a single Rust workspace.

```text
soroban-upgrade-doctor/
├── Cargo.toml
├── crates/
│   ├── doctor-cli/
│   ├── doctor-core/
│   ├── doctor-cargo/
│   ├── doctor-source/
│   ├── doctor-wasm/
│   ├── doctor-runner/
│   └── doctor-report/
├── fixtures/
│   ├── healthy-v28/
│   ├── legacy-export-arg/
│   ├── storage-add-optional/
│   ├── storage-add-required/
│   ├── interface-breaking/
│   ├── event-change/
│   └── failing-tests/
├── tests/
│   └── cli/
├── docs/
├── scripts/
├── .github/
│   └── workflows/
├── README.md
├── CONTRIBUTING.md
├── SECURITY.md
├── LICENSE
└── CHANGELOG.md
```

### Responsibilities

#### `doctor-cli`

- command parsing;
- terminal output;
- exit codes;
- loading configuration.

#### `doctor-core`

- shared domain models;
- rule IDs;
- severity;
- findings;
- analysis pipeline.

#### `doctor-cargo`

- Cargo workspace discovery;
- dependency extraction;
- package identification;
- Soroban contract package detection.

#### `doctor-source`

- Rust source inspection;
- known migration-rule checks;
- source evidence and line locations.

#### `doctor-wasm`

- Wasm metadata/spec extraction;
- normalized contract-interface model;
- before/after comparison.

#### `doctor-runner`

- safe subprocess execution;
- `cargo test`;
- `stellar contract build`;
- Stellar CLI inspection;
- timeouts;
- captured output.

#### `doctor-report`

- human terminal report;
- Markdown report;
- JSON report.

---

# 12. Data model

## Finding

```text
id
title
severity
category
status
summary
why_it_matters
evidence[]
recommendation
references[]
```

### Severity

```text
INFO
WARNING
BREAKING
MANUAL_REVIEW
```

### Categories

```text
TOOLCHAIN
DEPENDENCY
SOURCE
STORAGE
INTERFACE
EVENT
AUTH
BUILD
TEST
WASM
METADATA
```

---

# 13. Initial rules

The exact implementation should be verified against official Stellar documentation before coding each rule.

### `SDK_VERSION_DETECTED`

Report current Soroban SDK version.

### `SDK28_REMOVED_EXPORT_ARGUMENT`

Detect deprecated/removed `export = ...` usage associated with `contracttype` or `contracterror`.

Expected severity:

`BREAKING` when targeting SDK v28.

### `SDK28_EVENT_SHAPE_REVIEW`

Detect event definitions/usage that should be reviewed for v28 sparse publishing behavior.

This rule must be conservative. If source analysis cannot prove a break, return `MANUAL_REVIEW`, not `BREAKING`.

### `CUSTOM_ACCOUNT_EXECUTABLE_REVIEW`

If the project implements custom account authorization (`__check_auth`), flag Protocol 28 executable/deployment argument handling for manual review.

### `STORAGE_ADDED_OPTIONAL_FIELD`

When comparing known source/schema versions, report an added `Option<T>` field as a migration-related informational change.

Do not claim that the entire migration is safe.

### `STORAGE_ADDED_REQUIRED_FIELD`

When a named stored structure appears to gain a required field, flag for manual migration review.

### `PUBLIC_FUNCTION_REMOVED`

If a public function exists in the old contract spec and not the new spec:

`BREAKING`.

### `PUBLIC_FUNCTION_SIGNATURE_CHANGED`

If argument/return types differ:

`BREAKING`.

### `PUBLIC_FUNCTION_ADDED`

Usually:

`INFO`.

### `PUBLIC_TYPE_CHANGED`

Classify according to exact diff. Ambiguous compatibility should be manual review.

### `BUILD_FAILED`

Blocking.

### `TESTS_FAILED`

Blocking.

### `STELLAR_CLI_TOO_OLD_FOR_BUILD`

Use verified version requirements rather than hard-coded guesses.

---

# 14. Contract interface normalization

The comparer needs a normalized representation independent of display order.

Example:

```text
ContractInterface
  functions:
    - name
    - inputs[]
    - output
  types[]
  errors[]
  events[]
  metadata{}
```

Before comparing:

- sort stable collections;
- remove irrelevant formatting differences;
- preserve meaningful type information;
- ensure the same input always gives the same JSON.

---

# 15. CLI experience

## Basic scan

```bash
soroban-upgrade-doctor scan .
```

Example conceptual output:

```text
Soroban Upgrade Doctor

Repository: ./payments-contract
Soroban contracts found: 2
SDK: 28.0.0
Stellar CLI: 28.0.0

PASS  Cargo workspace discovered
PASS  Tests passed
PASS  Contracts built
WARN  1 migration review required
FAIL  1 breaking public interface change

Report: ./doctor-report.md
JSON:   ./doctor-report.json

Verdict: NOT READY
```

---

# 16. Configuration

Optional file:

```text
upgrade-doctor.toml
```

Possible options:

```toml
fail_on = ["BREAKING"]
run_tests = true
build_contracts = true
report_dir = "./doctor-reports"
exclude = ["target", "vendor"]
```

The default configuration must be useful without a config file.

---

# 17. Exit codes

Proposed:

```text
0 = checks completed and no configured blocking finding
1 = findings failed policy
2 = invalid arguments/config
3 = environment/tool execution failure
4 = internal Upgrade Doctor error
```

Document these and test them.

---

# 18. Reporting

Each run should be able to produce:

## Terminal

Fast summary for developers.

## Markdown

Readable PR/CI artifact containing:

- environment;
- packages scanned;
- commands run;
- findings grouped by severity/category;
- evidence;
- recommendations;
- final verdict.

## JSON

Stable schema suitable for CI and future integrations.

Add a `schema_version`.

---

# 19. Test strategy

Testing is a core feature of the project, not cleanup at the end.

## Unit tests

Test:

- semver parsing;
- Cargo dependency extraction;
- severity policy;
- rule matching;
- source-line evidence;
- interface normalization;
- interface diffing;
- report rendering;
- exit-code decisions.

## Fixture tests

Each rule gets at least:

- positive fixture;
- negative fixture.

Example:

`legacy-export-arg` must trigger `SDK28_REMOVED_EXPORT_ARGUMENT`.

`healthy-v28` must not.

## Integration tests

Execute the compiled CLI against fixture repositories and verify:

- stdout;
- exit code;
- generated report;
- generated JSON;
- expected finding IDs.

## Golden/snapshot tests

Use snapshots for normalized report structures where practical.

Avoid snapshots for unstable absolute paths or timestamps unless normalized.

## End-to-end test

On CI:

1. install required Rust target/tooling;
2. build Upgrade Doctor;
3. run its tests;
4. run it against `healthy-v28`;
5. confirm fixture builds;
6. run it against an intentionally broken fixture;
7. confirm the expected non-zero exit code and rule IDs.

---

# 20. Security requirements

- never execute arbitrary repository scripts automatically;
- execute only explicitly documented commands;
- do not interpolate untrusted strings into a shell command;
- use process argument arrays rather than `sh -c`;
- add command timeouts;
- sanitize paths written into reports when needed;
- never upload source code automatically;
- do not store private keys;
- do not require a funded account for normal scans;
- do not deploy anything during `scan` or `check`.

---

# 21. Performance targets

For a normal small/medium Soroban repository:

- static scan should feel near-instant;
- full time will mostly be Cargo/build/test time;
- do not compile more than necessary;
- cache results only after correctness is established;
- show progress for long-running external commands.

No artificial performance claims should be placed in the README until measured.

---

# 22. Documentation requirements

README must include:

- what the tool does;
- what it does not do;
- quick start;
- prerequisites;
- commands;
- example report;
- rule catalog;
- CI example;
- limitations;
- security disclaimer;
- contributing link.

Additional docs:

```text
docs/
├── architecture.md
├── rules.md
├── reports.md
├── ci.md
├── protocol-28.md
└── adding-a-rule.md
```

---

# 23. Definition of MVP done

v0.1.0 is ready only when all of the following are true:

- CLI installs/builds successfully;
- `scan` works;
- `compare` works on two Wasm artifacts;
- `check` works and returns correct exit codes;
- `explain` works for registered rules;
- SDK and CLI versions are reported;
- fixture-based v28 source rules work;
- Cargo tests run through the tool;
- Soroban contract build runs through the tool;
- old/new contract interface comparison works;
- Markdown and JSON reports are generated;
- error messages preserve real command failures;
- all workspace tests pass;
- CI passes from a clean checkout;
- README quick start has been manually followed successfully;
- at least one healthy and one intentionally unsafe example are demonstrated;
- no code path claims a security audit or guaranteed upgrade safety.

---

# 24. Future roadmap

After v0.1.0:

### v0.2

- git-ref comparison (`main` vs PR branch);
- SARIF output for GitHub code scanning;
- richer event comparison;
- improved storage-schema analysis.

### v0.3

- GitHub Action wrapper;
- PR annotations;
- reusable rule packs.

### Later

- optional browser report viewer;
- contract deployment metadata comparison;
- RPC/on-chain contract fetch support;
- multi-contract fleet analysis;
- executable-reference upgrade analysis.

None of these should block v0.1.0.

---

# 25. Success metrics

For the initial open-source release:

- users can run first scan in under five documented steps;
- every blocking result has evidence;
- every supported rule has tests;
- CI can use the tool without interactive input;
- a contributor can add one rule by following `docs/adding-a-rule.md`;
- no false claim of safety is emitted;
- project has a meaningful issue backlog for contributors.

---

# 26. The simplest mental model

Think of Soroban Upgrade Doctor as:

**ESLint + migration checker + contract-interface diff + build/test gate for Soroban upgrades.**

It is not an auditor.

It is not a deployment service.

It is a pre-deployment doctor that catches upgrade mistakes early.
