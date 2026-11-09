# Implementation Plan: LaTeX Explicit Spacing Commands

**Branch**: `feat/latex-spacing` (existing; spec dir `003-latex-spacing-commands`) | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-latex-spacing-commands/spec.md`

## Summary

Add `\,` `\:` `\;` `\!` `\quad` `\qquad` and backslash-space to the LaTeX typesetter. Spacing is
modelled as *glue*: a new AST node and atom variant that carries a signed width (mu-based or
em-based), has no atom class, and is skipped by Bin demotion, inter-atom spacing lookup and
previous-class tracking. Layout adds the width via `LBox::advance`; mu spaces scale with the style
(`Style::mu_scale`), em spaces do not. Root width is clamped to >= 0. Spacing-only formulas return
`EmptyFormula`.

## Technical Context

**Language/Version**: Rust 2021 (workspace edition)

**Primary Dependencies**: none new (existing `thiserror`, `ttf-parser`); no `Cargo.toml` changes

**Storage**: N/A

**Testing**: `cargo test -p agent-canvas` (unit, integration `tests/latex_*.rs`, `insta` snapshots, `proptest`)

**Target Platform**: any (pure Rust library)

**Project Type**: library crate (`crates/agent-canvas`)

**Performance Goals**: no regression; spacing is O(1) per item

**Constraints**: deterministic byte-identical SVG; zero panics (clippy deny lints); existing
`MAX_INPUT_CHARS`/`MAX_DEPTH` still apply; output for all previously valid formulas byte-identical

**Scale/Scope**: `lexer.rs`, `parser.rs`, `atoms.rs`, `box_model.rs`, small change in `mod.rs`
(empty check, width clamp), plus tests, README, specs. `error.rs` and `Cargo.toml` untouched.

## Constitution Check

| Principle | Status |
|---|---|
| I Pure Rust, no unsafe | Pass |
| II Zero panics | Pass: only `get`/f64 arithmetic; new code stays under the existing clippy denies |
| III Deterministic | Pass: constants only, no maps; covered by a byte-identity test (FR-009) |
| IV Test-first, user-scenario tests | Pass: tasks put failing public-API tests (`render_to_svg`, `layout_metrics`) before code |
| V Typed errors | Pass: reuses `MathError::Unsupported` / `EmptyFormula`; no new variants |
| VI Minimal deps | Pass: zero new dependencies |

No violations; Complexity Tracking not needed. Re-checked after Phase 1 design: still passes.

## Project Structure

### Documentation (this feature)

```text
specs/003-latex-spacing-commands/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/latex-spacing-grammar.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/agent-canvas/
├── README.md                          # list spacing commands + scaling rules
├── src/math/latex/
│   ├── lexer.rs                       # backslash + one of , : ; ! space -> Command(symbol)
│   ├── parser.rs                      # NodeKind::Space; scripts never attach to glue
│   ├── atoms.rs                       # AtomContent::Space; glue-transparent demotion
│   ├── box_model.rs                   # layout_list: glue advance, prev class untouched
│   └── mod.rs                         # spacing-only -> EmptyFormula; clamp root width
└── tests/
    ├── latex_spacing.rs               # new: user scenarios, widths, classification, edges
    ├── latex_errors.rs                # update the `a\,b` expectation and hint text
    ├── latex_proptest.rs              # mix spacing into the generator
    └── snapshots/latex_spacing__integral.snap   # new, reviewed before acceptance
```

**Structure Decision**: single library crate; changes confined to the latex module, its tests,
README and specs, per the stated constraint.

## Phase artifacts

- Phase 0: [research.md](research.md) (all decisions resolved, no NEEDS CLARIFICATION)
- Phase 1: [data-model.md](data-model.md), [contracts/latex-spacing-grammar.md](contracts/latex-spacing-grammar.md), [quickstart.md](quickstart.md)
