# Source rules (Phase 5)

The engine is a library, with documentation exposed via `sortralis explain <RULE_ID>`.
`scan` preserves Phase 4 Cargo discovery; Phase 6 `check` integrates the source
engine before tests and builds. No source writes or external processes occur.

## Verified sources

- [SDK v28 migration guide](https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs):
  exact published SDK 28.0.0 source downloaded in Phase 4 and inspected again.
  Tagged GitHub source URL verified through the GitHub contents API.
- [SDK custom-account interface](https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/auth.rs):
  installed v28 source inspected for the trait and method signature used in fixtures.
- [syn 3.0.6](https://docs.rs/syn/3.0.6/syn/): published version and installed
  parse_file, File, UseTree, Attribute::parse_args_with, Meta, Punctuated, and Visit
  APIs verified before use. Full/parsing/visit/printing features are enabled.
- [proc-macro2 1.0.107](https://docs.rs/proc-macro2/1.0.107/proc_macro2/struct.Span.html#method.start):
  installed span-locations source confirms stable line positions outside a
  procedural macro. This CLI/library parses source outside procedural macros.
  Both parser dependencies require Rust 1.71; development uses 1.96.0.

The migration guide confirms the export removal and executable authorization change.
It describes sparse map events, but deciding whether consumers need a change
requires semantics beyond source syntax. This phase registers that review manually.

## Registry and rules

`SourceRule` provides documentation (stable ID, description, context, severity,
recommendation, references, limitations) and independent `analyze` behavior.
`registry()` contains two analyzers and a manual entry whose analyzer is `None`.
Manual registration does not return a dummy finding or mark an analysis successful.

| ID | Severity | Evidence |
| --- | --- | --- |
| SDK28_REMOVED_EXPORT_ARGUMENT | Breaking | export key in SDK contracttype/contracterror attributes |
| CUSTOM_ACCOUNT_EXECUTABLE_REVIEW | ManualReview | implemented __check_auth method identifier |
| SDK28_EVENT_SHAPE_REVIEW | ManualReview | manual documentation only; no static evidence |

All automatic rules require explicit `TargetContext::Sdk28`, representing use of
or planned migration to SDK v28. Other/unknown contexts yield no rule findings;
absence of findings does not establish compatibility. The engine does not infer
SDK versions from source identifiers or broad semver ranges.

## Parsing and scope

No regex or text-search detector is used. Comments, strings, calls, unrelated
attributes and opaque macro bodies are not interpreted as implementation code.
Export detection recognizes explicit SDK qualification, module-level direct
imports/aliases (including explicit SDK extern-crate aliases), and unambiguous SDK wildcard imports. Non-SDK explicit imports
shadow wildcard names. Ambiguous wildcards and local macro definitions are treated
conservatively. Inline module import scopes are separate. Cargo dependency aliases
can be passed using `SourceOptions.sdk_crate_names` (Rust identifier spelling).

This is not a compiler or Rust name resolver. No macro expansion, cfg/cfg_attr
evaluation, cross-file reexport resolution, function-local import resolution,
or semantic type checking occurs. Function bodies are not inspected for locally
defined annotated types. Conditional items are source evidence; confirm they are
active before applying migration changes. A named impl method is a review trigger,
not proof of exported custom-account behavior or a broken authorization policy.

`analyze_text` preserves the caller's evidence path. `analyze_directory` canonicalizes
its package root and records absolute paths and one-based lines. Findings group all
locations once per rule, with deterministic sorting and identical evidence deduplicated.
Read and parse failures are typed errors including their path (and parse line),
not silently skipped files. These APIs neither load CLI configuration nor evaluate
exit policy; callers supply target context and options explicitly.

## Traversal and fixtures

Default ignored directory components: target, vendor, generated, .git, node_modules.
These defaults cannot be overridden. Custom exclusions are nonempty relative path
prefixes; single components exclude that name anywhere. Absolute paths, parent
traversal and leading dot components are rejected. Globs are not supported. Symlink entries
are skipped, including directory cycles. This is file traversal, not an OS sandbox.

`fixtures/source` contains legacy export arguments, healthy-v28 syntax, a custom
account and an ordinary contract. They are source-parser fixtures, not compiled
contract artifacts. The custom-account body panics intentionally: it is never
executed and must not be deployed. Tests cover positive/negative patterns, stable
IDs, locations, grouping, aliases, exclusions, parse errors and symlinks. The manual
event entry is tested for documentation availability and absence of fabricated
findings on either changed-shape or explicit opt-out source.
