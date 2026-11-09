# Research: LaTeX Explicit Spacing Commands

## R1. Representation of spacing
- **Decision**: `NodeKind::Space(Glue)` and `AtomContent::Space(Glue)`, with `Glue::Mu(f64)` or `Glue::Em(f64)`. The `Atom` keeps a placeholder class (`Ord`) that no code consults; every class-reading site checks for glue first.
- **Rationale**: smallest change to the existing enums; mirrors TeX math-list glue.
- **Alternatives**: a new `AtomClass::Glue` (rejected: every `match` and the spacing table must handle it, easy to leak into lookups); an empty Ord atom (rejected: Ord atoms trigger Bin demotion and automatic spacing, violating FR-005).

## R2. Transparency to classification (FR-005)
- **Decision**: in `demote_bins`, track `prev` and its index for the last non-glue atom and `continue` past glue. In `layout_list`, glue advances the width but does not set `prev`, so automatic spacing is computed between the nearest real atoms.
- **Rationale**: matches TeX, where glue does not change the previous atom type. Covers SC-003 (leading, trailing, before relation, after open bracket).

## R3. Scaling (FR-002/003)
- **Decision**: mu spaces use `mu * style.mu_scale(&consts)` (already scale/18): `\,`=3, `\:`=4, `\;`=5, `\!`=-3. Em spaces are `\quad`=1, `\qquad`=2, backslash-space=1/3, never multiplied by the style scale.
- **Rationale**: spec assumption; reuses the helper automatic spacing already uses.

## R4. Lexing
- **Decision**: after the backslash, if the next char is one of `, : ; !` or a literal space, emit `TokenKind::Command(<that char>)` and advance one char. Other punctuation keeps `Unsupported`, with a new hint listing the supported spacing commands and saying other symbol commands are out of scope. A trailing lone backslash stays `UnexpectedEof`. `\quad`/`\qquad` need no lexer change; the word rule keeps `\quadx` an unknown command (FR-010).
- **Rationale**: reuses the `Command` token and parser dispatch.

## R5. Scripts adjacent to glue (FR-006)
- **Decision**: `parse_item` returns right after a `Space` nucleus without consuming scripts, so `x\,^2` parses as `x`, space, then a leading-script item with an empty base (same as existing `^2`). `x^2\,` is unaffected.
- **Rationale**: glue is not a nucleus; avoids a `Scripts{base: Space}` shape.
- **Alternative**: attach the script to the previous atom (rejected: changes parse semantics).

## R6. ViewBox, negative width, NaN (FR-008)
- **Decision**: in `layout()` clamp `root.width = root.width.max(0.0)` before `finite()`. Glue is already part of `width`, so `render.rs` needs no change. Ink pushed left by negative leading space may overhang the viewBox padding; accepted.
- **Rationale**: minimal; satisfies "non-negative, no NaN". Coordinates stay far below `MAX_COORD` (input length x 2 em).

## R7. Spacing-only input (deferred to planning by the spec)
- **Decision**: if the atom tree contains glue and the laid-out root has no items, return `MathError::EmptyFormula`. Plain `{}` keeps rendering as before (FR-014).
- **Rationale**: an SVG with no ink is useless to agents and this matches whitespace-only input. Using the layout result rather than tokens also covers `{\,}`.
- **Alternative**: return an empty SVG (rejected: silent, useless output).

## R8. Limits (FR-012)
- **Decision**: no new limit. Each spacing command is one token, so `MAX_INPUT_CHARS` bounds the count and `MAX_DEPTH` is unaffected.

## R9. Tests and snapshot
- **Decision**: width assertions via `layout_metrics().width` differences (tolerance 0.01 em, SC-002); classification via unit tests in `atoms.rs` plus width equality; the integral snapshot via the existing `insta` setup, inspected (glyph `<use>` elements only, gap before `d`) before acceptance.
