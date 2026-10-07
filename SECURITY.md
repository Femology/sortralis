# Security policy

## Status and scope

Sortralis is pre-alpha and **has not been audited**.
No stable release is currently supported. Security fixes will target the current
development version.

Passing Sortralis is not a security audit and does not prove that an upgrade is safe to deploy.

The CLI prints help/version and can detect environment tool versions. The core
provides data models and validated configuration reading. The runner executes
explicit requests directly with argument arrays, null stdin, and timeout handling.
Repository analysis, contract build/test execution, and Wasm parsing are not implemented.
No private keys or funded accounts are needed.

## Responsible disclosure

Do not publish exploit details, private source, credentials, or sensitive files in
a public issue. When private vulnerability reporting is enabled on GitHub, use the
repository's Security tab and "Report a vulnerability".

Otherwise, arrange a private reporting channel with the repository owner before
sharing sensitive details. No dedicated security inbox is configured in this
local scaffold; do not assume one exists.

Include the affected version, reproduction steps, expected and observed behavior,
and a minimal sanitized example. A maintainer will confirm the report and discuss
a fix and disclosure timing. There is no promised response deadline yet.

Only trusted executable paths/PATH should be supplied to the runner. The current
CLI requests only rustc, cargo, and stellar with --version. Timeout handling is
not a security sandbox: see [runner limitations](docs/runner.md). Future Cargo
builds/tests may execute code from target repositories and require trusted inputs.
