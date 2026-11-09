# Implementation Plan: LaTeX Glyph Assembly and Math Kerning

**Branch**: `feat/latex-glyph-assembly` (existing; `setup-plan.sh` reports `004-latex-glyph-assembly-mathkern`, the spec directory name) | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-latex-glyph-assembly-mathkern/spec.md`

## Summary

Replace the vertical-scaling radical fallback with OpenType MATH glyph assembly, exposed as one
reusable `stretchy` layout function (base glyph → smallest sufficient variant → assembly → base
as last resort). Add MathKern corner kerning to script placement for single-glyph nuclei. Rebuild
the committed font subset so assembly parts and kern data are retained. All of it uses the
`ttf-parser` 0.25 MATH API already in the dependency tree. No new dependencies.

## Technical Context

**Language/Version**: Rust 2021 (workspace edition)

**Primary Dependencies**: existing only: `ttf-parser` 0.25 (`math` module: `GlyphConstruction.assembly`, `GlyphAssembly.parts`, `GlyphPart`, `Variants.min_connector_overlap`, `GlyphInfo.kern_infos`, `Kern`). `thiserror` 2. **Nothing added** (user constraint, Principle VI).

**Storage**: committed binary asset `crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf` (currently 21,804 bytes)

**Testing**: `cargo test` (unit tests in `font.rs`/`box_model.rs`/`render.rs`, integration in `tests/latex_font.rs`, `tests/latex_layout.rs`, `tests/latex_proptest.rs`), `insta` snapshots, `proptest`

**Target Platform**: any (pure Rust library)

**Project Type**: library crate (`agent-canvas`, module `math::latex`)

**Performance Goals**: no regression for inputs without stretchy/kerned glyphs; assembly is O(parts × extender count), extender count capped

**Constraints**: zero panics (deny `unwrap_used`, `expect_used`, `indexing_slicing`, `panic`); byte-deterministic output; font < 250 KB; family name stays "AgentCanvas Math"; existing depth/length limits unchanged

**Scale/Scope**: scope is limited to `latex/{font,box_model,render}.rs`, `scripts/subset-font.sh`, `assets/fonts/`, `tests/`, the README, and `specs/`. Parser, lexer, atoms and error modules are untouched.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 design (still passes).*

| Principle | Status | How |
|---|---|---|
| I. Pure Rust | Pass | No C/C++ or external binaries at build/run time. `uvx fonttools` stays developer-only (existing), never in build/test. |
| II. Zero panics | Pass | All new font reads use `Option`/`get`; part/kern iteration via iterators; overlap math guards zero/NaN/∞ divisors; extender count capped (R4). Tests relax lints locally only. |
| III. Deterministic | Pass | No hash maps; fixed iteration order; subset rebuild pins timestamps (existing) and is verified by building twice. |
| IV. Test-first | Pass (process) | Tasks put failing tests (assembly invariants, kern deltas, font closure) before implementation. Scenario tests render `\sqrt` over 12 nested `\frac`. |
| V. Typed errors | Pass | No new public error type. Existing `MathError::Font` is reused for corrupt data. |
| VI. Minimal deps | Pass | Zero new dependencies. |

No violations, so Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/004-latex-glyph-assembly-mathkern/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── stretchy-glyph.md
├── checklists/requirements.md
├── tasks.md
└── snapshot-notes.md # created during implementation only if a snapshot changes (T026)
```

### Source Code (repository root)

```text
crates/agent-canvas/
├── assets/fonts/AgentCanvasMath-subset.otf   # rebuilt (glyph list extended)
├── README.md                                 # note assembly/kern support
├── src/math/latex/
│   ├── font.rs        # + assembly(), min_connector_overlap(), corner_kern()
│   ├── box_model.rs   # + stretchy(), assemble(); sqrt() uses stretchy(); scripts() applies kern
│   └── render.rs      # drop ScaledGlyph handling only if the variant is removed (see R6)
└── tests/
    ├── latex_font.rs      # assembly parts + kern presence, closure, size, name
    ├── latex_layout.rs    # 12-nested-frac radical, kern delta scenarios
    ├── latex_spacing.rs / latex_quadratic.rs  # existing snapshots, must stay byte-identical
    └── latex_proptest.rs  # stretchy height >= request, never panics
scripts/subset-font.sh      # extended glyph list; closure check step
```

**Structure Decision**: existing single-crate layout. All logic sits in the three named modules; no new files in `src/`.

## Design Decisions (detail in research.md)

1. **Font reads** go in `font.rs` as plain-data structs (`Assembly`, `AssemblyPart`) in em units, built from `ttf-parser`. Layout never touches `ttf-parser` types directly.
2. **`Layout::stretchy(gid, required, style) -> Result<LBox>`** in `box_model.rs` is the single stretch entry point. `sqrt()` calls it; `\left \right` will call it later.
3. **Assembly** is a new `Item`-free construction: parts become ordinary `Item::Glyph` entries stacked bottom to top, so `render.rs` needs no new item kind.
4. **Kerning** is applied inside `scripts()` only when `kind == Nucleus::Glyph`. The nucleus gid is passed in through a new optional parameter.
5. **Subset** extends the unicode list. The retained glyph list is verified by a closure check.

## Risks

- **fontTools MATH closure (R5)**: whether `pyftsubset` keeps variant and assembly part glyphs when only the base code point is listed is unverified. Mitigation: a closure test in `latex_font.rs` plus a post-subset check in the script. If glyphs are dropped, add `--glyphs` or extra code points.
- **Snapshot drift (SC-005)**: italic nuclei with scripts (`x_i`, `ξ²`) may change if kern data exists for them. Mitigation: inspect each changed snapshot and record the reason in the PR notes.
- **Adjacent-glyph rendering seams**: pieces overlap by at least the minimum connector overlap, so seams should be closed. Verified by the geometric overlap test.
