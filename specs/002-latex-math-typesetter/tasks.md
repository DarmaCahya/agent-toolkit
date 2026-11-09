---

description: "Task list for LaTeX Math Typesetter (Vector SVG)"
---

# Tasks: LaTeX Math Typesetter (Vector SVG)

**Input**: Design documents from `specs/002-latex-math-typesetter/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/public-api.md, quickstart.md, `.specify/memory/constitution.md`

**Tests**: REQUIRED. Constitution Principle IV (test-first, user-scenario tests, red before green) and FR-025 mandate them. Every implementation task is preceded by a test task that MUST be seen failing first.

**Organization**: Grouped by user story. **Ordering deviation**: US4 (font, P3) is scheduled *before* US1 because every rendering story needs the embedded font; it is a hard prerequisite, not a lower-priority nice-to-have.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US4 (user-story phases only)

## Path Conventions

Crate root `C = crates/agent-canvas`. Module root `M = crates/agent-canvas/src/math/latex`. Integration tests in `crates/agent-canvas/tests/`. Scripts in `scripts/` (repo root; does not exist yet).

**Module-wide rules (apply to every `M/*.rs` task)**: no `unwrap`/`expect`/indexing/`panic!` (use `get`, iterators, pattern matching, checked/saturating arithmetic); positions are char indices (not bytes); ordered collections only (`BTreeMap`/sorted `Vec`), no `HashMap` in output paths; no `unsafe`.

---

## Phase 1: Setup

**Purpose**: Dependencies and module skeleton

- [X] T001 (`ttf-parser` with `std`+`opentype-layout` (std needed, see Cargo.toml comment); `insta` dev-dep added with `default-features = false` and works; Cargo.lock adds only insta/similar/tempfile/rustix/errno/linux-raw-sys, all dev-only.) Add `ttf-parser = { version = "0.25", default-features = false, features = ["opentype-layout"] }` to `[dependencies]` and `insta = { version = "1", default-features = false }` to `[dev-dependencies]` in `crates/agent-canvas/Cargo.toml`; run `cargo build -p agent-canvas` and confirm `Cargo.lock` adds no new runtime package (ttf-parser 0.25.1 already locked). If the MATH table or float math fails to compile, add the `std` feature (research R2) and record the outcome in a comment on the dependency line. If `insta` snapshot assertions fail without default features, enable the minimum extra feature needed (research R10).
- [X] T002 Create module skeleton: `M/mod.rs` with `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]`, `pub const MAX_INPUT_CHARS: usize = 8192; pub const MAX_DEPTH: usize = 64;`, `pub struct MathTypesetter;` with `new()`, `Default`, and `render_to_svg(&self, latex: &str) -> Result<String, MathError>` returning `Err(MathError::EmptyFormula)` as a stub; declare empty submodules `error, lexer, parser, atoms, font, box_model, render` (one file each under `M/`). Add `pub mod latex;` and `pub use latex::{MathError, MathTypesetter};` to `crates/agent-canvas/src/math/mod.rs`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The error type and the shared number formatter every later phase uses

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 [P] Write failing unit tests in `M/error.rs` (`#[cfg(test)]`): each variant's `position()` returns the documented `Option<usize>` (`EmptyFormula`, `InputTooLong`, `MissingGlyph`, `Font` → `None`; the others → `Some(pos)`), `Display` messages name the offending item (command name / char) and include a hint, and the type is `Clone + PartialEq + Eq + Debug`.
- [X] T004 Implement `MathError` in `M/error.rs` per research R8: `#[non_exhaustive]`, `thiserror::Error`, derives `Debug, Clone, PartialEq, Eq`; variants `EmptyFormula`, `InputTooLong{len,max}`, `NestingTooDeep{pos,max}`, `UnexpectedEof{pos,expected}`, `UnexpectedChar{pos,ch}`, `UnknownCommand{pos,name}`, `Unsupported{pos,name,hint}`, `MissingArgument{pos,command}`, `DoubleScript{pos,kind}`, `MissingGlyph{ch}`, `Font{reason}`; `pub fn position(&self) -> Option<usize>`. Make T003 pass.
- [X] T005 [P] Write failing unit tests in `M/render.rs` (`#[cfg(test)]`) for `fmt_num(f64) -> Result<String, MathError>`: rounds to 2 dp via integer fixed-point, strips trailing zeros (`1.50`→`1.5`, `2.00`→`2`), normalises `-0`/`-0.004` to `0`, never emits exponent, and rejects NaN/±inf with `MathError::Font`-class internal error (no panic).
- [X] T006 Implement `fmt_num` in `M/render.rs` using `(x * 100.0).round()` → `i64` (check range with `is_finite` and bounds before casting) and hand-built decimal output (research R4). Make T005 pass.

**Checkpoint**: `cargo test -p agent-canvas error` and `fmt_num` tests green.

---

## Phase 3: User Story 4 - Redistributable, license-compliant bundled math font (Priority: P3, scheduled first — blocks US1/US2) 

**Goal**: A committed, renamed STIX Two Math subset (< 250 KB, with MATH table) plus licence, notices, and a reproducible regeneration script; `font.rs` exposes the glyph/MATH data the layout needs.

**Independent Test**: Font tests pass on the committed asset; running `scripts/subset-font.sh` twice yields identical SHA-256 equal to the committed asset.

**Blocker resolved**: upstream font is `.font-src/STIXTwoMath-Regular.otf` (CTAN `stix2-otf` 2021-04-02, OFL-1.1, SHA-256 `95bc2729…8d90b`, pinned in `scripts/subset-font.sh`).

### Tests for User Story 4 (write first, must fail)

- [X] T007 [P] [US4] Write failing integration tests in `crates/agent-canvas/tests/latex_font.rs` reading the asset with `include_bytes!("../assets/fonts/AgentCanvasMath-subset.otf")` and `ttf_parser::Face::parse`: (a) file size `< 250 * 1024`; (b) `face.tables().math` is `Some` and `constants` is `Some` (MATH constants present); (c) name table family (name ID 1 and 16) equals `"AgentCanvas Math"` and no name record (any ID) contains `"STIX"` as a family/Reserved-Font-Name usage (attribution is allowed only in copyright/licence name IDs 0 and 13/14 — assert accordingly); (d) every code point required by FR-007–FR-009 maps to a glyph: `A–Z`, `a–z` via Mathematical Italic block U+1D434–U+1D467 (with `h`→U+210E), digits `0–9`, `+ - = < > ( ) ,` (use U+2212 for minus; assert the mapping chosen in T016), Greek lower/upper (italic lower U+1D6FC–U+1D714, upright upper U+0391–U+03A9), `± · × ≤ ≥ ≠ ∞`, `√ ∑ ∫ ∏`; (e) `√`, `∑`, `∫`, `∏` have vertical variants in MathVariants (`variants.vertical_constructions.get(gid)` non-empty); (f) italic-correction data exists for at least one math-italic glyph; (g) files `crates/agent-canvas/assets/fonts/OFL-1.1.txt` and a repo-root `THIRD_PARTY_NOTICES.md` entry containing `STIX Two` and `OFL` exist.

### Implementation for User Story 4

- [X] T008 [P] [US4] Add `crates/agent-canvas/assets/fonts/OFL-1.1.txt` (verbatim OFL 1.1 text with the STIX Fonts copyright line from the upstream release) and create `THIRD_PARTY_NOTICES.md` at the repo root (location confirmed in plan.md as root) with an entry attributing STIX Two Math (STI Pub Companies, OFL-1.1), stating that `AgentCanvasMath-subset.otf` is a modified, renamed subset and pointing to the licence file.
- [X] T009 [US4] Resolve the BLOCKER above (obtain the upstream font path or download approval) and record the final source path/URL and SHA-256 in a header comment of `scripts/subset-font.sh`.
- [X] T010 [US4] Create `scripts/subset-font.sh` (executable, `set -euo pipefail`, takes the source `.otf` path as `$1`): verify the source SHA-256 against the pinned value; run `uvx --from fonttools pyftsubset` with an explicit `--unicodes=` list covering exactly the T007(d) code points plus `--layout-features=` restricted to what MATH needs and glyph closure for size variants/extenders (`--glyph-names` off, `--no-hinting`, `--drop-tables+=` for unneeded tables, MATH kept); then rename name records via a small inline fontTools script (family/subfamily/full/PostScript names → "AgentCanvas Math", remove "STIX Two" Reserved Font Name from IDs 1,3,4,6,16,17; keep copyright + licence IDs 0,13,14); fix `head.modified`/`head.created` using a constant (`SOURCE_DATE_EPOCH`-style) for determinism; write to `crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf`. Run it twice and compare `sha256sum` output (SC-006).
- [X] T011 [US4] Run `scripts/subset-font.sh` to generate and commit `crates/agent-canvas/assets/fonts/AgentCanvasMath-subset.otf`; confirm size < 250 KB. Make T007 pass (iterate on the glyph list/drop-tables in T010 if size or coverage fails).
- [X] T012 [P] [US4] Write failing unit tests in `M/font.rs` (`#[cfg(test)]`) for the `MathFont` API: `MathFont::load()` succeeds; `consts()` returns non-zero `units_per_em`, `axis_height`, `script_percent_scale_down` (~70–80), `script_script_percent_scale_down`, fraction/radical constants; `glyph('a')` resolves to the italic glyph; `glyph('h')` resolves via U+210E; `glyph('€')` returns `Err(MissingGlyph)`; `advance`, `italic_correction`, `vertical_variants(√)` (sorted ascending by advance) and `display_variant(∑)` return plausible values; `outline_path(gid)` returns a non-empty SVG path string with only finite numbers.
- [X] T013 [US4] Implement `M/font.rs` (data-model "Font"): `include_bytes!("../../../assets/fonts/AgentCanvasMath-subset.otf")` (verify relative path from `M/font.rs`), `MathFont` wrapping `ttf_parser::Face` (build per call or via `std::sync::OnceLock`; no `unsafe`, no global mutable state), `MathConsts` snapshot struct of all MATH constants listed in data-model.md (script scale-downs, axis height, fraction rule thickness and numerator/denominator shifts and gaps for display/text, sub/superscript shifts and gaps, radical rule thickness / vertical gap (display+text) / extra ascender / degree bottom-raise percent / kern before and after degree, upper/lower limit gaps and min gap/shift, display-operator min height), lookups `glyph(char)`, `advance`, `italic_correction`, `vertical_variants`, `display_variant`, `bbox`, and `outline_path(gid, &mut impl)` with an `OutlineBuilder` that emits y-flipped path data in font units (`M/L/Q/C/Z`) with numbers formatted via `fmt_num`. Letter/Greek mapping helper `math_char(char, Face) -> char` lives here or in `atoms.rs` but must be defined once. Make T012 pass.

**Checkpoint**: Font asset, licence, notices, script, and `MathFont` all verified; `cargo test -p agent-canvas latex_font font` green.

---

## Phase 4: User Story 1 - Render a formula to a self-contained SVG (Priority: P1) 🎯 MVP

**Goal**: `MathTypesetter::render_to_svg(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")` returns a valid, deterministic, vector-only SVG with correct fraction, radical, superscript, and unary/binary minus spacing.

**Independent Test**: `cargo test -p agent-canvas --test latex_quadratic` passes: well-formed SVG, tight viewBox, no forbidden content, byte-identical repeat renders, unary vs binary minus spacing.

### Tests for User Story 1 (write first, must fail)

- [X] T014 [P] [US1] Write failing unit tests in `M/lexer.rs` (`#[cfg(test)]`): tokenises `\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` into expected `TokenKind`s with char-index `pos`; whitespace skipped; `Command` names parsed as ASCII letters after `\`; `LBracket`/`RBracket`/`Caret`/`Underscore`/braces; `( ) ,` lex as `Symbol`; `$`, `%`, `#`, `&`, `~`, non-ASCII, control chars, zero-width and tag code points (e.g. U+200B, U+E0041) → `UnexpectedChar{pos,ch}`; lone trailing `\` → `UnexpectedEof`; positions are char (not byte) indices when preceded by multi-byte chars.
- [X] T015 [P] [US1] Write failing unit tests in `M/parser.rs` (`#[cfg(test)]`): quadratic formula parses into `Frac{num, den}` with `Sqrt` inside; `b^2` → `Scripts{base:b, sup:2}`; `x^23` attaches only `2` as the script (single token) leaving `3` as a following sibling; `x_i^2` yields both scripts; `\sqrt[3]{x}` parses the index; symbol commands (`\pm`, `\alpha`, `\Gamma`, `\infty`, …) resolve to `Sym`; `\sum`/`\int`/`\prod` become `BigOp`; `^2` at start attaches to an empty-group base (Edge Cases).
- [X] T016 [P] [US1] Write failing unit tests in `M/atoms.rs` (`#[cfg(test)]`): class assignment (letters/digits Ord, `+ - ±` `·` `×` Bin, `= < > ≤ ≥ ≠` Rel, `(` Open, `)` Close, `,` Punct, `∑ ∫ ∏` Op); Bin→Ord demotion at start, end, after Bin/Rel/Open/Punct/Op, and before Rel/Close/Punct (`-b`, `a = -b`, `a +`); `space(left,right,style)` table values (Ord–Bin 4 mu, Ord–Rel 5 mu, Ord–Op 3 mu, …) and that 4/5 mu become 0 in script/scriptscript; `Style` transitions (`sup`, `sub`, `frac_num`, `frac_den`, `cramp`) per TeX; letter mapping (`a`→U+1D44E, `h`→U+210E, lowercase Greek italic, uppercase Greek upright, digits upright, `-`→U+2212).
- [X] T017 [P] [US1] Write failing unit tests in `M/box_model.rs` (`#[cfg(test)]`): `hbox` sums widths and takes max height/depth; `shift` moves items; a single glyph box has font advance as width; `frac` produces numerator and denominator horizontally centred on the bar (centre x within 0.001 em) and the bar thickness equals the MATH `fraction_rule_thickness`; basic `sqrt` box is taller than its body by at least rule thickness + vertical gap; all box numbers finite (`finite()` pass returns `Err` on injected NaN rather than panicking).
- [X] T018 [P] [US1] Write failing integration tests in `crates/agent-canvas/tests/latex_quadratic.rs` (relax lints with a local `#![allow(clippy::unwrap_used, clippy::expect_used)]`): (a) the quadratic formula renders `Ok`; output starts with `<svg`, has `xmlns`, `viewBox`, `fill="currentColor"`, and parses as well-formed XML (use a minimal tag-balance check helper in the test file, no extra dependency); (b) output contains none of `<text`, `<tspan`, `<style`, `<script`, `<foreignObject`, `<image`, `font-family`, ` on[a-z]+=`, or any `href` not starting with `#`; only elements `svg|defs|path|use|rect` appear; (c) 100 renders are byte-identical; (d) viewBox is within the formula bbox ± 20 units padding (compute from the box API or assert `width/height` ratio within tolerance of the reference); (e) unary minus: horizontal gap between the `−` glyph and `b` in `-b` is smaller than the gap between `a`, `−`, `b` in `a-b` (compare `<use>` x offsets); (f) repeated glyphs share one `<path id=…>` (render `aaaa`, count of `<path` equals 1); (g) snapshot via `insta::assert_snapshot!` of the quadratic SVG (accept the initial snapshot only after the visual check in T024).

### Implementation for User Story 1

- [X] T019 [US1] Implement `M/lexer.rs` (data-model "Lexer"): `Token { kind, pos }`, `TokenKind` as specified, whitelist-only lexing over `char_indices` converted to char positions via a counter (not byte offsets), return `Result<Vec<Token>, MathError>`. Make T014 pass.
- [X] T020 [US1] Implement `M/parser.rs` (data-model "AST"): recursive-descent over tokens using `get`/iterator cursor; `Node`/`NodeKind`/`Sym`; depth counter threaded through all recursion (full limit enforcement tested in US3 but the counter must exist now); `Scripts` attachment with empty-group base; `\frac{}{}`, `\sqrt[...]{}`, `\sum/\int/\prod`; symbol-command table for FR-007; unknown commands → `UnknownCommand`, with the `Unsupported` list stubbed to be completed in T038. Make T015 pass.
- [X] T021 [US1] Implement `M/atoms.rs` (data-model "Atoms"): `AtomClass`, `Style`/`StyleSize` with transitions and `mu_scale()` (derived from the font's script scale-down constants, not hard-coded 0.7), `Atom`, AST→atom conversion with Bin demotion, the TeX inter-atom spacing table, and letter/Greek/digit/operator code-point mapping (`h`→U+210E; hyphen-minus `-` → U+2212). Make T016 pass.
- [X] T022 [US1] Implement `M/box_model.rs` core (data-model "Box model"): `LBox`, `Item::{Glyph, Rule}`, `hbox` with inter-atom spacing (mu → em using current style and the font's 18 mu = 1 em convention), `shift`, `vstack`, glyph placement with italic correction, a `finite()` validation pass, TeX Rule 15a–e `frac` (display vs text shifts and gaps from MATH constants, bar centred on axis height, numerator/denominator centred), Rule 11 `sqrt` choosing the smallest vertical variant with advance ≥ required height (the scaled fallback is added in US2), and Rule 18 sub/superscript placement for the basic case needed by `b^2` (full Rule 18 variants finished in US2). Make T017 pass.
- [X] T023 [US1] Implement `M/render.rs` SVG emission (data-model "Render"): `<svg xmlns="http://www.w3.org/2000/svg" viewBox="minx miny w h" fill="currentColor">`, `<defs>` with one `<path id="g{gid}" d="…"/>` per unique glyph in `BTreeMap` order, `<use href="#g{gid}" transform="translate(x y) scale(s)"/>` per placed glyph, `<rect>` per rule, 1 em = 1000 units, padding 20 units, all numbers via `fmt_num`; deterministic attribute order.
- [X] T024 [US1] Wire `M/mod.rs` `render_to_svg`: `trim`-empty check (`EmptyFormula`), lex → parse → atoms → layout (initial style Display, uncramped) → render; propagate `MathError`. Make T018 pass; confirm the unit tests T014–T017 still pass. Then render the quadratic formula, open it in a browser (or rasterise with any available tool), visually confirm fraction/radical/unary-minus layout, and only then accept the T018(g) `insta` snapshot; state plainly whether visual verification was actually performed.

**Checkpoint**: US1 fully functional and independently testable (MVP). Run `cargo test -p agent-canvas --test latex_quadratic` and visually open the quadratic SVG in a browser (see T024).

---

## Phase 5: User Story 2 - Correct math layout: scripts, fractions, roots, large operators (Priority: P2)

**Goal**: TeX-quality scripts, nested fractions, tall radicals with scaled fallback, roots with index, and display-style big operators with limits.

**Independent Test**: `cargo test -p agent-canvas --test latex_layout` passes, including `test_math_nested_fractions` and `test_math_sqrt_stretch`.

### Prerequisite for User Story 2 tests (execute first in this phase)

- [X] T031 [US2] Add the `#[doc(hidden)] pub fn layout_metrics(&self, latex: &str) -> Result<BoxMetrics, MathError>` hook (and `BoxMetrics`) in `M/mod.rs`; must share the same pipeline as `render_to_svg` (no duplicate layout path). T025 cannot compile without it.

### Tests for User Story 2 (write first, must fail)

- [X] T025 [P] [US2] Write failing integration tests in `crates/agent-canvas/tests/latex_layout.rs` using a `pub(crate)`-style layout metrics hook exposed behind `#[doc(hidden)] pub fn layout_metrics(&self, latex) -> Result<BoxMetrics, MathError>` on `MathTypesetter` (hook added by T031, which runs first; `BoxMetrics { width, height, depth, items }` in em with named item positions as needed): (a) `x^2`, `x_i`, `x_i^2`: script glyph scale equals the font's script percentage (±1%), superscript baseline raised, subscript lowered, and superscript x on an italic `x` is offset right by the italic correction; (b) `test_math_nested_fractions`: for `\frac{\frac{a}{b}}{c}` the outer bar y and inner bar y both sit at their own level's math axis within 1% of em, inner fraction is in script-or-text style (smaller scale than outer), and all bars are horizontally centred over their content; (c) `test_math_sqrt_stretch`: height of `\sqrt{\frac{a}{b}}` radical glyph/rule region is strictly greater than that of `\sqrt{x}` and ≥ content height + vertical gap; top rule x-extent covers content width; (d) `\sqrt[3]{x}`: index scale is scriptscript-sized and positioned left of and above the radical's lower portion per MATH degree constants; (e) `\sum_{i=0}^{n} i` in display style: limits centred above/below the enlarged operator (centre x within 0.01 em); in the same formula nested in a script (e.g. `x^{\sum_i}`) limits become side scripts; (f) `\int_a^b` keeps side scripts in display style; (g) a very tall radicand (e.g. 12 nested fractions `\sqrt{\frac{\frac{…}{…}}{…}}` within depth limit) still renders `Ok` with the radical drawn using a `scale(` on the y axis and without clipping (radical height ≥ content height); (h) golden bbox for the quadratic formula against recorded reference values with tolerance ≤ 3% of formula height (SC-001) — record reference values from the first approved render and document how they were verified in a comment; (i) `insta` snapshots for `x_i^2`, `\sum_{i=0}^{n} i`, `\sqrt[3]{x}`.

### Implementation for User Story 2

- [X] T026 [US2] Complete TeX Rule 18 in `M/box_model.rs`: superscript/subscript shift-up/down from MATH constants (`superscript_shift_up`, `_cramped`, `subscript_shift_down`, `superscript_bottom_min`, `subscript_top_max`, `superscript_baseline_drop_max`, `subscript_baseline_drop_min`, `sub_superscript_gap_min`, `superscript_bottom_max_with_subscript`), script-style size from `Style::sup()/sub()`, italic correction added to superscript x offset only, cramped styles for subscripts. Part of making T025 (a) pass.
- [X] T027 [US2] Implement nested-style handling for fractions in `M/box_model.rs`: numerator uses `frac_num` style, denominator uses cramped `frac_den` style, display vs text constants chosen by current style, bar placed at axis height of the *current* level; scale items by the style's size factor when composing child boxes. Part of making T025 (b) pass.
- [X] T028 [US2] Implement radical growth in `M/box_model.rs`: required height = content height + depth + vertical gap + rule thickness (+ extra ascender); choose smallest variant with advance ≥ required (from `vertical_variants(√)`); if none, use the largest variant and emit a `Item::ScaledGlyph{ gid, x, y, sx, sy }` with `sy = required / advance_of_largest` (finite and clamped to a sane maximum, return `MathError::Font` instead of emitting non-finite values); draw the top rule over the content; place optional index using `radical_degree_bottom_raise_percent`, `radical_kern_before_degree`, `radical_kern_after_degree` with scriptscript style. Update `M/render.rs` to emit non-uniform `scale(sx sy)` for `ScaledGlyph`. Part of making T025 (c), (d), (g) pass.
- [X] T029 [US2] Implement big operators in `M/box_model.rs` and `M/atoms.rs`: in Display style use `display_variant(gid)` (min height from `display_operator_min_height`), centre the operator on the axis, Rule 13 limits above/below with `upper_limit_gap_min`, `upper_limit_baseline_rise_min`, `lower_limit_gap_min`, `lower_limit_baseline_drop_min`, and horizontal centring including italic correction shift for `∫`-like glyphs; `\int` keeps side scripts in all styles; other styles use text-size operator with side scripts (via Rule 18). Part of making T025 (e), (f) pass.
- [X] T030 [US2] Make T025 pass end to end; record golden bbox constants; then manually render `\sum_{i=0}^{n} i`, `\sqrt[3]{x}`, and the nested-fraction/tall-radical cases to SVG, open them in a browser (or rasterise with any available tool) and visually confirm: limits are centred, the index sits left of the radical, nested bars align, the tall radical covers its content. State plainly in the final report whether visual verification was actually performed. Only then accept the US2 `insta` snapshots with `cargo insta accept` (or `INSTA_UPDATE=always`).

**Checkpoint**: US1 and US2 pass independently; layout matches reference extents.

---

## Phase 6: User Story 3 - Clear, positioned errors and safe behaviour on hostile input (Priority: P2)

**Goal**: Every malformed, unsupported, or hostile input returns a typed error with position and hint; no panics, hangs, stack overflow, or giant output.

**Independent Test**: `cargo test -p agent-canvas --test latex_errors` passes, including the ≥ 10,000-case proptest.

### Tests for User Story 3 (write first, must fail)

- [X] T032 [P] [US3] Write failing integration tests in `crates/agent-canvas/tests/latex_errors.rs` (table-driven, asserting both variant and `position()`): `` and `"   "` → `EmptyFormula`; `\frac{a}{` → `UnexpectedEof` (pos = 9, the input's char count); `\foo` → `UnknownCommand{name:"foo",pos:0}`; `\left( x \right)`, `\text{hi}`, `\begin{matrix}`, `\hat{x}`, `\newcommand`, `\mathbf{x}`, `\,` → `Unsupported` naming the command; `x^2^3` and `x_1_2` → `DoubleScript` at the second script; `\frac{a}`, `\sqrt`, `x^` → `MissingArgument`/`UnexpectedEof`; `$x$`, `x%`, `é`, a string containing U+200B and one containing U+E0041 → `UnexpectedChar` at the right char index (including after multi-byte characters); a 8193-char input → `InputTooLong` returned *before* lexing (assert via a input of 8193 chars that would otherwise be an `UnexpectedChar` at position 0); an input of exactly 8192 valid chars is accepted or fails for a non-length reason; 64 nested `{` accepted, 65 → `NestingTooDeep`; nesting via `\frac` arguments, `\sqrt`, and `^{…}` chains each count toward the limit; every error row also asserts `err.to_string()` is non-empty and contains the offending name/char (SC-007).
- [X] T033 [P] [US3] Write failing property tests in `crates/agent-canvas/tests/latex_proptest.rs` with `ProptestConfig { cases: 10_000, .. }`: (a) arbitrary `String` input (`any::<String>()` plus a strategy that mixes LaTeX-ish tokens `\\frac`, `{`, `}`, `^`, `_`, `\\sqrt`, `[`, `]`, letters, digits, Greek names, and hostile Unicode) never panics and returns `Ok`/`Err`; (b) every `Ok` output passes the forbidden-content scan from T018 and contains only finite numbers (regex-free scan for `NaN`, `inf`, `e+`/`e-` in attribute values); (c) deeply nested generated input (depth 0–200) returns `Ok` iff depth ≤ 64; (d) wall-clock sanity: each case completes within 2 s (use `std::time::Instant` in the test only, not in library code); (e) render of an 8192-char worst-case (`{` × 64 nested then `a^` chains, and a long `\frac{a}{b}` repetition) completes within 2 s (SC-005).
- [X] T034 [P] [US3] Write failing performance/scenario tests in `crates/agent-canvas/tests/latex_errors.rs`: a ≤ 200-char realistic formula renders in < 50 ms in a release-or-debug-tolerant way (assert < 500 ms in debug, document the release target in a comment); output size for an 8192-char input is bounded (< 8 MiB).

### Implementation for User Story 3

- [X] T035 [US3] Enforce `MAX_INPUT_CHARS` in `M/mod.rs` as the very first step using `latex.chars().take(MAX_INPUT_CHARS + 1).count()` (do not walk the entire string), returning `InputTooLong{len,max}` where `len` is the exact char count only if cheaply known, else `MAX_INPUT_CHARS + 1` documented as "at least". Part of making T032 pass.
- [X] T036 [US3] Enforce `MAX_DEPTH` in `M/parser.rs`: increment the counter on entering each group, `\frac` arg, `\sqrt` arg/index, and script arg; return `NestingTooDeep{pos,max}` when exceeding 64 using checked arithmetic. Ensure the parser's recursion depth (and therefore stack usage) is bounded by this counter on every path, including long chains like `a^{b^{c^{…}}}`. Part of making T032/T033 pass.
- [X] T037 [US3] Implement `DoubleScript` detection and missing-argument/EOF errors in `M/parser.rs` with accurate char positions (second script's `^`/`_` position; position of the command lacking its argument; EOF position = input char count). Part of making T032 pass.
- [X] T038 [US3] Complete the `Unsupported` list in `M/parser.rs`: `left`, `right`, `text`, `begin`, `end`, `hat`, `bar`, `vec`, `tilde`, `dot`, `newcommand`, `renewcommand`, `def`, `mathbf`, `mathrm`, `mathit`, `mathcal`, `mathbb`, `color`, `,`/`;`/`:`/`!`/space commands, `displaystyle`, `textstyle`, `over`, `choose`; all others unknown → `UnknownCommand`. Each `Unsupported` error includes a hint such as "out of scope for v1; see issue #9 later parts". Part of making T032 pass.
- [X] T039 [US3] Audit all of `M/*.rs` for panics: run `cargo clippy -p agent-canvas --all-targets -- -D warnings` (module-level deny lints active); fix every finding without `#[allow]` (any unavoidable allowance must have an inline reason per the constitution); add checked/saturating arithmetic where coordinates or positions are computed; ensure no `as` cast can produce a misleading value without an `is_finite`/range check. Make T032, T033, T034 pass.

**Checkpoint**: All four user stories pass; error table and 10,000-case property test green.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T040 [P] Add rustdoc to `MathTypesetter`, `render_to_svg`, and `MathError` in `M/mod.rs` and `M/error.rs`, including the supported grammar from `contracts/public-api.md`, limits, determinism guarantee, and a doctest that renders `x^2` and asserts `Ok`.
- [X] T041 [P] Add `crates/agent-canvas/examples/latex-quadratic/main.rs` plus an `[[example]] name = "latex_quadratic"` entry in `crates/agent-canvas/Cargo.toml` that writes `quadratic.svg`, matching quickstart step 3. (Examples may relax lints locally.)
- [X] T042 [P] Update `crates/agent-canvas/README.md` (math section) and the repo `README.md` feature list to describe LaTeX → SVG support, the v1 grammar, limits, and out-of-scope items; mention `THIRD_PARTY_NOTICES.md`.
- [X] T043 Run the full quickstart (`specs/002-latex-math-typesetter/quickstart.md` steps 1–5) and fix any divergence between the doc and reality (update the doc if the doc is wrong). Confirm: `cargo fmt --check`; `cargo clippy -p agent-canvas --all-targets -- -D warnings`; `cargo test -p agent-canvas`; `Cargo.lock` diff shows no new runtime package (dev-dep `insta` only); run `scripts/subset-font.sh` twice and compare hashes; report every command's actual result.
- [X] T044 Prepare the PR description notes (do not open the PR unless asked): list the constitution principles touched (I, II, III, IV, V, VI), justify `ttf-parser` (what it replaces: hand-written OpenType glyf/CFF/MATH parsing) and `insta` (dev-only), and note the Python `fonttools` dev-only tool.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1)** → **Foundational (P2)** → **US4 font (P3)** → **US1 (P4)** → **US2 (P5)** and **US3 (P6)** → **Polish (P7)**
- US2 and US3 both depend on US1 (shared pipeline) but are independent of each other, except both touch `M/parser.rs` and `M/box_model.rs` — coordinate sequentially if one person does both, or split by function if two.
- US4's font tasks block every rendering task (T013 required by T017+).

### Within-Phase Ordering

- Tests (red) before implementation in every phase; confirm failure before writing code.
- US1: T019 → T020 → T021 → T022 → T023 → T024 (pipeline order); T014–T018 can all be written in parallel first.
- US2: T031 first (hook), then T026, T027, T028, T029 (different functions in `box_model.rs`; sequential recommended), then T030.
- US3: T035–T038 independent edits (T035 in `mod.rs`; T036–T038 in `parser.rs`, sequential), T039 last.

### Parallel Opportunities

- T003 ∥ T005; T007 ∥ T008 ∥ T012; T014 ∥ T015 ∥ T016 ∥ T017 ∥ T018; T025 alone (needs T031); T032 ∥ T033 ∥ T034; T040 ∥ T041 ∥ T042.

### Parallel Example: User Story 1 tests

```text
T014 lexer tests      (M/lexer.rs)
T015 parser tests     (M/parser.rs)
T016 atoms tests      (M/atoms.rs)
T017 box tests        (M/box_model.rs)
T018 scenario tests   (tests/latex_quadratic.rs)
```

---

## Implementation Strategy

### MVP First (US4 font + US1)

1. Phases 1–2 (deps, error, number formatter).
2. Phase 3 (font) — **resolve the font-source blocker first**.
3. Phase 4 (US1): quadratic formula end to end. **Stop and validate** (tests + visual check).

### Incremental Delivery

1. + US2: richer layout; validate with golden bboxes and snapshots.
2. + US3: hardening; validate with error table and 10,000-case proptest.
3. Polish, quickstart, PR notes.

### Notes

- Commit after each task or logical group; do not commit until asked.
- If the font source cannot be obtained, only Phases 1–2 and the non-font unit tests (T014–T016, lexer/parser/atoms implementations T019–T021) can proceed; everything that needs glyph data is blocked.
