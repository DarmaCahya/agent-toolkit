<!--
Sync Impact Report (scratch; remove before commit)
- Version change: (unfilled template) → 1.0.0
- Modified principles: none renamed; all five template slots filled
- Added sections: Core Principles I–VI, Engineering Constraints, Development Workflow, Governance
- Removed sections: none
- Follow-up TODOs: none deferred. Note: current crates declare only `#![forbid(unsafe_code)]`;
  the lint gates in Principle II are not yet enforced in code (see Next Actions in the summary).
-->
# agent-toolkit Constitution

## Core Principles

### I. Pure Rust
All library code MUST be written in Rust. Library crates MUST NOT link to C/C++ libraries, shell
out to external binaries, or require system packages at build or run time. `unsafe` code MUST NOT
be used (`#![forbid(unsafe_code)]` in every crate root).
Rationale: a pure-Rust build is portable, auditable, and safe to embed in agent sandboxes.

### II. Zero Panics in Library Code (NON-NEGOTIABLE)
Library code MUST NOT panic on any input. Each library crate MUST deny `clippy::unwrap_used`,
`clippy::expect_used`, `clippy::indexing_slicing`, and `clippy::panic`. Fallible operations MUST
return `Result`; slice access MUST use `get`, iterators, or pattern matching. Arithmetic that can
overflow MUST use checked or saturating forms. Tests, examples, and benches MAY relax these lints
locally.
Rationale: callers are autonomous agents processing untrusted input; a panic is an outage.

### III. Deterministic Output
The same input and options MUST produce byte-identical output across runs and platforms. Library
code MUST NOT depend on wall-clock time, randomness, environment, locale, or hash-map iteration
order; ordered collections or explicit sorting MUST be used wherever order affects output.
Rationale: reproducible output enables snapshot testing and trustworthy agent workflows.

### IV. Test-First with User-Scenario Tests (NON-NEGOTIABLE)
Tests MUST be written before implementation and MUST be seen failing first (red-green-refactor).
Every feature MUST have tests that express a real user scenario through the public API (an agent
or developer performing a task end to end), in addition to unit tests. Bug fixes MUST begin with
a failing regression test.
Rationale: scenario tests guard behavior users depend on, not implementation details.

### V. Typed, Non-Exhaustive Errors
Public error types MUST be defined with `thiserror` and MUST be annotated `#[non_exhaustive]` so
variants can be added without a breaking change. Errors MUST carry actionable context (what
failed and, where possible, how to fix it). Library code MUST NOT use `anyhow`, `Box<dyn Error>`,
or stringly-typed errors in public signatures.
Rationale: agents recover from failures only when errors are structured and stable.

### VI. Minimal Dependencies
Every new dependency MUST be justified in the PR that adds it (what it replaces, why it cannot be
reasonably written in-tree). Dependencies MUST be pure Rust, actively maintained, license
compatible with MIT OR Apache-2.0, and use the narrowest version requirement and feature set that
works so downstream lockfiles are not constrained unnecessarily.
Rationale: each dependency adds supply-chain risk, compile time, and downstream conflicts.

## Engineering Constraints

- Workspace edition is Rust 2021; crates are dual-licensed MIT OR Apache-2.0.
- Public API changes MUST follow semantic versioning per crate.
- Untrusted input (including non-rendering Unicode and tag code points) MUST be sanitized or
  rejected at the pipeline boundary, never passed through silently.

## Development Workflow

- A change is mergeable only when `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  and `cargo test` pass for the affected crates.
- Work follows the Spec Kit flow: specify → plan → tasks → implement; tests precede implementation
  tasks.
- Every PR MUST state which principles it touches and justify any exception.

## Governance

This constitution supersedes other practices. Amendments require a pull request that edits this
file, states the rationale, and includes a migration plan for any code that no longer complies.
Versioning follows semantic versioning: MAJOR for removed or redefined principles, MINOR for
added principles or materially expanded guidance, PATCH for clarifications. Reviewers MUST verify
compliance on every PR; unjustified violations block merge. Exceptions MUST be documented inline
with a reason (e.g., `#[allow(...)] // reason`).

**Version**: 1.0.0 | **Ratified**: 2026-11-08 | **Last Amended**: 2026-11-08
