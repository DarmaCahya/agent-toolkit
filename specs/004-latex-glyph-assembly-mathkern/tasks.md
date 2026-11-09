# Tasks: LaTeX Glyph Assembly and Math Kerning

**Input**: Design documents from `/specs/004-latex-glyph-assembly-mathkern/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/stretchy-glyph.md, quickstart.md

**Tests**: Included. Constitution Principle IV (test-first) and plan.md require failing tests before implementation.

**Organization**: Grouped by user story. Note US1 (radical) consumes US2 (`stretchy`), so US2's core builder is built first inside Phase 2; US1 then wires `sqrt()` to it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallel (different files, no unfinished dependencies)
- All paths relative to repo root. `L` = `crates/agent-canvas/src/math/latex`, `T` = `crates/agent-canvas/tests`.
- Constraints for every task: no `unwrap`/`expect`/indexing/`panic` in library code (use `get`, `Option`, iterators); no hash maps; no new dependencies; all lengths in em; every produced coordinate must pass `LBox::finite()`.

---

## Phase 1: Setup

**Purpose**: Establish a green baseline before changes.

- [X] T001 Run baseline gates from repo root and record results: `cargo fmt --check -p agent-canvas && cargo clippy -p agent-canvas --all-targets -- -D warnings && cargo test -p agent-canvas`. All must pass before continuing; no file changes.
- [X] T002 Record the current font facts for later comparison: `ls -l crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf` (expected 21,804 bytes) and `sha256sum` of it, noted in a scratch comment in your working notes (not committed).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Font subset and font-reading API that every story needs. Corresponds to User Story 4 plus the `font.rs` accessors.

**⚠️ CRITICAL**: No story work starts until this phase is complete.

### Tests first (must FAIL before implementation)

- [X] T003 [P] [US4] In `T/latex_font.rs` add tests: (a) `MathFont::assembly(gid)` returns `Some` with non-empty `parts` for the glyphs of `√ ( ) [ ] { } | ∫`; (b) every part `gid` in each assembly and every variant gid of those glyphs exists in the subset (closure); (c) `corner_kern(gid, Corner::TopRight)` / `BottomRight` is `Some` for at least one retained glyph; (d) subset byte length `< 250_000`; (e) family name still equals "AgentCanvas Math". Tests may relax lints locally only.
- [X] T004 [P] [US4] In `L/font.rs` `#[cfg(test)]` add unit tests for `CornerKern::at`: heights `[10,20]`, kerns `[a,b,c]`: `h=5→a`, `h=10→b` (equality uses entry just above), `h=15→b`, `h=20→c`, `h=99→c`, `+∞→c`, `−∞→a`, `NaN→0`; empty heights with one kern returns that kern.

### Implementation

- [X] T005 [US4] Extend `scripts/subset-font.sh` unicode list with `U+005B U+005D U+007B U+007D U+007C` alongside the existing `( ) √ ∑ ∫ ∏`. Add a post-subset closure-check step that walks retained variant and assembly part ids and exits non-zero if any is missing (if fontTools does not close over them, add `--glyphs` for the missing ids). Keep pinned SHA-256 check, timestamp pinning, and family name "AgentCanvas Math".
- [X] T006 [US4] Run `scripts/subset-font.sh <path-to-STIXTwoMath-Regular.otf>` twice; confirm identical printed sha256 both times and size `< 250_000` bytes; commit the rebuilt `crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf`. (Needs `uvx` + the pinned upstream file; if unavailable, stop and report, do not fabricate the asset.)
- [X] T007 [US4] In `L/font.rs` add `pub(crate) struct AssemblyPart { gid: GlyphId, start_connector: f64, end_connector: f64, advance: f64, extender: bool }` and `pub(crate) struct Assembly { parts: Vec<AssemblyPart>, min_overlap: f64 }` per data-model.md (em = units ÷ upem; drop parts with advance ≤ 0; `parts` never empty).
- [X] T008 [US4] In `L/font.rs` implement `MathFont::assembly(&self, gid) -> Option<Assembly>` using `ttf_parser::math` (`variants.vertical_constructions.get(gid)` → `assembly.parts`, `part_flags.extender()`, `variants.min_connector_overlap`). Return `None` when no vertical construction, no assembly, or no usable parts.
- [X] T009 [US4] In `L/font.rs` add `pub(crate) enum Corner { TopRight, BottomRight }`, `pub(crate) struct CornerKern { heights: Vec<f64>, kerns: Vec<f64> }` with `CornerKern::at(h: f64) -> f64` (first index `i` with `h < heights[i]` else last kern; `+∞`→last, `−∞`→first, NaN→0; use `get`, not indexing), and `MathFont::corner_kern(&self, gid, Corner) -> Option<CornerKern>` from `glyph_info.kern_infos.get(gid)` (`top_right`/`bottom_right`, `Kern::count/height/kern`; return `None` when no heights and no kerns).
- [X] T010 Run `cargo test -p agent-canvas --test latex_font` and the `font.rs` unit tests; T003 and T004 must now pass.

**Checkpoint**: Font has the data and typed accessors. `cargo test -p agent-canvas` still green.

---

## Phase 3: User Story 2 - Reusable stretchy-glyph builder (Priority: P1)

**Goal**: One `Layout::stretchy(gid, required, style) -> Result<LBox, MathError>` entry point choosing base → smallest variant → assembly → base.

**Independent Test**: Stretchy boxes for `( [ { | ∫ √` at heights from below base to 100× base are ≥ requested, never panic.

### Tests first (must FAIL before implementation)

- [X] T011 [P] [US2] In `L/box_model.rs` `#[cfg(test)]` add `stretchy` unit tests: required ≤ base extent → base glyph (single `Item::Glyph`); between variants → smallest sufficient variant; beyond largest variant → assembly of `Item::Glyph` only, extender count is the smallest reaching the target; glyph with no variants and no assembly → base glyph `Ok`; `NaN`, `0`, negative, `−∞` → base glyph; `+∞` and `1e12` → capped assembly with at most `MAX_EXTENDERS` (64) extenders, finite box.
- [X] T012 [P] [US2] In `L/box_model.rs` tests add assembly geometry tests: parts ordered bottom to top; each adjacent joint overlap `≥ min_overlap` and `≤ max(min_overlap, min(A.end, B.start))`; required parts appear exactly once; total height equals `required` when `min_height(n) ≤ required ≤ max_height(n)` (uniform overlap reduction) and uses max overlap otherwise; assembly with no extenders and assembly with all parts extenders terminate without panic.
- [X] T013 [P] [US2] In `T/latex_proptest.rs` add a proptest: for each glyph in `( ) [ ] { } | ∫ √` and `required` in `0.0..=100 × base extent` (plus special values NaN/±∞), `stretchy` never panics, result is finite, and `height + depth ≥ required` whenever the glyph has an assembly and `required` is finite and at most that glyph's cap-limited maximum (`MAX_EXTENDERS` extenders). Also record each glyph's maximum reachable extent so SC-003's 100× range is known rather than assumed (research R4).

### Implementation

- [X] T014 [US2] In `L/box_model.rs` add `const MAX_EXTENDERS: usize = 64;` (keep `MAX_RADICAL_STRETCH = 200.0`).
- [X] T015 [US2] In `L/box_model.rs` implement the assembly construction (private `assemble(&self, assembly: &Assembly, required: f64, style: Style) -> Result<LBox, MathError>`): per research R2 choose the smallest extender count `n ≥ 1` (or `0` if no extenders) with `max_height(n) ≥ required` and `n ≤ MAX_EXTENDERS`; compute per-joint `o_max = max(min_overlap, min(A.end, B.start))`; reduce overlaps uniformly from `o_max` toward `min_overlap` to hit `required` exactly when in range, else use `o_max`/`min_overlap` extremes; emit parts bottom to top as `Item::Glyph` at style scale with aligned x origin; guard zero/NaN/∞ divisors; return a box passing `LBox::finite()`.
- [X] T016 [US2] In `L/box_model.rs` implement `pub(crate) fn stretchy(&self, gid: GlyphId, required: f64, style: Style) -> Result<LBox, MathError>` per contracts/stretchy-glyph.md: measure ink extent `y_max − y_min` × style scale exactly like current radical code (research R3, keeps FR-008); non-finite-or-≤0 `required` → base glyph (`+∞` → capped assembly); order base → smallest sorted variant (stable `total_cmp` then gid) → `font.assembly()` → base; `MathError::Font` only for a glyph with no outline and no usable data.
- [X] T017 [US2] Run `cargo test -p agent-canvas stretchy` and `cargo test -p agent-canvas --test latex_proptest`; T011–T013 must pass.

**Checkpoint**: `stretchy` works standalone and is ready for `\left \right` later.

---

## Phase 4: User Story 1 - Tall radicals via assembly (Priority: P1) 🎯 MVP

**Goal**: `\sqrt` over a very tall radicand uses assembled pieces, not a vertically scaled glyph.

**Independent Test**: `\sqrt` over 12 nested `\frac` renders; no `ScaledGlyph`; radical height ≥ radicand height + gap; byte-identical across runs; fit-in-variant inputs unchanged.

### Tests first (must FAIL before implementation)

- [X] T018 [P] [US1] In `T/latex_layout.rs` add `sqrt_nested_fractions`: build `\sqrt{\frac{1}{\frac{1}{…}}}` with 12 nested `\frac` levels; assert render succeeds, SVG contains no non-uniform `scale(x y)` radical, radical total height ≥ radicand height + gap, adjacent radical parts overlap by ≥ the font's `min_connector_overlap` and are ordered bottom to top, and 10 renders are byte-identical (SC-001, SC-002, SC-004).
- [X] T019 [P] [US1] In `L/box_model.rs` replace the test `sqrt_of_huge_body_scales_instead_of_failing` (currently asserts `Item::ScaledGlyph` with `sy > 1.0`) with `sqrt_of_huge_body_uses_assembly`: asserts only `Item::Glyph` items form the radical and no `ScaledGlyph` is present. Keep `sqrt_is_taller_than_body`.

### Implementation

- [X] T020 [US1] In `L/box_model.rs` change `Layout::sqrt` (≈ line 351) to call `stretchy(radical_gid, body.height + body.depth + gap + rule, style)`. Keep the `ScaledGlyph` path (≈ line 388–416, guarded by `MAX_RADICAL_STRETCH`) only for a radical glyph with neither variants nor assembly (FR-007). Preserve existing positioning so fit-in-variant output is unchanged.
- [X] T021 [US1] In `L/render.rs` confirm `Item::ScaledGlyph` handling still compiles and is unchanged (kept per research R6); make no edit unless the compiler requires one.
- [X] T022 [US1] Run `cargo test -p agent-canvas --test latex_layout --test latex_spacing --test latex_quadratic`. All existing snapshots must be byte-identical (no `.snap.new`); T018 and T019 pass. If any snapshot changed, stop and investigate (FR-008).

**Checkpoint**: MVP complete. Tall radicals are crisp and deterministic.

---

## Phase 5: User Story 3 - MathKern corner kerning (Priority: P2)

**Goal**: Scripts on a single-glyph nucleus with kern data are offset by the font's kern value.

**Independent Test**: Script x differs from the kern-ignored placement by exactly the kern value; no-kern nucleus unchanged.

### Tests first (must FAIL before implementation)

- [X] T023 [P] [US3] In `L/box_model.rs` tests add kern tests: (a) superscript on a kerned glyph differs from kern-ignored placement by exactly `kern(TopRight, script_bottom) + kern(TopRight, nucleus_top)` (research R7); (b) subscript uses `BottomRight` with `script_top` and nucleus bottom, same rule; (c) nucleus without kern data renders identically to today; (d) composite nucleus (`Nucleus` not `Glyph`) unchanged; (e) sup and sub on the same nucleus are each kerned by their own heights; (f) italic correction is applied once and kern is added after it (FR-011). Find a real kerned glyph in the subset (e.g., by scanning `corner_kern`) rather than hard-coding one.
- [X] T024 [P] [US3] In `T/latex_layout.rs` add an end-to-end test rendering a script on a kerned glyph and on an unkerned glyph, asserting the unkerned output equals its existing snapshot and the kerned one differs by the kern amount in script x.

### Implementation

- [X] T025 [US3] In `L/box_model.rs` extend `Layout::scripts` (≈ line 449) with an optional nucleus gid parameter and update all call sites (pass `Some(gid)` only when `kind == Nucleus::Glyph`, else `None`). When `Some(gid)` and `font.corner_kern(gid, Corner::TopRight)` exists, superscript x = `nucleus.width + nucleus.italic + kern`, kern as in R7 (sum of lookups at script bottom and nucleus top, heights in em, converted consistently with `CornerKern::at`); likewise `BottomRight` for subscript x = `nucleus.width + kern`. When `None` or no kern data, placement is bit-for-bit as before.
- [X] T026 [US3] Run the full existing suite `cargo test -p agent-canvas`. For every changed snapshot, open the old and new SVG, confirm the nucleus glyph has kern data, and write one line of justification per snapshot into `specs/004-latex-glyph-assembly-mathkern/snapshot-notes.md` (SC-005); only then accept via `cargo insta`. Unchanged snapshots need no note.

**Checkpoint**: All user stories independently functional.

---

## Phase 6: Polish & Cross-Cutting

- [X] T027 [P] Update `crates/agent-canvas/README.md` with a short note that radicals/delimiters use OpenType MATH glyph assembly and scripts honour MathKern; mention `\left \right` remains unsupported.
- [X] T028 Verify `git diff` adds no `#[allow(...)]` in library code, no new dependency in any `Cargo.toml`/`Cargo.lock`, and no edits to `lexer.rs`, `parser.rs`, `atoms.rs`, `error.rs` (FR-015, FR-018, SC-008).
- [X] T029 Run the full gates from quickstart.md §1 and §2–§6: `cargo fmt --check -p agent-canvas`, `cargo clippy -p agent-canvas --all-targets -- -D warnings`, `cargo test -p agent-canvas`; confirm font size `< 250_000` and rebuild-twice hashes match (T006).

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (blocks everything) → Phase 3 (US2 `stretchy`) → Phase 4 (US1 radical) → Phase 5 (US3 kerning) → Phase 6.
- US1 depends on US2 (`sqrt` calls `stretchy`). US3 depends only on Phase 2 (`corner_kern`) but edits the same file `box_model.rs`, so run after Phase 4 to avoid conflicts (or in a separate branch and merge).
- Within a phase: tests first and failing, then implementation, then run.
- T005 → T006 → T007–T009 (T007–T009 are in one file `font.rs`; do sequentially). T003/T004 are parallel with each other.
- T014 → T015 → T016 (same file, sequential).

## Parallel Opportunities

- T003 ‖ T004 (different files).
- T011 ‖ T012 are in the same file (`box_model.rs` tests) so write together; T013 (proptest file) is parallel to both.
- T018 ‖ T019 (different files). T023 ‖ T024 (different files).
- T027 ‖ T028.

## Implementation Strategy

- **MVP**: Phases 1–4 (font data + `stretchy` + tall radical). Stop and validate quickstart §2–§3, §5, §6.
- **Increment**: Phase 5 (kerning) adds the typographic refinement; Phase 6 closes out.
- Commit after each phase checkpoint.
