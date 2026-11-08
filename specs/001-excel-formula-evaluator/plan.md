# Implementation Plan: Deterministic Excel Formula Evaluator

**Branch**: `feat/excel-evaluator-issue-9` (spec dir `001-excel-formula-evaluator`) | **Date**: 2026-11-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-excel-formula-evaluator/spec.md`

## Summary

Add a pure-Rust, panic-free, deterministic Excel formula evaluator to the existing `agent-canvas` crate at `crates/agent-canvas/src/math/excel/`, exported from `math/mod.rs`. The pipeline is: length check → sanitize → lexer → Pratt parser (depth-bounded) → AST → tree-walking evaluator (depth-bounded, lazy `IF`/`IFERROR`). Results are a four-kind `Value` (Number/Bool/Text/Error); every failure is an `Error` value, never a Rust `Err` or panic. No new runtime dependencies; `proptest` is added as a dev-dependency only, for never-panics property tests.

## Technical Context

**Language/Version**: Rust 2021 (workspace edition); no MSRV bump

**Primary Dependencies**: none new at runtime (existing `thiserror`, optional `serde` untouched). Dev-only: `proptest` (narrow version requirement, `default-features = false` + `std` where possible to limit lockfile impact)

**Storage**: N/A

**Testing**: `cargo test -p agent-canvas`; unit tests per module, user-scenario tests in `crates/agent-canvas/tests/excel_tests.rs`, property tests in `crates/agent-canvas/tests/excel_proptest.rs`

**Target Platform**: any Rust target (pure Rust, no I/O, no time/random/env)

**Project Type**: library module inside the `agent-canvas` crate

**Performance Goals**: single formula within limits < 10 ms (SC-005); expected microseconds

**Constraints**: input ≤ 8192 chars, nesting ≤ 64, zero panics, byte-identical output across platforms, `#![forbid(unsafe_code)]`

**Scale/Scope**: 13 functions, 6 comparison ops, ~12 operator forms; ~1.2k LOC estimated

No NEEDS CLARIFICATION remain; the deferred items (range hook shape, number formatting) are resolved in [research.md](research.md).

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design.*

| Principle | Status | How satisfied |
|---|---|---|
| I. Pure Rust | PASS | No C/FFI/external binaries; `forbid(unsafe_code)` in every new file. |
| II. Zero panics | PASS (scoped exception, documented) | New module declares `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]`; slices via `get`/iterators; checked depth counters; no `as` casts that can wrap on user data (explicit saturating conversions for `ROUND` digits). Exception: the constitution says each library *crate* MUST deny these lints, but the crate root does not yet enforce them and turning them on crate-wide would touch unrelated modules. They are applied at module level here (does not change other modules); the PR description (T045) MUST state this exception and its reason, and a crate-wide follow-up is recommended. |
| III. Deterministic | PASS | No time/random/env/locale; no `HashMap` in output paths (function table is a `match` on uppercased name); number formatting is hand-specified (research D5), not dependent on platform libm for display. |
| IV. Test-first + scenarios | PASS (process) | Tasks must order tests before implementation; user-scenario tests through the public `ExcelEvaluator::eval`. |
| V. Typed non-exhaustive errors | PASS with note | Evaluation errors are *values* (`ExcelError`, `#[non_exhaustive]`, derives `thiserror::Error` for `Display`/`Error`), not a `Result` error channel, per spec FR-003. The `RangeProvider` hook is infallible by design (returns `Option<Vec<Cell>>`). Actionable context is carried by the fixed Excel error code plus the error-mapping table in the contract; unit variants are deliberate because the codes are the Excel-compatible contract. |
| VI. Minimal deps | PASS | Zero runtime deps added. `proptest` is dev-only; justification: the spec's SC-004 (≥100k arbitrary inputs) cannot reasonably be hand-rolled with shrinking; test-only so it never reaches downstream lockfiles. |

No violations → Complexity Tracking not needed.

**Post-design re-check**: unchanged; design adds no dependency, no unsafe, no shared mutable state. PASS.

## Project Structure

### Documentation (this feature)

```text
specs/001-excel-formula-evaluator/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── rust-api.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/agent-canvas/
├── Cargo.toml                    # + [dev-dependencies] proptest
├── src/math/
│   ├── mod.rs                    # + pub mod excel; pub use excel::{...}
│   └── excel/
│       ├── mod.rs                # ExcelEvaluator, limits, module lint attrs, re-exports
│       ├── value.rs              # Value, ExcelError, display formatting, coercion
│       ├── lexer.rs              # sanitize + tokenizer
│       ├── parser.rs             # Pratt parser -> Expr, depth-bounded
│       ├── eval.rs               # tree-walk evaluator, depth-bounded, lazy IF/IFERROR
│       ├── functions.rs          # 13 Phase-1 functions
│       └── compare.rs            # 15-digit equality, cross-type ordering
└── tests/
    ├── excel_tests.rs            # user-scenario + quirk table + corpus
    └── excel_proptest.rs         # never-panics / bounded-behavior properties
```

**Structure Decision**: Single-module addition inside the existing library crate, following the flat `math/*` convention but as a directory because it has several cohesive files. The issue names `parser.rs` and `eval.rs`; the rest are splits to keep files small. No other crate changes.

## Complexity Tracking

None.
