# Source storage inventory and comparison

Phase 7 inventories explicit Rust source observations. It does **not** establish
an exact on-ledger schema, execute target code, inspect runtime ledger entries,
expand macros, evaluate cfg, or decode XDR. Unchanged source types do not prove
that no migration is required. Passing Sortralis is not a security audit and does
not prove that an upgrade is safe to deploy.

## Usage

Supply two explicit directories containing source snapshots of the same package:

```bash
cargo run -p doctor-cli -- compare-source --before fixtures/storage/identical --after fixtures/storage/changed-field-type
cargo run -p doctor-cli -- compare-source --before fixtures/storage/identical --after fixtures/storage/removed-field --json
```

Paths are relative to each snapshot in the inventory and evidence. Keep snapshot
layouts consistent; moving/renaming a file can look like removal/addition because
source type identities include file and inline-module paths.

Repeat `--exclude vendor-extra` for relative path prefixes excluded on both sides.
The usual source exclusions also apply: target, vendor, generated, .git and
node_modules. Symlinks are skipped. `--sdk-crate sdk` selects a dependency alias
instead of the default soroban_sdk. No Cargo discovery, tests, build, target
scripts, or target modifications occur in this command.

Exit 0 means the comparison completed, including when review/migration records
exist. It does not certify compatibility. Invalid input/exclusions, unreadable
snapshots and parse errors return 2; serialization errors return 4. Existing
`check` policy is unchanged. Storage records are not yet integrated into that
policy. The original artifact `compare` remains unfinished.

The Rust API is `doctor_source::inventory_storage_text` or
`inventory_storage_directory`, followed by
`doctor_core::storage::compare_storage`. Core structures derive serde serialization
and deserialization; `doctor_report::write_storage_diff` renders human-readable
records and before/after locations. No full scan report files are generated.

## Verified SDK patterns

The exact locally installed soroban-sdk **28.0.0** source was inspected:
`src/env.rs`, `src/storage.rs` and `src/_migrating.rs`.
Authoritative tagged sources:

- [Env::storage](https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/env.rs)
- [Storage, Instance, Persistent and Temporary](https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/storage.rs)
- [SDK v28 migration behavior](https://github.com/stellar/rs-soroban-sdk/blob/v28.0.0/soroban-sdk/src/_migrating.rs)

An explicit SDK Env parameter, including direct SDK imports/aliases and reference
parameters, establishes a recognized receiver. Direct chains such as
`env.storage().persistent().set(&key, &value)`, and simple local Env/storage/
durability aliases, are recognized. Keyed operations are has/get/set/remove/
update/try_update and supported keyed TTL operations. Instance TTL is
contract-wide and is not treated as a storage key.

Key observations include explicit contracttype enum unit/tuple variants,
symbol_short! and Symbol::new literals, string-like literals/String::new,
primitive literals, module constants, and simple local key aliases. Enum variant
payloads containing runtime expressions remain dynamic key families; struct keys,
computed expressions and unresolved keys require review. Source key identities
are not serialized keys, and different source identities can alias at runtime.

Likely value types come from explicit function/local annotations, explicit
storage generic arguments, struct literals, casts, bool/char and suffixed integer
literals. Unsuffixed value numerals, closure return inference and general function
return types remain unknown. get without explicit value generics remains unknown
even if a surrounding expression could provide type context.

Explicit SDK contracttype structs/enums retain named/tuple/unit shapes, fields,
variants and explicit discriminants. Key/value associations and nested local
contract types are tracked separately from unrelated declarations. Unrelated
contracttype changes do not produce storage migration records. Association is
source evidence, not proof that a call executes or the value exists on ledger.

## Normalization and classification

Inventory schema version is 1.0, independent of the product version. Entries are
sorted by source key identity and durability, with grouped operations, value
spellings and file/line evidence. An unknown value observation is retained even
alongside a typed access. Types and uncertainty evidence are sorted deterministically.

| Change | Classification |
| --- | --- |
| Added known key with a typed write | INFO, usually non-breaking if independent; confirm initialization/independence |
| Added read-only key use or unknown value | REVIEW_REQUIRED |
| Removed key use | REVIEW_REQUIRED |
| Durability change | MIGRATION_LIKELY |
| Known value type change | MIGRATION_LIKELY |
| Storage-associated key enum/struct shape change | REVIEW_REQUIRED |
| Field added/removed/type changed in a stored contract type | MIGRATION_LIKELY |
| Dynamic/unknown keys, unresolved values/coverage | REVIEW_REQUIRED |
| Unchanged known observations | No diff within scope; no migration-safety assurance |

Dynamic keys require review even when source text is unchanged: their actual
runtime key set cannot be compared statically. Findings retain both sides'
evidence where available, stable record IDs, classifications, subjects and
human-readable summaries. JSON emits those same records, the scope statement,
and normalized before_inventory/after_inventory objects. Machine consumers can
inspect old/new field shapes by record subject without parsing summary text.

SDK v28 named contracttype map unpacking tolerates missing Option fields as None;
missing other fields error. Additional fields are ignored and discarded, and may
be lost on write-back. Packing is unchanged. These facts do not make a source field
addition/removal automatically migration-free. This phase uses MIGRATION_LIKELY
even for optional additions, as requested by its conservative gate. Named-map
behavior is not assumed for tuple structs or enums.

## Coverage limits and fixtures

No full Rust name/type resolver exists. Cross-file reexports, renamed imported
contract types, aliases, traits, helper functions, dynamic keys and macro-generated
calls/types need manual review. Explicit external/unresolved stored type spellings
generate uncertainty evidence. Modules with out-of-line files are scanned as files,
without claiming that their Rust bindings have been resolved. Inline modules are
scoped separately. Local shadowing, closures, loop/match patterns and assignments
invalidate unsupported inference rather than inheriting a misleading outer type.
Conditional bindings are conservatively unresolved. Macro bodies generate
coverage uncertainty. Excluded sources and symlinked sources are outside scope.

The parser may observe code Rust would reject or compile out; this command does
not establish build validity or prove exhaustive storage coverage. Type spelling
changes can require review even when semantic types are equivalent. Changing
unobserved source declarations may still affect storage through unsupported paths.

`fixtures/storage` contains parser-only snapshots (no downloads or compilation):
identical schema, added independent key, changed stored field type, removed field,
changed durability, dynamic key and unrelated contracttype. The fixture gate asserts
each expected classification. Additional tests cover removed keys, optional fields,
changed key variants, changed/unknown values, precise paths/lines, nested types,
all durability classes, aliases, exclusions, shadowing and JSON round trips.
