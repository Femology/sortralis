# Safe Git source comparison (Phase 8)

`cargo run -p doctor-cli --bin sud -- diff --repo PATH --from REF --to REF`
prints a terminal summary. Add `--json` for the complete deterministic model.
The existing Sortralis commands and default Cargo binary remain available.

## Safety boundary

Refs are passed as separate arguments to `git rev-parse --verify --end-of-options`
with `^{commit}` peeling. Empty refs, option-like refs and control characters are
rejected; a single full commit OID is required. Git `ls-tree` and raw `cat-file
blob` read immutable objects. No checkout, switch, reset, stash, registered
worktree, hooks, clean/smudge filter or target build script is used. Replacement
objects, fsmonitor, untracked caches, optional locks and pagers are disabled.
Partial/promisor clones are rejected to avoid lazy fetching through remote
helpers. Repository redirect environment variables are rejected explicitly.

Dirty detection reads the index and HEAD tree, untracked names and raw file
hashes with `hash-object --no-filters` (without writing objects). It does not use
`git status` or refresh the index. Raw line-ending/filter differences, symlinks
and submodules may conservatively count as dirty. Dirty state does not block
comparison. The integration gate writes uncommitted binary sentinel bytes and
tracked edits, then checks branch, HEAD, index, registered worktrees and exact
file bytes after real CLI comparisons. Comparison never includes those edits.

## Temporary resources

`tempfile` 3.27.0 creates a uniquely named private `sortralis-git-*` directory in
the OS temporary directory, with independent `from` and `to` object snapshots.
A temporary directory inside the active worktree is rejected. Explicit cleanup
runs on success and ordinary snapshot/analysis errors. Cleanup failures preserve
the original error and identify the directory; Drop is a best-effort fallback.
Process kill, abort or power loss can bypass cleanup and leave that unregistered
folder. Remove the reported/recognized folder manually after the process ends;
there is no Git worktree registration to prune.

Only normal UTF-8 tree paths and regular blobs are supported. Symlinks,
submodules and unsafe paths fail clearly; they are never followed/materialized.
Limits per ref: 10,000 tree entries, 8 MiB per blob and 64 MiB total. Executable
permissions are not recreated because snapshots are not executed. `.cargo`
directories and `rust-toolchain`/`rust-toolchain.toml` files are omitted so target
Cargo/rustup configuration cannot select wrappers or other executables. Explicit dependency, target and workspace paths escaping the
snapshot are rejected before Cargo metadata; metadata paths are bounded again.
Consequently external path dependencies are unsupported, and Cargo configuration
changes are outside this comparison. The installed Cargo and user's trusted
global tool configuration remain part of the execution environment.

## Analysis and interpretation

Each snapshot discovers Cargo packages with offline `cargo metadata
--format-version 1 --no-deps`. It uses normal target source roots (excluding
build scripts, tests, examples and benches) and existing exclusions. Multiple
contract candidates are identified through normal `soroban-sdk` dependencies,
including inherited/aliased requirements. Requested SDK requirements and SDK
versions recorded anywhere in the workspace lockfile are distinguished; lock
records are not attributed as proven package resolution. Unknown target SDK
context is shown and rules requiring v28 are not silently assumed applicable.

Source inventories retain explicit SDK `contractimpl` methods/signatures,
`contracttype` struct/enum shapes and `contractevent` declarations, including
event topic/data attributes. Public or explicit trait implementation methods
are interface-like observations, not compiled exports. Function bodies and
source line movement alone do not count as function changes. Type identities
include source path/module, so moves can appear as removal/addition. Duplicate
export names are retained as grouped observations. Findings are compared by
package and stable rule ID; changed evidence is reported as changed rather than
claiming a new rule. Package matching uses relative manifest paths.

The analysis does not expand macros, resolve arbitrary imports/types, evaluate
cfg/features, compile contracts, infer legacy/dynamic event publication shapes,
or extract Wasm specifications. Trait defaults and item macros produce explicit
uncertainty observations. Event inventory covers explicit contractevent structs;
legacy `events().publish` payload schemas require manual review. Storage uses
Phase 7 conservative classifications and its documented limitations. Unchanged
source never proves that no migration is required. This is not a security audit
or a deployment-safety guarantee.

Exit 0 means a comparison completed, including one reporting migration risks.
Invalid refs/config/source or unsupported snapshots exit 2; external tool/cleanup
failures exit 3; invalid tool output exits 4. Diff does not enforce `fail_on` or
execute `run_tests`/`build_contracts`; use `check` for the execution/policy pipeline.
No later phase is implemented.

## Verified references

- [Git ref validation](https://git-scm.com/docs/git-rev-parse),
  [tree records](https://git-scm.com/docs/git-ls-tree),
  [raw blobs](https://git-scm.com/docs/git-cat-file), and installed Git 2.43.0 help.
- [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)
  and installed cargo_metadata 0.23.1 `Target.src_path` and target-kind methods.
- [Cargo workspace paths](https://doc.rust-lang.org/cargo/reference/workspaces.html),
  [dependency paths](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html),
  [target paths](https://doc.rust-lang.org/cargo/reference/cargo-targets.html),
  and [lockfile records](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html).
- Installed soroban-sdk 28.0.0 `src/lib.rs` macro documentation; fixture 27.0.0
  verified through `cargo info soroban-sdk@27.0.0`. Source inventories do not call
  SDK APIs or construct XDR/Wasm specifications.
- Installed tempfile 3.27.0 `Builder::prefix`, `tempdir`, `TempDir::close` and Drop
  implementation; exact dependency/version/license verified with Cargo.
