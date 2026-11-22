# Feature Specification: LaTeX `\left ... \right` Auto-Sizing Delimiters

**Feature Branch**: `feat/latex-left-right` (existing branch; no branch created or switched)

**Created**: 2026-11-22

**Status**: Implemented

**Input**: "Extend the merged LaTeX typesetter (`crates/agent-canvas/src/math/latex`) with
`\left ... \right` auto-sizing delimiters: parse `\left<delim> ... \right<delim>`, grow each
delimiter to cover the content through the existing OpenType MATH stretchy path, centre both on
the math axis, and classify the construct as a TeX Inner atom. Located errors for unmatched
`\left` / `\right` and unknown delimiters; nesting, scripts and font subset updates included."

## Scope

In scope: `\left` / `\right` with delimiters `( ) [ ] { } |`, `\{`, `\}`, `\|`, `\langle`,
`\rangle`, `\lvert`, `\rvert`, `\lVert`, `\rVert` and `.` (null delimiter). Out of scope (still
rejected with a located error): `\big`/`\Big`/`\bigg`/`\Bigg` fixed-size delimiters, `\middle`,
`\text`, matrices, accents, font and style commands, macros.

## Requirements

- **R1 Parse.** `\left<delim> ... \right<delim>` produces one atom. The body is an ordinary
  formula list, so groups, fractions, radicals, operators and spacing work inside. A `\right`
  closes the *innermost* open `\left`; `\left` inside `{...}` is closed by its own `\right`
  before the brace closes.
- **R2 Errors.** An unclosed `\left` is `MathError::MissingRight { pos }` at the `\left`; a
  `\right` outside any `\left` is `MathError::UnexpectedRight { pos }`; a missing or unsupported
  delimiter is `MathError::InvalidDelimiter { pos, command, found }`. All are `position()`-bearing
  and never panic.
- **R3 Sizing.** Each delimiter is laid out with the existing
  `BoxBuilder::stretchy(gid, required, style)` path (smallest sufficient vertical variant, then
  OpenType glyph assembly, capped at 64 extenders). `required = max(body.height + body.depth,
  2 * max(body.height - axis, body.depth + axis))` em: cover the body, and be symmetric about the
  math axis. The natural glyph size is the floor — a short body keeps the base glyph.
- **R4 Centring.** Delimiter ink is centred on the math axis taken from the font's MATH
  `axisHeight` constant (`MathConsts::axis_height`, not a hardcoded 0.25 em). With the delimiter
  dominant, `height - depth == 2 * axis_height` exactly.
- **R5 Semantics.** The construct is `AtomClass::Inner`, so the TeXbook inter-atom table applies:
  Ord/Op/Close/Punct/Inner before Inner = thin (3 mu), Inner before Ord/Open/Punct/Inner = thin,
  Inner-Bin and Bin-Inner = medium (4 mu), Inner-Rel and Rel-Inner = thick (5 mu), Inner-Close and
  Open-Inner = 0. Thin spaces also apply in script styles; medium/thick do not. No explicit glue is
  inserted; the Inner box is exactly `delim + body + delim` wide. `Inner` does not demote a
  neighbouring binary operator (TeX demotes only before Rel, Close and Punct).
- **R6 Nesting and scripts.** `\left( \left[ ... \right] \right)` nests; each `\left` counts
  toward `MAX_DEPTH` (64), so 64 levels are accepted and 65 give `NestingTooDeep`. Scripts attach
  to the construct as to any boxed nucleus (`(...)^2`), using the ordinary rule-18 placement.
- **R7 Font.** U+2016 (double bar), U+27E8/U+27E9 (angle brackets) were added to
  `scripts/subset-font.sh`; the subset was regenerated deterministically (fixed timestamps,
  closure check kept). It stays under 250 KB (28,972 bytes) and remains named "AgentCanvas Math".
- **R8 Output contract.** Unchanged: pure `<path>`/`<use>`/`<rect>` SVG, no `<text>`, no fonts,
  finite two-decimal numbers, byte-identical across renders.

## Assumptions

1. **TeX class.** TeXbook Chapter 18 makes `\left ... \right` an Inner atom; the spacing table row
   and column "Inner" is implemented as described in R5. Parenthesised entries of that table are
   the script-style-suppressed ones, as in the rest of the table.
2. **Inner style.** The body is laid out in the *current* style (display content stays display
   size inside delimiters), matching TeX's "subformula in the current style". It is not cramped.
3. **Mismatched delimiters are legal**, as in TeX: `\left( x \right]` renders. Only a
   *missing* `\right`, a stray `\right` and an unsupported delimiter are errors.
4. **No null-delimiter glue.** TeX adds `\nulldelimiterspace` (0.12 em) for a null delimiter; the
   task requires no explicit glue, so `\left. x \right.` is byte-identical in metrics to `x`.
5. **`\{`, `\}`, `\|` are also ordinary math symbols** (Ord class) outside `\left`/`\right`;
   `|` is Ord, and `.` remains invalid outside a delimiter position (same `UnexpectedChar` error
   as before). The lexer gained `|` and `.` tokens for this.
6. **Delimiter glyph choice.** `\|`, `\lVert`, `\rVert` map to U+2016; `\lvert`, `\rvert` and
   `|` map to U+007C; `\langle`, `\rangle` map to U+27E8/U+27E9. Angle brackets have size variants
   but no glyph assembly in STIX Two Math; the shared stretchy path then returns the tallest
   variant, which is acceptable (variants reach 3.8 em).
7. **Snapshot churn is glyph-id only.** SVG path ids are font glyph ids. Adding three code points
   to the subset renumbers the retained glyphs (pyftsubset preserves source glyph order), so the
   five pre-existing snapshots changed *only* in `id="gN"` / `href="#gN"` integers; path data and
   every coordinate are byte-identical (verified by an id-normalised diff).

## Verification

`cargo test -p agent-canvas` covers: parser shapes/errors/depth (`parser::tests`), Inner class and
spacing table (`atoms::tests`), centring/nulling/coverage (`box_model::tests`), the new
`tests/latex_left_right.rs` (safe SVG, determinism, tall fraction, nesting, null delimiter,
delimiters around `\sum`, Inner spacing widths, located errors, depth, hostile input, snapshots)
and `tests/latex_font.rs` (new glyphs, variants, assemblies, size). Visual sanity was checked by
rendering the SVGs in a browser and inspecting them (fraction inside parentheses, sum with limits,
nested `( [ ... ] )`, `\langle \rangle`, double bars, assembled tall `|`).
