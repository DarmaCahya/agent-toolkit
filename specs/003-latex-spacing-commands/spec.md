# Feature Specification: LaTeX Explicit Spacing Commands

**Feature Branch**: `feat/latex-spacing` (existing branch; no branch created or switched)

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "Extend the merged LaTeX typesetter (crates/agent-canvas/src/math/latex) with explicit spacing commands, found missing in UAT: `\int_a^b f(x)\,dx` currently errors with 'unsupported command \,'. Add `\,` (3mu thin), `\:` (4mu medium), `\;` (5mu thick), `\!` (-3mu negative thin), `\quad` (1em), `\qquad` (2em), and `\ ` (interword space, about 1/3 em). Follow TeX semantics for how spacing scales with style. Spacing must not change atom-class neighbour logic (binary-operator demotion). Output stays pure path SVG; the viewBox must include space where it affects layout. Leading/trailing spacing must not crash or produce NaN. Update the error hint, README, tests and the integral snapshot. `\left\right`, `\text`, accents stay out of scope."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Write the standard integral with a thin space before `dx` (Priority: P1)

A user (or agent) writes `\int_a^b f(x)\,dx`, the canonical calculus notation, and gets a typeset formula in which `dx` is visibly separated from `f(x)` by a thin space, instead of an "unsupported command" error.

**Why this priority**: This is the reported UAT failure. The most common real-world formulas contain `\,` before a differential.

**Independent Test**: Render `\int_a^b f(x)\,dx` and confirm it succeeds. Confirm the gap between `)` and `d` is wider than in `\int_a^b f(x)dx` by exactly the thin-space amount.

**Acceptance Scenarios**:

1. **Given** `\int_a^b f(x)\,dx`, **When** rendered, **Then** a valid SVG is returned with no error.
2. **Given** `f(x)\,dx` and `f(x)dx`, **When** both are rendered, **Then** the first is wider than the second by 3/18 em at the formula's base size.
3. **Given** the integral example, **When** rendered twice, **Then** the SVG bytes are identical, and the SVG matches the updated reference snapshot.

---

### User Story 2 - Choose any of the standard explicit spaces (Priority: P1)

A user picks the amount of horizontal space they want, using the full conventional set: thin, medium, thick, negative thin, one-em, two-em, and a normal word space.

**Why this priority**: Users expect the whole family to work, not just the one command that was reported.

**Independent Test**: Render `a<space>b` with each command between two letters and compare the widths against the no-space `ab`.

**Acceptance Scenarios**:

1. **Given** `a\,b`, `a\:b`, `a\;b`, **When** rendered, **Then** each is wider than `ab` by 3/18, 4/18 and 5/18 em respectively.
2. **Given** `a\!b`, **When** rendered, **Then** it is narrower than `ab` by 3/18 em (the glyphs move closer together).
3. **Given** `a\quad b` and `a\qquad b`, **When** rendered, **Then** each is wider than `ab` by 1 em and 2 em respectively.
4. **Given** `a\ b` (backslash followed by a space), **When** rendered, **Then** it is wider than `ab` by about 1/3 em.
5. **Given** several spacing commands in a row (`a\,\,\,b`), **When** rendered, **Then** the spaces accumulate additively.

---

### User Story 3 - Spacing never alters how operators are classified (Priority: P2)

A user inserts spacing next to binary operators, such as `a\,+\,b`, `\,-b`, or `a+\,b`. Spacing adds its explicit width but never changes whether `+` or `-` is treated as a binary operator or a unary sign, matching TeX.

**Why this priority**: A subtle correctness property. If spacing disturbed classification, formulas would silently typeset differently from real TeX.

**Independent Test**: Compare atom classification and layout of formulas with and without spacing around binary operators.

**Acceptance Scenarios**:

1. **Given** `\,-b` (spacing before a leading minus), **When** rendered, **Then** the minus is still a unary sign. It receives no binary-operator spacing on either side, apart from the explicit thin space.
2. **Given** `a+\,b`, **When** rendered, **Then** `+` is still a binary operator, with the normal medium spaces around it plus the explicit thin space.
3. **Given** `a+\,=b`, **When** rendered, **Then** `+` is demoted to ordinary exactly as in `a+=b`.
4. **Given** `a\,+b`, **When** rendered, **Then** the width equals that of `a+b` plus the thin space.

---

### User Story 4 - Safe, sensible behaviour at the edges (Priority: P2)

Spacing at the start or end of a formula, a formula consisting only of spacing, spacing inside scripts and fractions, and very many repeated spaces all behave predictably and never crash or produce invalid numbers.

**Why this priority**: The library is consumed by autonomous agents with untrusted input.

**Independent Test**: Render each edge case and assert that the SVG is well formed, contains no NaN or infinite values, and has a non-negative viewBox size.

**Acceptance Scenarios**:

1. **Given** `\,x` or `x\,` (leading or trailing spacing), **When** rendered, **Then** the space is included in the viewBox: leading space shifts the content right, trailing space widens the canvas.
2. **Given** a formula that is only spacing (`\,`, `\quad`), **When** rendered, **Then** the same `EmptyFormula` error that an empty formula gives is returned (decided in planning, research R7). It never crashes.
3. **Given** `\!\!\!x`, whose negative space exceeds the content width, **When** rendered, **Then** the viewBox width is clamped to a non-negative value, with no NaN and no negative width.
4. **Given** spacing inside a superscript (`x^{a\,b}`), a subscript, or a fraction, **When** rendered, **Then** it works, and mu-based spaces are measured at that nested style's size.
5. **Given** thousands of repeated spacing commands, **When** rendered, **Then** the existing input-length and depth limits still apply and the output size stays bounded.
6. **Given** an out-of-scope command (`\left`, `\text`, `\hat`) or an unknown one-character command such as `\@`, **When** rendered, **Then** the unsupported error is still returned, and its hint no longer claims that spacing commands are out of scope.

## Edge Cases

- Spacing as the first or last item inside a group (`{\,x}`, `{x\,}`), including empty groups containing only spacing.
- Spacing directly adjacent to a script (`x\,^2` and `x^2\,`): base and script attachment must be unchanged by the glue.
- Spacing between a big operator and its limits or operand (`\sum\,_i`, `\int\,f`).
- Spacing in a fraction numerator or denominator (centring accounts for the added width).
- Negative space larger than the preceding content (`\!\!\!x`): the total width is clamped, with no negative or NaN output.
- A backslash followed by a space at the end of the input (`x\ `), versus a lone trailing backslash (still an unexpected-end error).
- A lone `\` followed by other punctuation (`\@`, `\%`): still reported as unsupported.
- `\,` immediately followed by a letter or a command (`\,dx`, `\,\alpha`): the lexer must not swallow following characters.
- Whitespace after `\quad` or `\qquad` is still ignored (they are word-style commands).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The typesetter MUST accept `\,`, `\:`, `\;`, `\!`, `\quad`, `\qquad`, and `\ ` (backslash-space) in any position where an atom is accepted: top level, group contents, script arguments, fraction arguments, and root arguments.
- **FR-002**: `\,` MUST add 3/18 of a quad, `\:` 4/18, `\;` 5/18, and `\!` MUST subtract 3/18. These mu-based spaces are measured against the quad of the style in effect at the point where the command appears, so they shrink inside script and scriptscript styles.
- **FR-003**: `\quad` MUST add exactly 1 em and `\qquad` exactly 2 em. `\ ` MUST add one interword space of about 1/3 em. These three are font-relative, not mu-based, so they do NOT shrink in script styles.
- **FR-004**: Explicit spacing MUST be additive with, and independent of, the automatic inter-atom spacing: `a\,+b` has both the thin space and the usual spacing before a binary operator.
- **FR-005**: Spacing MUST be transparent to atom classification. Binary-operator demotion, unary-sign detection, and inter-atom spacing lookup MUST yield the same results as if the spacing were absent, so the class of an atom's neighbours is determined by the nearest real atoms and not by the spacing.
- **FR-006**: Spacing MUST not affect script attachment: in `x\,^2` or `x^2\,`, base and script keep their normal relationship.
- **FR-007**: The rendered SVG MUST remain pure vector (glyph outlines as `<path>` definitions drawn through internal `<use>` references; no text, fonts, scripts, or external references). Spacing itself emits no drawing element.
- **FR-008**: The SVG viewBox MUST include explicit spacing wherever it affects layout, including leading and trailing spacing. Total width MUST be clamped to a non-negative value, and no coordinate or dimension may be NaN or infinite.
- **FR-009**: Output MUST remain deterministic: identical input gives byte-identical SVG.
- **FR-010**: Lexing MUST consume exactly the command characters: `\,` and friends consume one symbol character; `\quad` and `\qquad` follow existing word-command rules (so `\quadx` remains an unknown command named `quadx`, as it is in TeX).
- **FR-011**: Unsupported commands (`\left`, `\right`, `\text`, accents, unknown names) MUST continue to produce an unsupported-command error with name and position. The hint for unknown single-symbol commands MUST no longer say spacing is out of scope, and MUST not mention the newly supported commands as unsupported.
- **FR-012**: Existing size/depth limits MUST continue to apply, and repeated spacing MUST NOT allow unbounded output or runtime.
- **FR-013**: The crate README MUST list the supported spacing commands and state their scaling behaviour. The integral example MUST be covered by an updated reference snapshot, whose SVG structure was inspected before acceptance.
- **FR-014**: Behaviour of every formula that rendered before this feature MUST be unchanged (byte-identical output).

### Key Entities

- **Spacing item**: A horizontal gap in a formula, with a signed width given either as a count of math units (style-relative) or as a font-relative length (fixed). It has no glyph, no atom class, and no effect on neighbouring atoms' classes.
- **Formula layout box**: The overall extent of the formula, whose width and viewBox include all spacing items.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `\int_a^b f(x)\,dx` renders successfully and reproducibly, and the 0 → 1 change from "error" to "valid SVG" is covered by an automated test.
- **SC-002**: For each of the seven commands, the width difference against the unspaced formula matches the specified amount (3/18, 4/18, 5/18, -3/18, 1, 2, ~1/3 em) within 0.01 em.
- **SC-003**: In 100% of tested binary-operator contexts (leading, trailing, before relation, after open bracket, between atoms), the classification and automatic spacing with spacing present equal those without it.
- **SC-004**: Across all edge cases listed above, plus randomised inputs mixing spacing commands with existing constructs, 0 panics and 0 occurrences of NaN, infinity, or negative size in the output.
- **SC-005**: All pre-existing LaTeX typesetter tests and snapshots pass unchanged, other than the intentionally updated hint text and the new integral snapshot.
- **SC-006**: Mu-based spaces inside a superscript are measurably smaller than the same space at the base size, while `\quad` inside a superscript is not.

## Assumptions

- **TeX semantics decide scaling.** In TeX, `\,`, `\:`, `\;`, `\!` are `\mskip` glue measured in mu, where 1mu is 1/18 of the quad of the math symbol font **at the size of the current style**, so they shrink in script and scriptscript styles. `\quad`, `\qquad`, and `\ ` are ordinary glue in em or interword units of the surrounding text font, so they do not shrink with script style. This is the rule adopted here.
- Base size: 1em is the formula's base (text-size) em. The quad for mu is the font's em at the style's size, consistent with how the existing automatic inter-atom spacing is already converted.
- Stretch and shrink components of `\:` and `\;` are ignored. Only their natural width is used, since there is no line breaking or justification.
- The interword space is fixed at 1/3 em (the classic TeX value for the default font), not read from font data.
- Spacing is glue: like TeX's math list conversion, it does not update the "previous atom class" and so is invisible to Bin demotion and to the inter-atom spacing table. It does not break binary-operator context.
- Trailing or leading spacing is kept in the viewBox, because the user asked for it. A formula consisting only of spacing returns `EmptyFormula`, the same error as empty input (decided in planning, research R7).
- Negative total width is clamped to zero for the viewBox.
- Stated out of scope: `\hspace{..}`, `\hskip`, `\mkern`, `\mskip` with arbitrary dimensions, `\phantom`/`\hphantom`, `\left`/`\right`, `\text`, accents, and line breaking. These remain unsupported errors.
- No new dependencies are needed. Only the existing math module, its tests, snapshot, error hint, and README change.
