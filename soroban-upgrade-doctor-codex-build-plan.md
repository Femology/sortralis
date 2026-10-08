# Soroban Upgrade Doctor — Production Build Plan for Codex

> **Purpose:** Build Soroban Upgrade Doctor from an empty repository to a production-quality v0.1.0 in small, verifiable phases.
>
> **Rule:** Do not skip phases. Do not invent APIs. Do not claim a phase is complete until every required command has been run and its output has been checked.

---

# 0. GLOBAL INSTRUCTIONS FOR CODEX

Use this block at the beginning of **every** phase prompt.

```text
You are the implementation engineer for Soroban Upgrade Doctor.

Work only from the repository state you can inspect and from authoritative documentation you can actually verify.

NON-NEGOTIABLE RULES

1. Never hallucinate:
   - package versions;
   - Stellar CLI syntax;
   - Soroban SDK APIs;
   - XDR types;
   - WASM custom-section formats;
   - file paths;
   - command output;
   - test results.

2. Before using any Stellar/Soroban API that is not already proven in this repository:
   - inspect the installed dependency/source/docs available to you;
   - if internet/docs access is available, prefer official Stellar docs, docs.rs for the exact crate version, and official Stellar GitHub repositories;
   - record the verified source in notes or code comments only where it is useful;
   - if verification is impossible, stop that specific implementation and report exactly what is unverified. Do not invent a replacement.

3. Inspect before editing:
   - run `git status --short`;
   - inspect relevant Cargo.toml files;
   - inspect existing source/tests;
   - preserve working functionality.

4. No placeholders in production paths:
   - no `todo!()`;
   - no `unimplemented!()`;
   - no dummy success responses;
   - no fake parsers;
   - no tests that assert only that code executed.

5. Rust quality:
   - use typed errors;
   - avoid `unwrap()` and `expect()` in production code unless an invariant is locally proven and documented;
   - tests may use `unwrap()` when appropriate;
   - run `cargo fmt --all -- --check`;
   - run `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
   - run `cargo test --workspace --all-features`.

6. Shell/process safety:
   - never build shell commands by string concatenation;
   - use `std::process::Command` or a safe equivalent with separate args;
   - never execute arbitrary scripts found in the target repository;
   - impose timeouts on external commands where the implementation supports it.

7. Testing discipline:
   - write or update tests in the same phase as the behavior;
   - include a positive and negative test for each detection rule;
   - execute the tests yourself;
   - if any test fails, diagnose and fix it before declaring success;
   - never delete or weaken a legitimate failing test just to make CI green.

8. Scope discipline:
   - implement only the current phase;
   - do not add a web frontend, database, wallet, smart contract, AI API, or hosted service;
   - do not refactor unrelated working code.

9. Evidence in your final response for the phase:
   - list files changed;
   - list exact commands run;
   - state pass/fail for each command;
   - describe remaining limitations;
   - include `git diff --stat`;
   - do not say “all tests pass” unless you actually ran them in this session.

10. Git:
   - do not use `git add .`;
   - stage only files belonging to the current logical change;
   - use conventional commit format if you are asked to commit;
   - do not rewrite history.

PROJECT DEFINITION

Soroban Upgrade Doctor is a local-first CLI for analyzing Soroban contract upgrades. It checks environment/toolchain compatibility, source migration risks, test/build health, Wasm contract interfaces, and before/after compatibility. It emits terminal, Markdown, and JSON reports.

The tool does NOT claim to be a security audit and must never output guaranteed “safe to deploy” language.
```

---

# PHASE 1 — Repository scaffold and quality gates

## Goal

Create a clean Rust workspace and CI-quality baseline before implementing product logic.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 1 OBJECTIVE

Create the initial Rust workspace for Soroban Upgrade Doctor.

Required structure:

soroban-upgrade-doctor/
  Cargo.toml
  crates/
    doctor-cli/
    doctor-core/
    doctor-cargo/
    doctor-source/
    doctor-wasm/
    doctor-runner/
    doctor-report/
  fixtures/
  tests/
  docs/
  scripts/
  .github/workflows/
  README.md
  CONTRIBUTING.md
  SECURITY.md
  LICENSE
  CHANGELOG.md

Requirements:

1. Workspace
   - Rust workspace resolver = "2".
   - Use edition 2021 unless the currently verified project/toolchain requires a newer edition; do not guess.
   - Each crate must compile.
   - `doctor-cli` is the only binary crate.
   - Other crates are libraries.
   - Keep dependencies minimal.

2. Core API placeholder rule
   - It is acceptable for crates to contain tiny real data types/functions needed to compile.
   - Do NOT add `todo!()` or pretend functionality exists.
   - CLI may print a version/help message only.

3. CLI
   - Add clap using a verified current compatible version.
   - Binary name: `soroban-upgrade-doctor`.
   - Add subcommand names now:
     `scan`
     `compare`
     `check`
     `explain`
   - They may return a typed “not implemented in this phase” error and non-zero exit code.
   - Help output must work.

4. Quality configuration
   - Add `.gitignore`.
   - Add workspace lint configuration where practical.
   - Add GitHub Actions CI for:
       cargo fmt --all -- --check
       cargo clippy --workspace --all-targets --all-features -- -D warnings
       cargo test --workspace --all-features
   - Do not add fake coverage badges.

5. Docs
   - README: concise product description, current pre-alpha status, build instructions, explicit security-audit disclaimer.
   - SECURITY.md: responsible disclosure instructions and “not audited” note.
   - CONTRIBUTING.md: local checks required before PR.

6. Tests
   - Add a CLI integration test that verifies:
       --help exits 0;
       all four subcommand names appear.
   - Do not test exact full help text if that would be unnecessarily brittle.

VERIFY

Run:
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace --all-features
- cargo run -p doctor-cli -- --help

Do not proceed until all pass.

DELIVERABLE

Summarize files created, commands run, results, limitations, and git diff --stat.
```

## Phase 1 acceptance gate

- Workspace compiles.
- CI exists.
- Help command works.
- Tests pass.
- No fake implementation.

---

# PHASE 2 — Domain model, configuration, findings and exit policy

## Goal

Create the stable internal model used by every later analyzer.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 2 OBJECTIVE

Implement the domain model in `doctor-core`.

Add typed models for:

Finding:
- id: stable machine-readable rule ID
- title
- severity
- category
- status if useful
- summary
- why_it_matters
- evidence: Vec<Evidence>
- recommendation
- references

Evidence:
- path when available
- line when available
- message
- optional command evidence

Severity enum:
- Info
- Warning
- Breaking
- ManualReview

Category enum:
- Toolchain
- Dependency
- Source
- Storage
- Interface
- Event
- Auth
- Build
- Test
- Wasm
- Metadata

Scan/analysis result:
- tool version
- repository path
- detected packages
- findings
- command results
- verdict

Configuration:
- optional `upgrade-doctor.toml`
- default fail_on includes Breaking
- run_tests = true
- build_contracts = true
- report_dir default is sensible and documented
- exclude list

Exit policy:
0 = completed/no configured blocking finding
1 = policy failed because findings
2 = invalid CLI/config
3 = external environment/tool execution failure
4 = internal tool error

Requirements:

1. Use serde for JSON-serializable stable structures.
2. Add a report schema version constant.
3. Do not mix terminal formatting into core models.
4. Make severity ordering explicit and tested; never rely on enum declaration order accidentally.
5. Invalid configuration must return a useful typed error.
6. Unknown `fail_on` values must fail clearly instead of being ignored.

Tests:
- serialization round trip;
- default config;
- explicit config parsing;
- invalid severity/config rejection;
- blocking policy decisions;
- stable rule ID behavior;
- exit-code decisions.

Update README with the documented exit codes and config example.

VERIFY:
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace --all-features

Do not implement Cargo scanning yet.
```

## Acceptance gate

All later crates can depend on `doctor-core` without inventing their own finding format.

---

# PHASE 3 — Safe external command runner and environment detection

## Goal

Build the trustworthy process-execution layer first.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 3 OBJECTIVE

Implement `doctor-runner`.

Capabilities:

1. Safe command execution abstraction.
2. Capture:
   - program;
   - args;
   - working directory;
   - exit code;
   - stdout;
   - stderr;
   - elapsed duration;
   - timeout status.
3. Never invoke through `sh -c`, `bash -c`, PowerShell string execution, or equivalent.
4. Add timeout support.
5. Preserve raw failure evidence.

Environment detection commands:
- `rustc --version`
- `cargo --version`
- `stellar --version`

Before parsing Stellar CLI version:
- run the real command on the current environment if available;
- inspect representative verified output;
- implement tolerant version parsing;
- if Stellar CLI is absent, return a structured “not installed” environment finding rather than panicking.

Do not hard-code that any particular version is required yet. This phase detects versions only.

Tests:
- successful command;
- command with non-zero exit;
- missing binary;
- timeout;
- stdout/stderr capture;
- version parser fixtures.

CLI:
Add a temporary internal path so `scan` can print detected environment versions, but do not claim the full scan is implemented.

VERIFY full workspace checks.

SECURITY TEST:
Prove through a unit/integration test that an argument containing shell metacharacters is passed as an argument and is not executed as a second command.
```

## Acceptance gate

External command execution is safe and its failures are observable.

---

# PHASE 4 — Cargo workspace and Soroban project discovery

## Goal

Understand a target repository without fragile assumptions.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 4 OBJECTIVE

Implement `doctor-cargo`.

Use Cargo's machine-readable metadata rather than manually guessing workspace layout.

Before coding:
- inspect the documented output/schema of `cargo metadata --format-version 1`;
- prefer a maintained crate such as `cargo_metadata` if compatible;
- verify its exact API/version before use.

The analyzer must:

1. Accept a repository path.
2. Locate Cargo workspace/package information.
3. Identify packages that likely contain Soroban contracts based on verified evidence such as:
   - dependency on `soroban-sdk`;
   - crate type/configuration;
   - source use, only if needed.
4. Extract the resolved/requested `soroban-sdk` dependency requirement where possible.
5. Report multiple contract packages.
6. Handle:
   - non-Cargo directory;
   - Cargo project without Soroban;
   - malformed manifest;
   - virtual workspace;
   - workspace dependency inherited by child crates.

Do not modify the target repository.

Create fixtures:
- healthy-v28 workspace;
- non-soroban Rust project;
- workspace-inherited soroban dependency.

Fixture versions must use verified values available to the development environment. If compiling a fixture would require network downloads not available, design metadata-only fixture tests and clearly document that limitation.

Tests:
- finds workspace;
- finds contract package;
- finds multiple contracts;
- detects inherited dependency;
- gracefully reports no Soroban packages;
- path evidence is correct.

Integrate this into `scan` so it prints a real repository summary.

VERIFY full workspace checks.
```

## Acceptance gate

A repo can be discovered correctly without assuming `contracts/*`.

---

# PHASE 5 — Source analyzer framework and Protocol 28 source rules

## Goal

Implement the first genuinely useful migration checks.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 5 OBJECTIVE

Implement `doctor-source` with a rule registry and source evidence.

IMPORTANT:
Do not use naive regex for Rust syntax when it would cause obvious false positives. Prefer a verified parser approach (`syn`, tree-sitter-rust, or another appropriate library) after checking compatibility.

Build a generic rule interface:
- stable rule ID;
- description;
- supported target context;
- analyze(...);
- findings with exact file/line evidence when possible.

Implement only rules that can be supported by verified Stellar SDK v28 migration documentation.

Required rules:

1. SDK28_REMOVED_EXPORT_ARGUMENT
   Detect use of removed `export` argument in `contracttype` or `contracterror` attributes/macros when targeting/using v28.
   Severity: Breaking.
   Evidence must identify source location.

2. CUSTOM_ACCOUNT_EXECUTABLE_REVIEW
   Detect a project that implements `__check_auth`.
   Produce ManualReview explaining that Protocol 28 executable/deployment argument handling must be reviewed.
   Do not claim it is broken merely because `__check_auth` exists.

3. SDK28_EVENT_SHAPE_REVIEW
   Implement only if you can derive a deterministic, useful source pattern from the verified migration documentation.
   Otherwise register an explanation that this check is currently manual and DO NOT fabricate a static detector.

Rule engine requirements:
- rules are independently testable;
- duplicate findings are prevented or intentionally grouped;
- excluded paths are respected;
- `target/` is ignored;
- generated/vendor directories are not scanned by default.

Fixtures:
- legacy-export-arg positive;
- healthy-v28 negative;
- custom-account positive;
- ordinary contract negative.

For each rule add:
- positive test;
- negative test;
- test for exact stable ID;
- evidence path/line test when deterministic.

Update `doctor explain <RULE_ID>` to return rule documentation from the registry.

VERIFY full workspace checks.
```

## Acceptance gate

At least two real Protocol 28 checks work with fixture evidence.

---

# PHASE 6 — Test/build execution and real upgrade-readiness gate

## Goal

Make `check` prove whether the target repo actually builds and tests.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 6 OBJECTIVE

Wire real build and test execution into the analysis pipeline.

Required behavior for a Soroban repository:

1. If config `run_tests=true`:
   - run `cargo test` from the correct workspace root;
   - capture output;
   - create TEST finding on failure;
   - never replace the actual stderr with a generic message.

2. If config `build_contracts=true`:
   - verify the exact currently supported Stellar CLI build command from installed CLI help or official docs;
   - run the verified `stellar contract build` command from the target workspace.
   - create BUILD finding on failure.

3. Determine whether the detected Stellar CLI can build SDK v28 contracts using a VERIFIED requirement.
   - Do not invent a version threshold.
   - Centralize this compatibility fact so it is easy to update.
   - Add a source/reference to rule metadata.

4. `check` pipeline order:
   - validate path/config;
   - detect environment;
   - Cargo discovery;
   - static source analysis;
   - tests;
   - build;
   - aggregate findings;
   - apply exit policy;
   - reports will be added later.

5. A failure in tests should not necessarily prevent static findings from being displayed.
6. A missing required build tool is environment exit code 3.
7. A successful command must be explicitly recorded; do not infer it.

Fixtures:
- healthy-v28;
- failing-tests.

Integration tests should run against the fixtures where the environment permits.
When Stellar tooling is not available in CI, use a command-runner test double for unit tests and add a separate opt-in real-tool integration test. Do not pretend the real command ran.

VERIFY all tests and document which tests use the real Stellar CLI.
```

## Acceptance gate

`check` provides truthful build/test status and meaningful exit codes.

---

# PHASE 7 — Wasm contract information extraction

## Goal

Read the contract interface from real Soroban Wasm without inventing a parser.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 7 OBJECTIVE

Implement `doctor-wasm` interface extraction.

First research the CURRENT verified mechanisms for obtaining:
- contract interface/spec;
- contract metadata;
from a local Wasm file using Stellar CLI and/or maintained Stellar Rust crates.

Prefer the least fragile supported approach.

Current official CLI documentation should be checked for `stellar contract info` capabilities. Do not rely on deprecated `inspect` if a supported command exists.

Implementation requirements:

1. Input: path to Wasm.
2. Validate file exists/is readable.
3. Extract contract interface into an INTERNAL normalized model:
   ContractInterface
     functions
     public types
     errors
     events if the verified interface exposes them
     metadata

4. Keep the raw external output available for diagnostics.
5. Parser errors must include actionable evidence.
6. Do not parse human terminal output with brittle regex if the CLI offers structured output or a stable Rust/XDR API.
7. If structured CLI output is unavailable, investigate a maintained XDR/spec parsing crate and verify exact APIs before implementation.

Normalization:
- deterministic ordering;
- meaningful type identity preserved;
- no timestamps or machine-specific paths.

Tests:
- known fixture Wasm;
- invalid Wasm;
- missing file;
- deterministic normalized output;
- metadata presence/absence.

Fixture Wasm MUST be built from fixture source by a reproducible script or checked-in only if licensing/provenance is documented.
Prefer build-at-test/setup where reliable.

Do not implement comparison yet.
```

## Acceptance gate

The tool can produce a deterministic structured representation of a real contract interface.

---

# PHASE 8 — Interface diff engine and `compare`

## Goal

Detect breaking public contract changes.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 8 OBJECTIVE

Implement deterministic before/after contract interface comparison.

Command:

soroban-upgrade-doctor compare --before <old.wasm> --after <new.wasm>

Rules required:

PUBLIC_FUNCTION_REMOVED
- function in old, absent in new
- severity Breaking

PUBLIC_FUNCTION_ADDED
- function absent in old, present in new
- severity Info

PUBLIC_FUNCTION_SIGNATURE_CHANGED
- same function name but input/output signature changed
- severity Breaking

PUBLIC_TYPE_CHANGED
- detect structurally meaningful public type differences
- classify Breaking only when compatibility impact is clear;
- otherwise ManualReview.

PUBLIC_ERROR_CHANGED
- report removed/renumbered/changed errors based on what the normalized spec can prove.
- Do not overclaim semantic compatibility.

EVENT_INTERFACE_CHANGED
- only implement if event specification data can be reliably extracted.
- otherwise leave as documented limitation rather than inventing support.

Requirements:
- diff output order deterministic;
- evidence shows old and new forms;
- no duplicate findings;
- compare identical artifact to itself => zero interface-change findings.

Fixtures:
- v1 basic contract;
- v2 added function;
- v2 removed function;
- v2 changed signature;
- type-change pair.

Tests:
- one focused test per rule;
- identical comparison;
- deterministic ordering;
- CLI exit behavior.

VERIFY full workspace checks and manually run one compare example.
```

## Acceptance gate

The tool catches provable interface breaks between two built versions.

---

# PHASE 9 — Storage migration analyzer

## Goal

Provide useful, conservative analysis of data-structure evolution.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 9 OBJECTIVE

Implement conservative storage/schema migration analysis.

This is a high-risk area for false claims. The product must not say “safe” just because a field is optional.

First verify Soroban Rust SDK v28 migration behavior for named `#[contracttype]` structs from official SDK v28 migration docs.

Design:

- compare an explicit before/after source representation OR another deterministic representation you can prove corresponds to relevant stored structures;
- do not assume every `#[contracttype]` struct is persisted;
- if persistence cannot be proven, phrase the finding as “this contract type changed and may affect stored values if used in storage.”

Rules:

STORAGE_ADDED_OPTIONAL_FIELD
- detect named field added as Option<T>;
- severity Info or ManualReview depending on proven storage usage;
- explain v28 sparse-unpack behavior without claiming full migration safety.

STORAGE_REMOVED_FIELD
- report removal;
- classify conservatively.

STORAGE_ADDED_REQUIRED_FIELD
- required field added;
- ManualReview or Breaking only if the tool can prove incompatibility in the actual target context.
- do not invent certainty.

STORAGE_TYPE_CHANGED
- field type changed;
- ManualReview at minimum.

Requirements:
- include before/after evidence;
- distinguish named structs from tuple structs/enums;
- do not apply named-map migration assumptions to tuple structs/enums;
- tests must encode documented v28 behavior.

If source-version comparison is too large for this phase, implement it behind:
`compare-source --before <dir> --after <dir>`
or an internal API used by fixture tests.
Do not silently infer an old source version that the user did not supply.

Add documentation explaining exactly what can and cannot be concluded.
```

## Acceptance gate

Storage findings are useful without pretending the tool understands runtime state it cannot see.

---

# PHASE 10 — Markdown/JSON/terminal reports

## Goal

Turn raw findings into a polished artifact suitable for a PR or Wave submission.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 10 OBJECTIVE

Implement `doctor-report`.

Outputs:

1. terminal
2. Markdown
3. JSON

JSON:
- stable top-level schema_version;
- product version;
- repository;
- environment;
- packages;
- commands run and status;
- findings;
- verdict;
- no ANSI escapes.

Markdown:
- title and timestamp;
- disclaimer;
- repository summary;
- environment/tool versions;
- commands run;
- findings grouped by severity;
- evidence;
- recommendations;
- final verdict;
- limitations.

Terminal:
- concise;
- color only when terminal supports it;
- `--no-color` option;
- plain output in redirected/CI contexts where appropriate.

Do not print “SAFE”.
Use verdict language:
- READY_FOR_MANUAL_REVIEW
- NOT_READY
- CHECKS_COMPLETED_WITH_WARNINGS
or another clearly documented set.

Default report filenames:
- doctor-report.md
- doctor-report.json
unless config changes report directory.

Requirements:
- atomic file writes where practical;
- useful error if output directory is unwritable;
- deterministic JSON field/content ordering where appropriate;
- timestamps are allowed in complete reports but snapshot tests must normalize them.

Tests:
- Markdown golden test;
- JSON schema/serialization test;
- no ANSI in JSON;
- escaping of weird file paths/content;
- unwritable path behavior;
- terminal snapshot with color disabled.
```

## Acceptance gate

Every `scan`/`check` run can leave a clear artifact behind.

---

# PHASE 11 — Complete CLI UX

## Goal

Make every public command coherent.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 11 OBJECTIVE

Finish the public CLI.

Commands:

1. scan <path>
   - environment
   - Cargo discovery
   - static rules
   - optional lightweight Wasm inspection if artifacts are provided/found deterministically
   - no tests/build unless scan semantics explicitly include them; document the choice.

2. check <path>
   - full static + test + build gate
   - writes reports by default

3. compare --before <wasm> --after <wasm>
   - interface diff
   - optional output reports

4. explain <RULE_ID>
   - title
   - severity/default classification
   - meaning
   - detection method
   - recommendation
   - references

Global flags:
- --config
- --format or explicit report controls if needed
- --no-color
- --verbose
- --version

Requirements:
- paths support spaces;
- relative paths normalized carefully without requiring canonicalization where it breaks symlinks unnecessarily;
- errors go to stderr;
- machine-readable output must not be polluted by logs;
- Ctrl-C/interrupt should not leave corrupted report files.

Add CLI integration tests for:
- help;
- version;
- bad path;
- bad config;
- unknown rule;
- successful fixture scan;
- expected failing fixture check;
- compare identical;
- compare breaking;
- output file creation.
```

## Acceptance gate

A new user can use all four commands from `--help` alone.

---

# PHASE 12 — CI, real-tool integration and reproducibility

## Goal

Prove the repo works from a clean checkout.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 12 OBJECTIVE

Strengthen CI and reproducibility.

Before editing CI:
- verify current Rust requirements for the chosen dependencies;
- verify current Stellar CLI installation approach from official documentation/releases;
- verify SDK v28 contract build requirements.

CI jobs:

1. quality
   - fmt
   - clippy
   - unit/integration tests not requiring Stellar CLI

2. stellar-integration
   - install/pin a verified Stellar CLI version compatible with fixtures
   - ensure required Rust wasm target exists
   - build fixture Soroban contracts
   - run real `soroban-upgrade-doctor check fixtures/healthy-v28`
   - run intentionally failing fixture and assert the EXPECTED non-zero code, not generic failure
   - build before/after fixture Wasms
   - run `compare`
   - assert expected stable finding IDs

3. release-build
   - cargo build --release
   - smoke test --help/--version

Pin versions where reproducibility matters. Do not blindly use `latest` for every tool.

Add:
- `scripts/verify.sh` or cross-platform equivalent if appropriate;
- documented local equivalent of CI.

Do not hide flaky tests with unconditional retries. Fix the cause.
```

## Acceptance gate

A clean CI runner proves the actual Stellar integration works.

---

# PHASE 13 — Documentation and contributor readiness

## Goal

Make this look and behave like a real open-source maintainer project.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 13 OBJECTIVE

Write production documentation from the REAL implemented CLI.

Never document a command you have not executed successfully.

README sections:
- logo/banner placeholder only if an actual asset exists; do not reference missing images;
- one-sentence description;
- why the project exists;
- features;
- non-goals;
- requirements;
- install/build;
- 5-minute quick start;
- scan example;
- check example;
- compare example;
- sample real output captured from fixtures;
- CI usage;
- rule catalog link;
- limitations;
- security disclaimer;
- contributing.

Docs:
docs/architecture.md
docs/rules.md
docs/reports.md
docs/ci.md
docs/protocol-28.md
docs/adding-a-rule.md

`docs/adding-a-rule.md` must walk a contributor through:
- create stable ID;
- implement detector;
- add positive fixture/test;
- add negative fixture/test;
- add explanation/reference;
- run checks.

CONTRIBUTING.md:
- setup;
- test commands;
- commit/PR expectations;
- how to add fixtures;
- no invented protocol rules.

SECURITY.md:
- disclosure route;
- supported versions;
- explicit “not a security audit” note.

Create a real issue backlog document or `docs/roadmap.md` containing 20+ meaningful future issues grouped by:
- new rules;
- interface analysis;
- storage analysis;
- CI/GitHub integration;
- developer UX;
- testing/docs.

Do NOT create fake GitHub issues unless GitHub access is explicitly available and requested.
```

## Acceptance gate

An outside developer can install, run, understand, and contribute.

---

# PHASE 14 — Security, robustness and failure-mode hardening

## Goal

Try to break the tool before release.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 14 OBJECTIVE

Perform a focused robustness pass.

Threat/failure cases:

- repository path contains spaces;
- repository path contains shell metacharacters;
- symlinked directories;
- unreadable file;
- malformed Cargo.toml;
- huge stderr from failed build;
- command timeout;
- missing Stellar CLI;
- unsupported/unknown version string;
- invalid Wasm;
- valid Wasm without expected Soroban sections;
- report directory unwritable;
- Unicode source paths;
- duplicate function/type names in malformed input;
- interrupted command;
- target repository containing malicious build scripts.

Important:
`cargo test` and `stellar contract build` can execute build-time code belonging to the target repository. This is a real trust boundary.

Update documentation to explicitly warn:
- only run full `check` on repositories you trust;
- `scan` static analysis should avoid executing target code where possible.

Review every process invocation.
Confirm no untrusted value passes through a shell.

Add regression tests for every bug found.

Run:
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets --all-features -- -D warnings
- cargo test --workspace --all-features

If available and appropriate:
- cargo audit
but do not make it mandatory without verifying tooling availability.

Write `docs/security-model.md`.
```

## Acceptance gate

Trust boundaries and execution risks are explicit and tested.

---

# PHASE 15 — Release candidate verification

## Goal

Prove v0.1.0 instead of merely naming it v0.1.0.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 15 OBJECTIVE

Prepare v0.1.0 release candidate.

DO NOT CHANGE FEATURES unless a release-blocking bug is discovered.

Checklist:

1. Clean checkout validation.
2. Build release binary.
3. Run all quality commands.
4. Build healthy-v28 fixture with real Stellar tooling.
5. Run:
   soroban-upgrade-doctor scan fixtures/healthy-v28
6. Run:
   soroban-upgrade-doctor check fixtures/healthy-v28
7. Build before/after comparison fixtures.
8. Run compare and confirm expected breaking findings.
9. Run failing-tests fixture and confirm expected failure.
10. Open generated Markdown report and manually inspect for:
    - correct versions;
    - correct paths;
    - real evidence;
    - no fake success language;
    - no ANSI garbage.
11. Validate README commands by copy/pasting them into a clean shell.
12. Ensure SECURITY/CONTRIBUTING/LICENSE exist.
13. Update CHANGELOG with only features that actually exist.
14. Set crate/package version to 0.1.0 consistently.
15. Ensure `--version` reports 0.1.0.

Create `docs/release-checklist.md` containing the verified procedure.

Do not publish packages or create a Git tag unless explicitly instructed.

Final response must contain:
- exact commands run;
- exact pass/fail results;
- known limitations;
- git diff --stat;
- release readiness verdict.
```

---

# PHASE 16 — Drips/Wave repository polish

## Goal

Make the finished project contributor-friendly without fabricating activity.

## Codex prompt

```text
[PASTE GLOBAL INSTRUCTIONS FIRST]

PHASE 16 OBJECTIVE

Polish the existing finished v0.1.0 repository for outside contributors.

Do not add fake commits, fake contributors, fake adoption numbers, fake testimonials, or fake production claims.

Tasks:

1. README
   - concise problem;
   - clear screenshots/text output from REAL runs;
   - architecture;
   - quick start;
   - contribution entry point.

2. Issue templates
   - bug;
   - new rule;
   - documentation;
   - feature request.

3. PR template
   - summary;
   - tests;
   - rule evidence/reference when relevant;
   - checklist.

4. Labels documentation
   Proposed:
   - good first issue
   - help wanted
   - rule
   - wasm
   - cargo
   - protocol-28
   - testing
   - documentation
   - bug
   - enhancement

5. Roadmap
   Convert the roadmap into clearly scoped contributor tasks.
   Each task must contain:
   - problem;
   - scope;
   - acceptance criteria;
   - likely crate(s);
   - testing expectations.

6. Release notes draft for v0.1.0.
7. Confirm CI job names in README match actual YAML job IDs/names.
8. Verify every README link/file path exists.

Run all quality checks again.
```

---

# FINAL DEFINITION OF DONE

Codex must not mark the project complete until:

```text
[ ] cargo fmt passes
[ ] cargo clippy with -D warnings passes
[ ] cargo test workspace passes
[ ] real Stellar integration test passes in at least one verified environment
[ ] healthy v28 fixture passes
[ ] intentionally broken fixture fails for the expected reason
[ ] compare detects known breaking ABI/interface changes
[ ] Markdown report is correct
[ ] JSON report parses and contains schema_version
[ ] no production todo!/unimplemented!()
[ ] no fake success paths
[ ] no shell-string execution
[ ] target repo execution risk is documented
[ ] README quick start was manually verified
[ ] CONTRIBUTING.md exists
[ ] SECURITY.md exists
[ ] release checklist exists
[ ] version is consistent
```

---

# RECOMMENDED IMPLEMENTATION ORDER

Do not parallelize these until the interfaces are stable:

```text
1  scaffold
2  domain model
3  command runner
4  Cargo discovery
5  source rules
6  test/build gate
7  Wasm extraction
8  interface diff
9  storage analysis
10 reports
11 CLI UX
12 CI integration
13 docs
14 security hardening
15 release verification
16 Wave polish
```

This order is intentional.

The dangerous mistake would be to start by building a pretty CLI or dashboard before proving that the analyzers, external commands, Wasm inspection, and compatibility rules are correct.
