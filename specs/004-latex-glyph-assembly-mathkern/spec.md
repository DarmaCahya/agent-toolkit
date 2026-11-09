# Feature Specification: LaTeX Glyph Assembly and Math Kerning

**Feature Branch**: `feat/latex-glyph-assembly` (existing branch; no branch created or switched)

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "Extend the merged LaTeX typesetter (crates/agent-canvas/src/math/latex): (1) OpenType MATH glyph ASSEMBLY for radicals and big operators/delimiters that exceed the largest pre-drawn variant, instead of the current vertical-scaling fallback, exposed as a reusable stretchy-glyph layout function for the upcoming `\left \right` feature; (2) MathKern corner kerning for superscript/subscript placement when the nucleus glyph has kern data; (3) update the committed font subset so assembly parts and kern data are available, deterministic, under 250 KB, still named AgentCanvas Math. Out of scope: `\left \right` parsing, `\text`, accents, matrices."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Tall radicals stay crisp, not stretched (Priority: P1)

A user (or agent) renders a square root over a very tall radicand, such as `\sqrt` of a stack of 12 nested fractions. Today, when the radicand is taller than the biggest pre-drawn radical, the radical sign is a single glyph scaled vertically, which distorts stroke thickness and the hook. After this feature, the radical is built from the font's designed pieces (bottom hook, repeated vertical extension, top piece) so stroke weight stays uniform at any height.

**Why this priority**: This is the visible defect. Tall radicals are common in nested-fraction and continued-fraction formulas, and the distortion is noticeable.

**Independent Test**: Render `\sqrt` over 12 nested `\frac` levels and inspect the output: it must succeed, the radical must cover the content height plus its vertical gap, and the radical must be made of assembled pieces with no distorted single glyph.

**Acceptance Scenarios**:

1. **Given** `\sqrt` of 12 nested fractions, **When** rendered, **Then** the radical sign is assembled from font-provided pieces and no vertically scaled single glyph is used.
2. **Given** that same render, **When** the radical's total height is measured, **Then** it is at least the radicand height plus the required vertical gap.
3. **Given** the assembled radical, **When** adjacent pieces are examined, **Then** each adjacent pair overlaps by at least the font's minimum connector overlap, leaving no visible gap, and pieces are ordered bottom to top.
4. **Given** the same input rendered twice, **When** outputs are compared, **Then** the SVG bytes are identical.
5. **Given** a radicand that fits within an existing pre-drawn radical variant, **When** rendered, **Then** the output is unchanged from today (the variant is chosen, no assembly).

---

### User Story 2 - A reusable stretchy-glyph builder for delimiters and big operators (Priority: P1)

The next planned feature (`\left ... \right`) needs to grow parentheses, brackets, braces, vertical bars, and integral signs to arbitrary heights. This feature provides one reusable layout capability that, given a glyph and a required height, returns a box of at least that height using the best pre-drawn variant or, when none is tall enough, an assembly of parts. Radicals use the same capability.

**Why this priority**: It is the shared foundation for `\left \right`. Without it the follow-up feature would duplicate this logic. It ranks equal to Story 1 because Story 1 is its first consumer.

**Independent Test**: Request stretchy boxes for parenthesis, bracket, brace, vertical bar, integral, and radical glyphs at a range of heights (smaller than the base glyph, between variants, larger than the largest variant) and verify each result's height is at least the requested height and that it never fails or panics.

**Acceptance Scenarios**:

1. **Given** a stretchable glyph and a required height at or below the base glyph's height, **When** a stretchy box is requested, **Then** the base glyph is returned.
2. **Given** a required height between pre-drawn variants, **When** requested, **Then** the smallest variant that meets the height is returned.
3. **Given** a required height beyond the largest variant, **When** requested, **Then** an assembly at least that tall is returned, and its extender repeat count is the smallest that reaches the height.
4. **Given** a glyph with no variants and no assembly data, **When** requested, **Then** the base glyph is returned unchanged (never an error or panic).
5. **Given** an absurdly large, zero, negative, or non-finite required height, **When** requested, **Then** the result is bounded (extender count is capped) and no panic, NaN, or unbounded allocation occurs.

---

### User Story 3 - Scripts hug slanted and cornered nuclei correctly (Priority: P2)

A user writes a superscript or subscript on a nucleus whose outline has a corner that leans or recedes, such as `V^2`, `f_i`, `\int` with limits written as scripts, or an italic capital with a script. The font carries corner-kerning data describing how close a script may come to the nucleus at a given height. After this feature, script placement uses that data so the script sits closer or further away as the glyph shape allows, instead of using only the italic correction.

**Why this priority**: It is a typographic refinement. Output is already acceptable without it, so it follows the tall-radical defect and the shared builder.

**Independent Test**: Render a script on a nucleus glyph that has kern data and the same script on the same glyph with kern data ignored; the horizontal script position differs by the kern amount in the direction the data specifies. Render a nucleus with no kern data and confirm placement is unchanged.

**Acceptance Scenarios**:

1. **Given** a nucleus glyph with top-right kern data and a superscript, **When** rendered, **Then** the superscript's horizontal offset differs from the no-kern placement by the kern value taken at the script's height (summed with the nucleus-corner lookup, per Assumptions), in the direction (closer or further) that the data indicates.
2. **Given** a nucleus glyph with bottom-right kern data and a subscript, **When** rendered, **Then** the subscript is offset by the kern value taken at the script's height.
3. **Given** a nucleus with no kern data, **When** a script is rendered, **Then** placement is identical to today.
4. **Given** both a superscript and a subscript on a kerned nucleus, **When** rendered, **Then** each script is kerned independently by its own height.
5. **Given** the full existing snapshot set, **When** the suite runs, **Then** every snapshot is byte-identical to before, unless a snapshot's formula places a script on a kerned glyph, in which case the changed SVG is inspected and the change is justified in the change record.

---

### User Story 4 - Font subset carries what the new features need (Priority: P2)

A maintainer rebuilds the bundled math font from the upstream source. The resulting subset includes the glyphs for assembly parts of the radical, parentheses, brackets, braces, vertical bar, and integral, plus the kern data, stays small, is reproducible, and keeps the AgentCanvas Math name.

**Why this priority**: Stories 1 to 3 depend on the data being present; it is an enabling, maintainer-facing change.

**Independent Test**: Rebuild the subset twice from the pinned upstream source; verify identical bytes, size under 250 KB, that every glyph referenced by retained variant and assembly data is present in the subset, and that the font tests pass.

**Acceptance Scenarios**:

1. **Given** the pinned upstream font source, **When** the subset is built twice, **Then** the two outputs are byte-identical.
2. **Given** the built subset, **When** its size is checked, **Then** it is under 250 KB.
3. **Given** the built subset, **When** every retained variant and assembly part reference is followed, **Then** every referenced glyph exists in the subset (closure holds).
4. **Given** the built subset, **When** its family name is read, **Then** it is still "AgentCanvas Math".
5. **Given** the built subset, **When** the font tests run, **Then** they assert the new glyph and data coverage and pass.

---

### Edge Cases

- A required height of zero, negative, NaN, or infinity: return the base glyph (or the largest sane result) without error or panic.
- A required height so large that the part count would be huge: the extender repeat count is capped so output size and time stay bounded.
- An assembly whose parts are all flagged as extenders, or has no extenders: handled without panic; if the target cannot be reached without extenders, return the tallest assembly achievable.
- Adjacent parts whose connector overlap allowance is smaller than the glyph's declared minimum: never overlap by less than the minimum; if the allowed overlap range cannot span exactly, distribute the surplus equally among joints so no gap appears.
- A glyph that has variants but is missing an assembly (or the reverse): use what exists, fall back to the base glyph.
- Kern data that is present but has empty height tables: treated as no kern.
- Kern height lookup landing exactly on a correction-height boundary: the rule for boundary values is deterministic and documented.
- Deeply nested input and very long input: existing depth and length limits are unchanged and still produce the same errors.
- Inputs that never contained stretchy or kerned glyphs: output is byte-identical to before.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When a radical needs more height than the largest pre-drawn radical variant, the system MUST build the radical sign from the font's assembly pieces instead of scaling a single glyph vertically.
- **FR-002**: The same behaviour MUST apply to large operators and delimiters (parentheses, brackets, braces, vertical bar, integral) that are asked to stretch beyond their largest variant.
- **FR-003**: An assembly MUST order pieces bottom to top, repeat extender pieces as many times as needed, and overlap each adjacent pair by at least the font's minimum connector overlap and no more than the smaller of the two pieces' relevant connector lengths, so there are no gaps.
- **FR-004**: The extender repeat count MUST be the smallest count whose total height reaches the required height, using the maximum permitted overlap to compute the minimum height and the minimum permitted overlap to compute the maximum; when the target lies between, overlaps are enlarged uniformly to hit the target exactly. Required (non-extender) pieces appear exactly once.
- **FR-005**: The system MUST expose a single reusable stretchy-glyph capability that takes a glyph and a required height and returns a laid-out box at least that tall (or the tallest achievable if the glyph cannot stretch that far). It MUST choose, in order: base glyph if sufficient, smallest sufficient pre-drawn variant, assembly, then base glyph as last resort.
- **FR-006**: Radical layout MUST use that capability, and the final radical height MUST be at least the radicand height plus the vertical gap defined by existing radical layout rules.
- **FR-007**: The vertical-scaling fallback MUST no longer be used for any glyph that has assembly data. It MAY remain only for glyphs without variants or assembly data.
- **FR-008**: Output for any input whose stretchy glyphs fit within existing variants MUST remain byte-identical to current output.
- **FR-009**: Superscript and subscript placement MUST consult the nucleus glyph's corner-kern data (top-right for superscripts, bottom-right for subscripts) at the script's relevant height, and adjust the script's horizontal offset by the looked-up value, when such data exists. Height selection follows the font's correction-height table convention.
- **FR-010**: When the nucleus glyph has no kern data, or the nucleus is not a single glyph, script placement MUST remain exactly as it is today.
- **FR-011**: Kern handling MUST combine correctly with the existing italic correction behaviour, with no double application.
- **FR-012**: The bundled font subset MUST include the assembly piece glyphs and extender glyphs for the radical, parentheses, brackets, braces, vertical bar, and integral, and MUST retain the kern data.
- **FR-013**: The font subset build MUST be deterministic, produce a file under 250 KB, retain the family name "AgentCanvas Math", and be checked for glyph-reference closure (every glyph referenced by retained variant, assembly, and kern data exists in the subset).
- **FR-014**: The system MUST never panic on any input, including malformed or truncated font data, extreme heights, or extreme counts, in line with the project's zero-panic rule.
- **FR-015**: Existing nesting-depth and input-length limits MUST be unchanged.
- **FR-016**: Output MUST remain deterministic: identical input yields identical bytes across runs.
- **FR-017**: The font test suite MUST be updated to assert the new subset content (assembly part glyphs present, kern data present, size bound, name).
- **FR-018**: `\left \right` parsing, `\text`, accents, and matrices MUST NOT be added by this feature.

### Key Entities

- **Stretchy glyph request**: A glyph plus a required height; produces a layout box that satisfies the height by variant selection or assembly.
- **Glyph variant list**: The font's ordered pre-drawn larger forms of a glyph, each with an advance height.
- **Glyph assembly**: Ordered pieces (bottom to top), each flagged required or extender, with start and end connector lengths and a full advance, plus the font-wide minimum connector overlap.
- **Corner kern table**: For a glyph and a corner, a set of heights and kern values; the kern for a given height is chosen from the table.
- **Font subset**: The committed reduced math font containing only needed glyphs and tables.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A radical over 12 nested fractions renders with zero vertically scaled glyphs, and its height is at least the radicand height plus the required gap.
- **SC-002**: In every assembled sign tested, 100% of adjacent pieces overlap by at least the font's minimum connector overlap, and 0 gaps exist.
- **SC-003**: For any required height from zero up to 100 times the base glyph height, the stretchy capability never panics and returns a finite box. The box is at least the required height in 100% of tested cases for glyphs with assembly data, whenever the required height is within what the extender cap allows; beyond that, the tallest capped assembly is returned.
- **SC-004**: The same input rendered 10 times yields byte-identical output every time.
- **SC-005**: 100% of existing reference outputs remain byte-identical, except any whose formula places a script on a kerned glyph; each such change is individually inspected and justified in writing.
- **SC-006**: A script on a kerned glyph is offset from its no-kern position by exactly the font's kern value, in the font-indicated direction, in all tested cases.
- **SC-007**: The bundled font stays under 250 KB and two independent rebuilds are byte-identical.
- **SC-008**: Full project tests and lint gates pass with no new panics-related lint exceptions in library code.

## Assumptions

- The user's instruction to decide without asking and record assumptions applies; no clarification markers are used.
- The bundled font's variant and assembly data exists for the radical, parentheses, brackets, braces, vertical bar, and integral, and the upstream source is the pinned STIX Two Math file named in the request (its checksum is the one recorded in the request). The exact data API of the font-reading dependency will be verified from its official documentation during planning, not assumed.
- Extender repeat count and overlap distribution follow the OpenType MATH specification and TeX's glyph-assembly behaviour: start from the minimum number of extenders and use the largest overlap; add extenders until the maximum achievable height reaches the target, then reduce overlap uniformly to hit the target exactly. The result may exceed the target by no more than the smallest overlap adjustment.
- The assembly's height is measured as the sum of part advances minus overlaps; for horizontal-orientation assemblies, nothing is added (only vertical assemblies are in scope).
- "Required height" for delimiters is supplied by the caller; this feature does not decide delimiter sizing policy (that belongs to `\left \right`).
- Kern value lookup uses the font's correction-height table: values below the first height use the first kern; values at or above the last height use the last kern; a height exactly equal to a table height uses the kern entry just above that height, as the font specification states. The script's height used for lookup is taken at the script's baseline-relative corner per OpenType MATH guidance (superscript: the bottom of the superscript relative to the nucleus baseline and the nucleus top; subscript: the top of the subscript), with both lookups summed as the specification describes.
- Kern is applied only when the nucleus is a single glyph atom with kern data; composite nuclei keep current behaviour.
- Existing snapshots that do not script a kerned glyph are not expected to change. If any snapshot does change because its nucleus glyph has kern data (for example an italic letter with a script), the change is accepted only after the new SVG is inspected and recorded as justified.
- Variant selection compares required height against the same ink-height measure the current radical code uses, not the font's per-variant advance measurement, so that existing output stays byte-identical (FR-008 takes precedence; see research R3).
- Extender repeat count is capped at a fixed bound large enough for any realistic formula and small enough to bound output size; exceeding it returns the capped (tallest allowed) assembly rather than an error.
- Font subset regeneration uses the existing subset script and pinned upstream source; upstream source is gitignored and unchanged, and only the glyph list and retained tables change. If the closure check requires extra glyphs beyond those named, they are added as long as the size limit holds.
- Out of scope: `\left \right` parsing, `\text`, accents, matrices, horizontal stretching (wide accents), and non-vertical assemblies.
