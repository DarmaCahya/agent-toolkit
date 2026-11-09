# Tasks: LaTeX Explicit Spacing Commands

**Input**: Design documents from `/specs/003-latex-spacing-commands/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/latex-spacing-grammar.md, quickstart.md

**Tests**: INCLUDED. Constitution principle IV (test-first, user-scenario tests) is NON-NEGOTIABLE: every test task below MUST be written and seen failing (red) before its implementation tasks.

**Organization**: Grouped by user story. All paths are relative to the repo root. Crate root: `crates/agent-canvas/`. Latex module: `crates/agent-canvas/src/math/latex/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1..US4 from spec.md
- Constraints for every task: `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/indexing/`panic` in library code (use `get`, pattern matching), no new dependencies, no `Cargo.toml` or `error.rs` changes, no new `MathError` variants.

## Conventions used by tests

Existing public API: `MathTypesetter::new().render_to_svg(&str)` and `layout_metrics(&str).width` (as used in `crates/agent-canvas/tests/latex_layout.rs`). Width assertions compare `layout_metrics(spaced).width - layout_metrics(unspaced).width` against the expected em within 0.01 (SC-002). Mu widths are mu/18 of the base em at base (text/display) style.

---

## Phase 1: Setup

**Purpose**: Establish baseline and test scaffolding.

- [X] T001 Run `cargo test -p agent-canvas` and `cargo clippy -p agent-canvas --all-targets -- -D warnings` from the repo root and record that the baseline is green (all existing latex snapshots and tests pass) before any edit.
- [X] T002 [P] Create `crates/agent-canvas/tests/latex_spacing.rs` with the module header, the `use` lines copied from `crates/agent-canvas/tests/latex_layout.rs`, and two helpers: `fn width(src: &str) -> f64` (via `layout_metrics`) and `fn delta(spaced: &str, plain: &str) -> f64` (`width(spaced) - width(plain)`). No test functions yet.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Types every story needs. No behaviour is changed for existing formulas (FR-014).

**CRITICAL**: No user story work starts until this phase compiles.

- [X] T003 In `crates/agent-canvas/src/math/latex/parser.rs` add `pub(crate) enum Glue { Mu(f64), Em(f64) }` (derive `Debug, Clone, Copy, PartialEq`) and the variant `NodeKind::Space(Glue)`. Fix every non-exhaustive `match` on `NodeKind` so the crate compiles (arms may be temporary pass-throughs that yield no atom).
- [X] T004 In `crates/agent-canvas/src/math/latex/atoms.rs` add `AtomContent::Space(Glue)` and `Atom::is_glue(&self) -> bool` (true only for `AtomContent::Space`). Glue atoms get the placeholder class `AtomClass::Ord`; document in a comment that the class is never read for glue. Fix every non-exhaustive `match` so the crate compiles.
- [X] T005 In `crates/agent-canvas/src/math/latex/box_model.rs` fix every non-exhaustive `match` on `AtomContent` so the crate compiles (temporary arm: zero-width, no items). Run `cargo build -p agent-canvas` and `cargo test -p agent-canvas` to confirm nothing regressed.

**Checkpoint**: Crate compiles, all existing tests still pass.

---

## Phase 3: User Story 1 - Standard integral with thin space (Priority: P1) MVP

**Goal**: `\int_a^b f(x)\,dx` renders instead of erroring `unsupported command \,`, with a thin space before `dx`. This story builds the full end-to-end pipeline (lex, parse, classify-transparent, layout, clamp) using the complete command table from `data-model.md`, so US2 is verified rather than rebuilt.

**Independent Test**: Render `\int_a^b f(x)\,dx`: it is `Ok`, byte-identical on a second render, matches the new snapshot, and `f(x)\,dx` is wider than `f(x)dx` by 3/18 em.

### Tests for User Story 1 (write first, confirm RED)

- [X] T006 [P] [US1] In `crates/agent-canvas/tests/latex_spacing.rs` add `integral_with_thin_space_renders_ok` (asserts `render_to_svg(r"\int_a^b f(x)\,dx")` is `Ok` and the output has no `<text` element), `thin_space_widens_by_three_eighteenths_em` (asserts `delta(r"f(x)\,dx", "f(x)dx")` is within 0.01 of `3.0/18.0`), and `integral_render_is_deterministic` (two renders are byte-identical).
- [X] T007 [P] [US1] In `crates/agent-canvas/tests/latex_spacing.rs` add `integral_snapshot` using `insta::assert_snapshot!` on the SVG of `\int_a^b f(x)\,dx` (same pattern as `crates/agent-canvas/tests/latex_quadratic.rs`). Do NOT accept the snapshot yet.
- [X] T008 [US1] Run `cargo test -p agent-canvas --test latex_spacing` and confirm T006/T007 tests FAIL (red). Paste the failure summary into the PR notes.

### Implementation for User Story 1

- [X] T009 [US1] In `crates/agent-canvas/src/math/latex/lexer.rs`, after a backslash, if the next char is one of `,` `:` `;` `!` or a literal space (`' '`), emit `TokenKind::Command` carrying that single-character string and advance exactly one char (FR-010). Any other punctuation keeps the `Unsupported` path. A lone trailing backslash stays `UnexpectedEof`. Do not change the word-command rule, so `\quad`/`\qquad` lex as words and `\quadx` stays an unknown command.
- [X] T010 [US1] In `crates/agent-canvas/src/math/latex/parser.rs`, in the command dispatch, map: `,`→`Glue::Mu(3.0)`, `:`→`Mu(4.0)`, `;`→`Mu(5.0)`, `!`→`Mu(-3.0)`, `" "`→`Em(1.0/3.0)`, `quad`→`Em(1.0)`, `qquad`→`Em(2.0)`, each producing `NodeKind::Space(glue)`. In `parse_item`, return immediately after a `Space` nucleus without consuming `^`/`_` (FR-006, research R5).
- [X] T011 [US1] In `crates/agent-canvas/src/math/latex/atoms.rs`, lower `NodeKind::Space(g)` to an `Atom` with `AtomContent::Space(g)`.
- [X] T012 [US1] In `crates/agent-canvas/src/math/latex/box_model.rs` `layout_list`, for a glue atom compute `Mu(m) -> m * style.mu_scale(&consts)` and `Em(e) -> e`, apply it with `LBox::advance`, and `continue` WITHOUT updating `prev` (so automatic inter-atom spacing is computed between the nearest real atoms; FR-002/003/004). Replace the temporary arm from T005.
- [X] T013 [US1] In `crates/agent-canvas/src/math/latex/mod.rs` `layout()`, clamp `root.width = root.width.max(0.0)` before the existing `finite()` check (FR-008).
- [X] T014 [US1] Run `cargo test -p agent-canvas --test latex_spacing` until T006 tests pass (green). Then run `cargo insta review` (or inspect the `.snap.new`), confirm the SVG contains only `<path>` glyph definitions and `<use>` references, no `<text>`, and a visible gap before the `d`, then accept as `crates/agent-canvas/tests/snapshots/latex_spacing__integral.snap` (FR-013).

**Checkpoint**: US1 independently demonstrable: the reported UAT formula renders.

---

## Phase 4: User Story 2 - Full family of explicit spaces (Priority: P1)

**Goal**: Each of the seven commands adds exactly its specified width, additive when repeated.

**Independent Test**: `a<space>b` for each command vs `ab` matches the expected em within 0.01.

### Tests for User Story 2 (written first; most should pass once US1 lands, confirm any failure is a real defect)

- [X] T015 [P] [US2] In `crates/agent-canvas/tests/latex_spacing.rs` add `each_space_command_has_specified_width` as a table-driven test using `delta(spaced, "ab")` with: `a\,b`→3/18, `a\:b`→4/18, `a\;b`→5/18, `a\!b`→-3/18, `a\quad b`→1.0, `a\qquad b`→2.0, `a\ b`→1/3, tolerance 0.01 (SC-002). Note `a\ b` has a literal space after the backslash.
- [X] T016 [P] [US2] Add `repeated_spaces_accumulate` (`a\,\,\,b` vs `ab` equals 9/18) and `quad_whitespace_after_word_command_is_ignored` (`a\quad    b` has the same width as `a\quad b`).
- [X] T017 [P] [US2] Add `spacing_allowed_in_every_atom_position` asserting `Ok` for `{\,x}`, `x^{a\,b}`, `x_{a\,b}`, `\frac{a\,b}{c\quad d}`, `\sqrt{a\,b}`, `\sqrt[a\,b]{c}` (FR-001).
- [X] T018 [P] [US2] Add `lexer_does_not_swallow_following_chars` asserting `\,dx` and `\,\alpha` both render `Ok` and that `\,dx` is wider than `dx` by 3/18 once a real atom is present (use `a\,dx` vs `adx`), and that `\quadx` returns `Err` whose variant is the unknown-command error with name `quadx` (FR-010).
- [X] T019 [US2] Run `cargo test -p agent-canvas --test latex_spacing`; fix any real defect found in T009-T013 (no test weakening).

**Checkpoint**: US1 and US2 both pass; all seven commands verified.

---

## Phase 5: User Story 3 - Spacing is classification-transparent (Priority: P2)

**Goal**: Spacing never changes Bin demotion, unary detection, or automatic inter-atom spacing.

**Independent Test**: Width and demotion of each spaced binary-operator formula equal the unspaced one plus the explicit glue.

### Tests for User Story 3 (write first, confirm RED where applicable)

- [X] T020 [P] [US3] In `crates/agent-canvas/tests/latex_spacing.rs` add width-equality tests via `delta`: `\,-b` vs `-b` is 3/18 (minus stays unary); `a+\,b` vs `a+b` is 3/18; `a\,+b` vs `a+b` is 3/18; `a+\,=b` vs `a+=b` is 3/18 (demotion identical); `(\,+b)` vs `(+b)` is 3/18 (after open bracket); `a+\,` vs `a+` is 3/18 (trailing). Covers SC-003.
- [X] T021 [P] [US3] In `crates/agent-canvas/src/math/latex/atoms.rs` `#[cfg(test)] mod tests`, add unit tests that run the existing Bin-demotion function over atom lists with an interleaved `AtomContent::Space` atom and assert the resulting classes equal those of the list without the glue: leading `[Space, Bin, Ord]`, `[Ord, Bin, Space, Rel]`, `[Open, Space, Bin, Ord]`, `[Ord, Space, Bin, Ord]`.
- [X] T022 [US3] Run `cargo test -p agent-canvas` and confirm T020/T021 fail if `demote_bins` still treats glue as a normal atom (red).

### Implementation for User Story 3

- [X] T023 [US3] In `crates/agent-canvas/src/math/latex/atoms.rs` `demote_bins`, skip glue atoms (`is_glue()`): `prev` and its index always refer to the last non-glue atom; glue is never read for class and never demoted (FR-005, research R2).
- [X] T024 [US3] Re-run `cargo test -p agent-canvas`; T020/T021 pass.

**Checkpoint**: Classification equivalence holds in all tested contexts (SC-003).

---

## Phase 6: User Story 4 - Safe behaviour at the edges (Priority: P2)

**Goal**: No panics, NaN, infinities or negative sizes; sensible empty/spacing-only handling; updated error hint.

**Independent Test**: Every edge case yields a well-formed SVG (or the specified error) with finite, non-negative viewBox dimensions.

### Tests for User Story 4 (write first, confirm RED)

- [X] T025 [P] [US4] In `crates/agent-canvas/tests/latex_spacing.rs` add a helper `assert_sane(svg: &str)` that parses the `viewBox` attribute and asserts four finite numbers with width and height >= 0, and that the string contains none of `NaN`, `inf`. Add tests: `leading_space_shifts_content` (`\quad x` is wider than `x` by 1 and viewBox is sane), `trailing_space_widens_canvas` (`x\quad` wider than `x` by 1), `negative_total_width_is_clamped` (`x\!`, `\!x`, `\!\!\!x` all `Ok`, `assert_sane`).
- [X] T026 [P] [US4] Add `spacing_only_formula_is_empty_formula_error`: `\,`, `\quad`, `{\,}`, `\!` each return `Err(MathError::EmptyFormula)`; and `plain_empty_group_unchanged` asserting `{}` behaves exactly as before the feature (capture the current result from the unchanged code path; do not alter it) (R7, FR-014).
- [X] T027 [P] [US4] Add `script_adjacent_glue_does_not_change_attachment`: `x\,^2` and `x^2\,` are `Ok`; `x^2\,` width exceeds `x^2` by 3/18; `x^2` render bytes are unchanged vs a pinned expectation taken from the existing `x^2` output.
- [X] T028 [P] [US4] Add `mu_scales_in_script_but_em_does_not` (SC-006): the width contribution of `\,` inside `x^{a\,b}` (vs `x^{ab}`) is smaller than 3/18 and approximately 3/18 times the script scale; the contribution of `\quad` inside `x^{a\quad b}` (vs `x^{ab}`) is 1.0 within 0.01. Also add `spacing_in_fraction_arguments_is_centred_and_sane` (`\frac{\,a}{b}` renders, `assert_sane`).
- [X] T029 [P] [US4] Add `limits_and_operand_spacing_ok`: `\sum\,_i`, `\int\,f`, `\sum_{i}\,x` all `Ok`, `assert_sane`.
- [X] T030 [P] [US4] Add `many_spaces_respect_limits`: a string of `MAX_INPUT_CHARS / 2` repetitions of `\,` plus `x` renders `Ok` with sane output; an input exceeding `MAX_INPUT_CHARS` still returns the existing input-too-long error; deeply nested `{` beyond `MAX_DEPTH` containing `\,` still returns the existing depth error (FR-012). Import the limits the same way `crates/agent-canvas/tests/latex_errors.rs` does.
- [X] T031 [P] [US4] Add `trailing_backslash_space_vs_lone_backslash`: `x\ ` (backslash then space at end) is `Ok`; `x\` returns `Err(UnexpectedEof)`.
- [X] T032 [P] [US4] Add `unsupported_commands_still_error_with_new_hint`: `\left`, `\text`, `\hat` return the unsupported error with name and position; `\@` and `\%` return `Unsupported` whose hint does not say spacing is out of scope (assert it does not contain the old text `spacing and symbol commands are out of scope`) and lists the supported spacing commands (assert it contains `\quad`) (FR-011, R4).
- [X] T033 [P] [US4] In `crates/agent-canvas/tests/latex_errors.rs` update the existing case `(r"a\,b", ",", 1)` (line ~62): `a\,b` is now valid, so replace it with an unsupported single-symbol case such as `(r"a\@b", "@", 1)`. That file asserts no hint text, so nothing else changes. Do not weaken the other cases.
- [X] T034 [US4] Add `crates/agent-canvas/tests/latex_proptest.rs` generator coverage: extend the existing formula generator with the seven spacing commands as additional atoms/separators. Properties: never panics; on `Ok`, SVG has finite numbers and no `NaN`/`inf` (SC-004). Keep the existing properties unchanged.
- [X] T035 [US4] Run `cargo test -p agent-canvas` and confirm the new US4 tests that depend on the unimplemented parts (spacing-only error, hint) FAIL (red).

### Implementation for User Story 4

- [X] T036 [US4] In `crates/agent-canvas/src/math/latex/lexer.rs` (line ~67) replace the hint `"spacing and symbol commands are out of scope for v1"` with text that lists the supported spacing commands (`\,` `\:` `\;` `\!` `\quad` `\qquad` and backslash-space) and states other symbol commands are out of scope. It must not call spacing unsupported (FR-011).
- [X] T037 [US4] In `crates/agent-canvas/src/math/latex/mod.rs` `layout()`, after layout: if the atom tree contains glue (walk atoms, `is_glue()`) and the laid-out root has no drawable items, return `Err(MathError::EmptyFormula)`. Plain `{}` and all previously valid formulas are untouched (R7, FR-014). Use no panicking constructs.
- [X] T038 [US4] Re-run `cargo test -p agent-canvas`; all US4 tests and the proptests pass. Fix real defects only.

**Checkpoint**: All four stories pass independently.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T039 [P] Update `crates/agent-canvas/README.md`: list `\,` (3mu), `\:` (4mu), `\;` (5mu), `\!` (-3mu), `\quad` (1em), `\qquad` (2em), backslash-space (1/3 em); state that mu spaces scale with script style while em spaces do not; state that spacing-only input returns `EmptyFormula`; move spacing out of any "unsupported" list; keep `\left`/`\right`/`\text`/accents/`\hspace`/`\phantom` listed as unsupported (FR-013).
- [X] T040 [P] Add a byte-identity regression test in `crates/agent-canvas/tests/latex_spacing.rs`, `existing_formulas_render_identically`: render the formulas covered by the existing snapshots (`x^2`, sum with limits, sqrt with index, quadratic) and assert equality with the committed snapshot contents. Then confirm `git status` shows no changes to the existing `crates/agent-canvas/tests/snapshots/latex_layout__*.snap` or `latex_quadratic__*.snap` files (FR-014, SC-005).
- [X] T041 Run `cargo fmt --check` and `cargo clippy -p agent-canvas --all-targets -- -D warnings`; fix any lint (no `#[allow]` for the denied panic lints in library code).
- [ ] T042 Run the full `cargo test -p agent-canvas` and then `cargo test --workspace`; confirm the only snapshot changes are the new `latex_spacing__integral.snap` and the intentional hint text in `latex_errors.rs`.
- [ ] T043 Walk through `specs/003-latex-spacing-commands/quickstart.md` step 3 manually (integral Ok and deterministic; `\,` alone gives `EmptyFormula`; `\@` gives `Unsupported` with the new hint) and report the actual outputs, flagging anything unverified.
- [X] T044 Confirm scope: `git diff --stat main` touches only `crates/agent-canvas/src/math/latex/{lexer,parser,atoms,box_model,mod}.rs`, `crates/agent-canvas/tests/*`, `crates/agent-canvas/README.md` and `specs/003-latex-spacing-commands/`; `Cargo.toml`, `Cargo.lock` and `error.rs` are unchanged.

---

## Dependencies & Execution Order

### Phase dependencies

- Phase 1 → Phase 2 → Phases 3-6 → Phase 7.
- Phase 2 BLOCKS all stories (shared types).

### Story dependencies

- **US1 (P1)**: after Phase 2. Builds the whole pipeline; MVP.
- **US2 (P1)**: after US1 (verifies the command table built in T010). Test-only unless a defect appears.
- **US3 (P2)**: after US1 (needs glue in layout). Touches `atoms.rs` `demote_bins` only.
- **US4 (P2)**: after US1. T036/T037 touch `lexer.rs` and `mod.rs`; T033 depends on the hint text from T036.

### Within each story

- Tests written and seen RED before implementation (constitution IV).
- Same-file tasks are sequential: `lexer.rs` (T009 → T036), `parser.rs` (T003 → T010), `atoms.rs` (T004 → T011 → T023), `box_model.rs` (T005 → T012), `mod.rs` (T013 → T037), `tests/latex_spacing.rs` (all test tasks append to one file, so [P] there means independent test functions that can be drafted in parallel but must be merged serially).

### Parallel opportunities

- T002 alongside T001.
- Test-writing tasks T006/T007, T015-T018, T020/T021, T025-T034 are independent test functions (different logical tests); only the shared file requires serial merging. T021 (atoms.rs unit tests) and T033/T034 (other test files) are truly parallel to the `latex_spacing.rs` work.
- After US1, US3 (`atoms.rs`) and US4 hint/empty work (`lexer.rs`, `mod.rs`) touch different source files and can proceed in parallel.
- T039 and T040 in Polish are parallel.

### Parallel example: after US1

```text
Developer A: T020-T024 (US3, atoms.rs)
Developer B: T025-T038 (US4, lexer.rs / mod.rs / tests)
```

## Implementation Strategy

### MVP first (US1 only)

1. Phase 1 and 2.
2. Phase 3 (US1): red tests, pipeline, snapshot reviewed.
3. STOP and validate: `\int_a^b f(x)\,dx` renders, deterministic, snapshot inspected.

### Incremental delivery

1. US1 → ships the reported fix.
2. US2 → proves the full command family.
3. US3 → proves TeX-faithful classification.
4. US4 → hardens edges, hint text, spacing-only behaviour.
5. Polish → README, byte-identity, lint, scope check.

## Notes

- Every `[P]` test task writes to `tests/latex_spacing.rs` except T021, T033, T034; coordinate merges.
- Never weaken a test to pass; fix the implementation. Report any failing or unverified check honestly.
- Commit after each phase checkpoint.
