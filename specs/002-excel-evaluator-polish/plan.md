# Implementation Plan: Excel Evaluator Polish

**Branch**: `fix/excel-evaluator-polish` (spec dir `002-excel-evaluator-polish`) | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-excel-evaluator-polish/spec.md`

## Summary

Four polish items on the merged Excel evaluator: (1) structured, located parse errors plus a fallible
entry point (the infallible one stays total); (2) `Value::as_str()` becomes text-only, the coercing
behaviour moves to `to_text()`, and `Display` is added; (3) first-class `Value::Blank` with Excel
coercion; (4) function arity validated at parse time. Approach: give tokens source spans, make
lexer and parser return a typed `ParseError`, move arity checking into the parser, add `Blank`
handling in coercion/compare/eval. Pure in-crate Rust; `thiserror` is already a dependency.

## Technical Context

**Language/Version**: Rust 2021 (workspace edition)
**Primary Dependencies**: none added. Existing: `thiserror 2`, optional `serde`; dev: `proptest`
**Storage**: N/A
**Testing**: `cargo test -p agent-canvas` (module unit tests, `tests/excel_tests.rs`, `tests/excel_proptest.rs`, doc tests, examples compile)
**Target Platform**: any (pure Rust, deterministic)
**Project Type**: library crate (`agent-canvas`), module `src/math/excel`
**Performance Goals**: unchanged; span tracking is O(n) with input capped at 8192 chars
**Constraints**: no panics (`unwrap/expect/indexing/panic` denied in the module); deterministic; existing length/depth limits
**Scale/Scope**: ~1750 lines in 7 files plus tests, README, one example

**Allowed change surface** (user directive): `crates/agent-canvas/src/math/excel/**`, `crates/agent-canvas/tests/**`, `crates/agent-canvas/README.md`, `crates/agent-canvas/examples/**`. The re-export in `src/math/mod.rs` and the `Cargo.toml` version bump are outside it; see Open Items.

## Constitution Check

| Principle | Status | Notes |
|---|---|---|
| I Pure Rust | PASS | Rust only |
| II Zero panics | PASS | module already denies the four lints; span math uses `get`/saturating ops |
| III Deterministic | PASS | no time/hash/locale; `expected` lists built in fixed order |
| IV Test-first | PASS (enforced in tasks) | tasks must put failing tests before each implementation step; scenario tests via public API |
| V Typed non-exhaustive errors | PASS | `ParseError`/`ParseErrorKind` via `thiserror`, both `#[non_exhaustive]` |
| VI Minimal deps | PASS | zero new dependencies |
| Semver | ATTENTION | `as_str` signature change and `Value` growth are breaking: minor bump on 0.x plus changelog (Open Items) |

No violations; Complexity Tracking not needed.

## Project Structure

### Documentation

```text
specs/002-excel-evaluator-polish/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/public-api.md
└── tasks.md            # generated; next: /speckit-implement
```

### Source Code

```text
crates/agent-canvas/
├── src/math/excel/
│   ├── mod.rs        # try_eval/try_eval_with, evaluate() -> Result, offset mapping, docs
│   ├── error.rs      # NEW: ParseError, ParseErrorKind (thiserror, non_exhaustive)
│   ├── lexer.rs      # tokens carry byte spans; typed errors
│   ├── parser.rs     # typed errors; expected items; arity check when a Call is built
│   ├── functions.rs  # arity table (variadic min 1); runtime guard kept as defence in depth
│   ├── value.rs      # Blank, as_str/to_text/Display, internal coercion helpers
│   ├── compare.rs    # Blank vs Number/Text/Bool/Blank
│   └── eval.rs       # single empty cell -> Blank
├── tests/excel_tests.rs, tests/excel_proptest.rs
├── examples/issue-009-demo/main.rs
└── README.md
```

**Structure Decision**: single library crate; one new file `error.rs` keeps the error type out of `value.rs`.

## Design Decisions (detail in [research.md](research.md))

1. **Spans**: lexer emits byte spans into the sanitized body; `evaluate()` keeps a body-offset to original-offset table (accounts for stripped invisible chars, trimming, and the leading `=`), so reported offsets refer to the caller's string. `=SUM(` gives offset 5, empty span.
2. **Expected items**: static labels (`"expression"`, `")"`, `","`, ...) in fixed order; after `SUM(` at end of input: `["expression", ")"]`.
3. **Arity at parse time**: checked when a `Call` is built, known names only, case-insensitive; unknown names stay `#NAME?` at eval.
4. **Blank**: `Value::Blank`; arithmetic and numeric compare read 0, `&`/CONCAT/text compare read "", logical reads FALSE, public `to_text()`/`Display` give "0"; a single empty cell evaluates to Blank.
5. **Limits**: length/depth overflow is `ParseErrorKind::LimitExceeded`; infallible path still yields `#VALUE!`.
6. **Total path**: `eval*` = `try_eval*` with any `Err` mapped to `Value::Error(ExcelError::Value)`.

## Open Items (user decision needed; not blocking Phase 1)

- **Re-export**: `math::excel` is already `pub`, so the new types are reachable as `agent_canvas::math::excel::{ParseError, ParseErrorKind}` via `excel/mod.rs` with no change outside the surface. Convenience re-export from `src/math/mod.rs` (next to `ExcelError`) is a one-line change outside the surface; not planned unless you approve it.
- **Version bump and changelog** (`Cargo.toml`, no CHANGELOG file exists) are outside the surface; the spec calls for them. Deferred.

## Post-Design Constitution Re-check

All principles still PASS; Principle IV is enforced by task ordering in `tasks.md`.
