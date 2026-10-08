## Summary

Describe the change and why it belongs in Sortralis.

## Evidence

For Stellar/Soroban-specific behavior, link the authoritative documentation, exact crate source/docs, or verified CLI output used to justify the change.

## Testing

List the exact commands you ran and their results.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p doctor-cli -- --help
```

Add focused positive and negative tests for new/changed rules. Do not weaken legitimate failing tests.

## Safety and compatibility checklist

- [ ] I did not introduce shell-string command construction.
- [ ] I preserved typed error handling in production paths.
- [ ] I did not add `todo!()` or `unimplemented!()` to production code.
- [ ] I documented uncertainty instead of claiming safety where the analyzer cannot prove it.
- [ ] I updated user-facing documentation when behavior or CLI output changed.
- [ ] I staged only files belonging to this logical change.
- [ ] I did not rewrite shared Git history.

## User-visible impact

Describe any rule IDs, exit codes, report schema, CLI behavior, or documentation changes users should know about.
