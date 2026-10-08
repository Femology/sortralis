# Architecture

Sortralis is designed around a modular workspace containing several focused crates:

## Core Crates

1. **`doctor-core`**: Contains all foundational types, traits, and schemas. This includes the unified `Report` structure, rule severities, finding representations, and storage diff definitions.

2. **`doctor-cli`**: The entrypoint and user interface. It parses command-line arguments utilizing `clap`, orchestrates the execution of different analysis phases, formats the output (Terminal, JSON, HTML, SARIF), and exits with accurate codes.

3. **`doctor-source`**: The static analysis engine for Rust code. 
   - Uses the `syn` crate to parse Soroban contract source files into an Abstract Syntax Tree (AST).
   - Rules implement the `Rule` trait and use `syn::visit` to walk the AST and record violations.
   - Responsible for deep inspections like API deprecation, struct field mutations, and authentication patterns.

4. **`doctor-wasm`**: The binary analysis engine.
   - Inspects compiled Wasm artifacts by invoking the Stellar CLI to extract environment metadata.
   - Decodes protocol versions, exported functions, and custom sections to determine interface compliance.

5. **`doctor-runner`**: The orchestration and discovery crate.
   - Discovers Cargo projects and specific `soroban-sdk` dependencies within a workspace.
   - Detects the local environment state (installed `rustc`, `cargo`, `stellar` CLI tools) and their versions.

6. **`doctor-report`**: The formatting engine.
   - Translates internal core structures into human-readable terminal output.
   - Formats ANSI terminal coloring and builds the final unified HTML, SARIF, and JSON representations.
