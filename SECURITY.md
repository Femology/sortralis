# Security Policy

## Project status

Sortralis v0.1.0 is the current public release and has not been audited. Security fixes target the latest supported release and the current development version on `main`.

Passing Sortralis is not a security audit and does not prove that a contract upgrade is safe to deploy.

## Scope

This policy covers security issues in the Sortralis CLI and its `doctor-*` crates, including issues such as:

- unsafe handling of malicious repository paths or source inputs;
- command-execution or argument-injection flaws;
- report-generation injection issues;
- unsafe Git operations;
- unintended exposure of secrets or sensitive source content;
- denial-of-service behavior caused by Sortralis itself.

A missed Soroban migration rule or false positive may be an analyzer bug without necessarily being a vulnerability in Sortralis.

## Current safety model

Sortralis is local-first. It does not require a funded account or private key for normal analysis.

External processes are launched with separate program arguments rather than shell command strings. Timeouts and captured stdout/stderr are used where implemented, but subprocess execution is **not** an operating-system sandbox.

The `check` and verification workflows may invoke Cargo and Stellar build/test tooling for the target repository. Cargo build scripts and tests are executable code, so only run full verification against repositories you are willing to execute locally.

HTML reports escape repository-controlled strings and are designed to be self-contained without remote scripts or trackers.

## Responsible disclosure

Do not publish exploit details, credentials, private source, or other sensitive material in a public GitHub issue.

If GitHub private vulnerability reporting is enabled for this repository, use the repository's **Security** tab and **Report a vulnerability** flow.

If private vulnerability reporting is not available, contact the repository owner through GitHub first to establish a private reporting channel before sharing sensitive details. No dedicated security email address is currently documented, so do not assume one exists.

A useful report should include:

- affected commit/version;
- minimal reproduction steps;
- expected and observed behavior;
- impact;
- a sanitized proof of concept where possible.

There is currently no guaranteed response-time SLA.
