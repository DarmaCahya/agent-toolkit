# Implementation Plan: LaTeX Math Typesetter (Vector SVG)

**Branch**: `feat/latex-typesetter` (spec dir `002-latex-math-typesetter`) | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-latex-math-typesetter/spec.md`

## Summary

Add a pure-Rust LaTeX math-mode → vector-SVG typesetter at `crates/agent-canvas/src/math/latex/`.
Pipeline: `lexer → parser (AST) → atoms (TeX classes, Bin demotion, spacing) → box_model (TeX
Appendix G layout driven by the font's OpenType MATH table) → render (deduped glyph `<path>`
defs + `<use>` + rule `<rect>`/paths)`. Glyph outlines and MATH data come from an embedded,
renamed STIX Two Math subset read with `ttf-parser`. `MathTypesetter` and `MathError` are
re-exported from `math/mod.rs`.

## Technical Context

**Language/Version**: Rust 2021 (workspace edition); no MSRV change.

**Primary Dependencies**: `thiserror = "2"` (already in crate), `ttf-parser = "0.25"` with
`default-features = false, features = ["opentype-layout"]` (adds MATH table access; 0.25.1 is
already in `Cargo.lock` via another crate, so no new lockfile package). Verify at task time whether
`std` must be added for float math (see research R2).

**Storage**: Embedded font bytes via `include_bytes!` from `crates/agent-canvas/assets/` (no runtime files). No `build.rs`.

**Testing**: `cargo test` — unit tests per module; integration tests in `crates/agent-canvas/tests/`; dev-deps `proptest` (already present) and `insta` (new, dev-only).

**Target Platform**: Any Rust target (pure Rust, no C, no I/O); deterministic across platforms.

**Project Type**: Library module inside existing crate `agent-canvas`.

**Performance Goals**: ≤200-char formula < 50 ms; 8192-char input renders or errors < 2 s (SC-005).

**Constraints**: No panics (module-level `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]`); `forbid(unsafe_code)` already crate-wide; font asset < 250 KB; byte-identical output; input ≤ 8192 chars; nesting ≤ 64.

**Scale/Scope**: v1 grammar of FR-007–FR-009 (letters, digits, Greek, a few operators, `( ) ,`, groups, scripts, `\frac`, `\sqrt[n]{}`, `\sum \int \prod`).

**Open item (not blocking design)**: no local path for the STIX Two Math source has been established (spec Assumptions defers to maintainers) and none was found by the planning pass. The user must supply the font location (or permit download from the upstream STIX repo) before the font task (US4) can run. See research R6.

## Constitution Check

| Principle | Status | How the plan satisfies it |
|---|---|---|
| I. Pure Rust | PASS | `ttf-parser` is pure Rust; subset script is dev-only tooling (Python `fonttools` via `uvx`), not used at build/run time; no `build.rs`. |
| II. Zero panics | PASS (needs care) | Module-level deny lints in `math/latex/mod.rs`; `get`/iterators only; checked/saturating arithmetic on positions and sizes; proptest never-panics test. |
| III. Deterministic | PASS | No hash maps in output paths (`BTreeMap` for glyph defs); fixed-precision number formatting (2 dp) via integer-rounded formatter; no env/time. |
| IV. Test-first | PASS | Tasks order tests (red) before each module; user-scenario tests through `MathTypesetter::render_to_svg`. |
| V. Typed errors | PASS | `MathError` via `thiserror`, `#[non_exhaustive]`, carries position + hint. |
| VI. Minimal deps | PASS (justify in PR) | One new runtime dep (`ttf-parser`, narrowest features, `"0.25"`), one new dev-dep (`insta`). Rationale: parsing OpenType glyf/MATH tables in-tree is large and error-prone. |

Engineering constraint "untrusted input incl. non-rendering/tag code points rejected": covered by lexer rejecting any char outside the supported set with a positioned error.

No violations → Complexity Tracking not needed. **Post-design re-check: PASS** (no design choice added a dependency, `unsafe`, or build script).

## Project Structure

### Documentation (this feature)

```text
specs/002-latex-math-typesetter/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── public-api.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/agent-canvas/
├── Cargo.toml                       # + ttf-parser, + insta (dev)
├── assets/
│   ├── fonts/AgentCanvasMath-subset.otf   # committed subset (<250 KB)
│   ├── fonts/OFL-1.1.txt                  # licence text
│   └── ...
├── src/math/
│   ├── mod.rs                       # + `pub mod latex;` + `pub use latex::{MathTypesetter, MathError};`
│   └── latex/
│       ├── mod.rs                   # MathTypesetter, limits, deny-lints, pipeline glue
│       ├── error.rs                 # MathError (thiserror, non_exhaustive)
│       ├── lexer.rs                 # chars → tokens with char positions; whitelist
│       ├── parser.rs                # tokens → AST; depth limit; script/arg errors
│       ├── atoms.rs                 # AST → classed atoms; Bin demotion; inter-atom spacing table; Style
│       ├── font.rs                  # include_bytes, ttf-parser Face, MATH constants/variants/italic corr., outlines
│       ├── box_model.rs             # layout: Box tree, Rules 11/13/15/18, radical stretch+scale fallback
│       └── render.rs                # Box tree → deterministic SVG string
├── tests/
│   └── latex_typesetter.rs          # user-scenario, golden bbox, snapshot, proptest
scripts/subset-font.sh               # repo root; regenerates subset via uvx fonttools
THIRD_PARTY_NOTICES.md               # repo root (decided)
```

**Structure Decision**: Single library module matching the user-specified file split under `math/latex/`. `error.rs` is added beside the requested files so `MathError` has one home (re-exported from `math/mod.rs` as requested). The existing `canvas/font.rs` is unrelated (text metrics) and is not reused; the new `font.rs` is private to `latex`.

## Complexity Tracking

No constitution violations to justify.
