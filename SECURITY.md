# Security policy

## Status and scope

Soroban Upgrade Doctor is pre-alpha and **has not been audited**.
No stable release is currently supported. Security fixes will target the current
development version.

Passing Soroban Upgrade Doctor is not a security audit and does not prove that an upgrade is safe to deploy.

Phase 1 only parses CLI commands and prints help/version or explicit unavailable
command errors. Repository analysis, build/test execution, and Wasm parsing are
not implemented. No private keys or funded accounts are needed.

## Responsible disclosure

Do not publish exploit details, private source, credentials, or sensitive files in
a public issue. When the project is hosted on GitHub and private vulnerability
reporting is enabled, use the repository's Security tab and "Report a vulnerability".

Otherwise, arrange a private reporting channel with the repository owner before
sharing sensitive details. No dedicated security inbox is configured in this
local scaffold; do not assume one exists.

Include the affected version, reproduction steps, expected and observed behavior,
and a minimal sanitized example. A maintainer will confirm the report and discuss
a fix and disclosure timing. There is no promised response deadline yet.

Future phases that execute Cargo builds/tests will need to document the trust
boundary: build scripts and tests can execute code from the target repository.
