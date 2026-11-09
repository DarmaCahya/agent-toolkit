# Feature Specification: LaTeX Math Typesetter (Vector SVG)

**Feature Branch**: `feat/latex-typesetter` (existing branch; no branch created or switched)

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "GitHub issue #9 part 1: pure-Rust LaTeX math-mode to vector SVG typesetter in crates/agent-canvas/src/math/latex. Public entry point `MathTypesetter::render_to_svg(latex) -> Result<String, MathError>`. Must render `\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` correctly. Output is pure vector path SVG with no text elements, no font references, no runtime font files, no scripts."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Render a formula to a self-contained SVG (Priority: P1)

An agent or developer building a canvas/diagram passes a LaTeX math string (for example the quadratic formula `\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`) and receives a single SVG string that displays the typeset formula identically on any viewer, with no fonts installed and no external resources.

**Why this priority**: This is the whole point of the feature; everything else refines it.

**Independent Test**: Call the typesetter with the quadratic formula; verify the SVG is well formed, contains only vector shapes, has a viewBox tightly fitting the formula, and visually matches the reference TeX layout (numerator over denominator, bar between, radical spanning the discriminant, `-` before `b` rendered as a tight unary minus).

**Acceptance Scenarios**:

1. **Given** the quadratic formula, **When** rendered, **Then** the result is a valid SVG showing a fraction with the numerator `-b ± √(b² − 4ac)` centred above a fraction bar and `2a` centred below it.
2. **Given** any valid input in scope, **When** rendered, **Then** the output contains no text elements, no font-family references, no script, and no external references.
3. **Given** the same input rendered twice (or on different platforms), **When** compared, **Then** the SVG bytes are identical.
4. **Given** a leading `-b`, **When** rendered, **Then** the minus is spaced as a unary sign (no space before `b`), whereas `a - b` receives binary-operator spacing.

---

### User Story 2 - Correct math layout: scripts, fractions, roots, large operators (Priority: P2)

A user writes formulas using superscripts, subscripts, nested fractions, roots (with optional index), and big operators (`\sum`, `\int`, `\prod`) and gets TeX-quality layout: scripts shrink and shift properly, nested fractions keep bars aligned and shrink in the right style, radicals grow to cover tall content, and display-style operators place limits above and below.

**Why this priority**: Correct structure beyond the single formula is what makes the typesetter generally useful.

**Independent Test**: Render each construct and check overall size and position of key parts against known reference extents within tolerance.

**Acceptance Scenarios**:

1. **Given** `x^2`, `x_i`, and `x_i^2`, **When** rendered, **Then** scripts are smaller than the base, raised/lowered per TeX rules, and the superscript on an italic letter is nudged right by its italic correction.
2. **Given** a fraction nested in a fraction's numerator, **When** rendered, **Then** the inner fraction is in a smaller style and the bars of nested fractions are vertically aligned consistently with the math axis of their own level.
3. **Given** `\sqrt{x}` and `\sqrt{\frac{a}{b}}`, **When** rendered, **Then** the radical for the taller content is visibly taller than for the short content, with its top rule extending over the content.
4. **Given** `\sqrt[3]{x}`, **When** rendered, **Then** the index appears small and raised at the left of the radical.
5. **Given** `\sum_{i=0}^{n} i` in display style, **When** rendered, **Then** limits are centred above and below the enlarged operator; in inline/script style they appear as ordinary scripts.
6. **Given** a radical needing more height than any pre-built glyph size provides, **When** rendered, **Then** the radical is still drawn at the required height (scaled drawing) rather than failing or clipping.

---

### User Story 3 - Clear, positioned errors and safe behaviour on hostile input (Priority: P2)

A caller (often an autonomous agent) submits malformed, unsupported, or adversarial input and gets a structured error that says what went wrong and where (character position), never a crash, hang, or giant output.

**Why this priority**: The library is consumed by agents processing untrusted text; failures must be recoverable and informative.

**Independent Test**: Feed unbalanced braces, unknown commands, out-of-scope commands, over-long input, and deeply nested input; verify each returns an error variant with a position and a hint, and that random input never panics.

**Acceptance Scenarios**:

1. **Given** `\frac{a}{`, **When** rendered, **Then** an error identifies unexpected end of input and its position.
2. **Given** `\foo` or an out-of-scope construct such as `\left( x \right)`, `\text{hi}`, `\begin{matrix}`, `\hat{x}`, or `\newcommand`, **When** rendered, **Then** an "unsupported/unknown command" error names the command and its position.
3. **Given** input longer than 8192 characters, **When** rendered, **Then** an input-too-long error is returned without attempting layout.
4. **Given** brace/group nesting deeper than 64 levels, **When** rendered, **Then** a nesting-too-deep error is returned (no stack exhaustion).
5. **Given** arbitrary random strings, **When** rendered, **Then** the call returns `Ok` or `Err` and never panics.

---

### User Story 4 - Redistributable, license-compliant bundled math font (Priority: P3)

The maintainers ship the typesetter without requiring any font at runtime. A compact, renamed subset of an open-licensed math font is embedded, with its licence and notices included, and a script regenerates it reproducibly.

**Why this priority**: Required for legal distribution and reproducibility, but invisible to end users once done.

**Independent Test**: Run the regeneration script and confirm the produced subset equals the committed asset; run the tests that check the subset's size, its math-layout data, its renamed family, and the presence of licence and notices files.

**Acceptance Scenarios**:

1. **Given** the committed font asset, **When** inspected, **Then** it is well under 250 KB, still contains the math layout data (constants, size variants, italic corrections), and contains every glyph required by the v1 scope.
2. **Given** the asset's naming data, **When** inspected, **Then** its family name is the renamed one ("AgentCanvas Math") and does not use the original reserved font name.
3. **Given** the repository, **When** inspected, **Then** the font licence text and a third-party notices entry exist alongside the asset.
4. **Given** the regeneration script, **When** run from a clean checkout with the source font available, **Then** it produces the same subset deterministically.

### Edge Cases

- Empty input or whitespace-only input: returns a typed "empty formula" error (see Assumptions).
- Missing braced argument (`x^`, `\frac{a}`, `\sqrt`) → error with position.
- Double scripts (`x^2^3`, `x_1_2`) → error identifying the repeated script.
- Script or limits attached to nothing (`^2` at start) → handled by attaching to an empty base, as TeX does.
- Bin atoms at the start, end, or after another Bin/Rel/Open/Punct become Ord (`-b`, `a = -b`, `(-b)`, `a +`).
- Letter `h` maps to the Planck-constant code point, since the math-italic `h` position is unassigned.
- Unicode input outside the supported set (including non-rendering and tag code points) is rejected with a positioned error rather than silently dropped or passed through.
- Very wide or tall formulas: output stays bounded by the input-size limit; coordinates are finite and never NaN/inf.
- Whitespace in math mode is ignored (TeX semantics); explicit spacing commands are out of scope.
- A glyph missing from the embedded font yields an error, not a blank box.
- Repeated glyphs (e.g. many `a`) share one outline definition, keeping output small.

## Requirements *(mandatory)*

### Functional Requirements

**API & output**

- **FR-001**: The library MUST expose a `MathTypesetter` with a `render_to_svg(latex: &str) -> Result<String, MathError>` operation, located under `crates/agent-canvas/src/math/latex`.
- **FR-002**: `MathError` MUST be a public, non-exhaustive, typed error that carries the character position of the problem (where one applies) and an actionable message.
- **FR-003**: The SVG output MUST consist solely of vector path shapes (glyph outlines and rules). It MUST NOT contain text elements, font-family or other font references, scripts, event handlers, or external links.
- **FR-004**: No font file or other resource may be required at runtime; the glyph data is embedded in the library.
- **FR-005**: Glyph outlines MUST be defined once and reused by reference; numeric coordinates MUST use a bounded decimal precision; output MUST be deterministic (byte-identical for identical input across runs and platforms).
- **FR-006**: The SVG viewBox MUST be derived from the formula's laid-out bounding box (with a small fixed padding), so the formula neither clips nor floats in excess whitespace.

**Input language (v1 scope)**

- **FR-007**: The system MUST accept Latin letters (rendered as math-italic, including the special case for `h`), digits, and the symbols `+ - = < >`, the delimiters `( )` and the comma `,` (classes Open, Close, Punct), plus Greek letters (lower- and upper-case, as in standard TeX), `\pm \cdot \times \leq \geq \neq \infty`.
- **FR-008**: Braces MUST group content; `^` and `_` MUST attach superscripts/subscripts (single token or braced group).
- **FR-009**: `\frac{a}{b}`, `\sqrt{x}`, `\sqrt[n]{x}`, and the big operators `\sum`, `\int`, `\prod` (with `^`/`_` limits) MUST be supported.
- **FR-010**: Unknown commands, out-of-scope constructs (stretchy `\left`/`\right`, matrices/environments, `\text`, accents, macros, `\newcommand`), and unsupported characters MUST yield a positioned error naming the offending item.

**Typesetting behaviour (follows the TeX math-layout rules, Appendix G)**

- **FR-011**: Each atom MUST be assigned a TeX class (Ord, Op, Bin, Rel, Open, Close, Punct), and a Bin atom lacking a valid left operand or followed by a Rel/Close/Punct MUST be demoted to Ord.
- **FR-012**: Spacing between adjacent atoms MUST follow the TeX inter-atom table (thin 3mu, medium 4mu, thick 5mu), with the conditional (medium/thick) spaces suppressed in script styles, and mu scaled by the current style.
- **FR-013**: Four style levels (display, text, script, scriptscript) MUST be supported, with script sizes derived from the font's own scale-down values; cramped variants MUST be used where TeX specifies (denominators, subscripts, radicands).
- **FR-014**: Super/subscript placement MUST follow TeX Rule 18 using the font's math constants, including italic-correction handling for superscripts on italic glyphs.
- **FR-015**: Fractions MUST follow TeX Rules 15a–e using the font's fraction constants (numerator/denominator shifts and gaps, rule thickness, display vs text variants); numerator and denominator MUST be centred on the bar.
- **FR-016**: Radicals MUST follow TeX Rule 11 using the font's radical constants (rule thickness, vertical gap, extra ascender), choosing the smallest pre-built size variant that covers the content, and MUST fall back to a scaled drawing of the radical when no variant is large enough. An optional index MUST be placed using the font's radical-degree constants.
- **FR-017**: In display style, `\sum`, `\int`, `\prod` MUST use the larger display size and place limits above/below per Rule 13 (integral limits remain side scripts, as in TeX default); in other styles they use text size and side scripts.

**Safety limits**

- **FR-018**: Input longer than 8192 characters MUST be rejected up front with a typed error.
- **FR-019**: Nesting deeper than 64 levels (groups, fractions, roots, scripts) MUST be rejected with a typed error; no input may cause stack overflow, infinite loop, or unbounded memory.
- **FR-020**: The library MUST never panic for any input; all failures return `Err`.

**Bundled font & compliance**

- **FR-021**: The embedded font MUST be a subset of STIX Two Math that retains its math layout data (constants, size variants and assemblies, italic corrections) and every glyph needed by FR-007–FR-009 including size variants and radical/operator variants; total size MUST be well under 250 KB.
- **FR-022**: The subset MUST be renamed (family "AgentCanvas Math") and MUST NOT use the original Reserved Font Name.
- **FR-023**: The OFL-1.1 licence text MUST ship in the assets directory and a third-party notices entry MUST attribute the font.
- **FR-024**: A repeatable script (`scripts/subset-font.sh`) MUST regenerate the subset from the upstream font and produce identical output on repeat runs.

**Verification**

- **FR-025**: Tests MUST include: SVG golden/snapshot checks; bounding-box golden checks with tolerance, including `test_math_nested_fractions` (nested fraction bar alignment) and `test_math_sqrt_stretch` (radical grows with inner height); a font test proving the subset has math constants and the renamed family; a test proving outputs contain no text/font-family; and a property test that random input never panics.

### Key Entities *(include if feature involves data)*

- **Formula input**: A LaTeX math-mode string (no surrounding `$`), at most 8192 characters.
- **Atom**: A unit of math with a TeX class (Ord, Op, Bin, Rel, Open, Close, Punct) that determines spacing and Bin demotion.
- **Style**: Display, Text, Script, ScriptScript, each with a cramped flag; controls size and spacing.
- **Box**: A laid-out rectangle with width, height, depth, and drawing content (glyph placements and rules); composed hierarchically.
- **Math font data**: Glyph outlines, advances, and math layout constants/variants/italic corrections of the bundled subset.
- **MathError**: A typed error with a kind, a character position (when applicable), and a hint.
- **SVG document**: The output string: viewBox, a defs section of unique glyph outlines, and the placed uses and rules.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The quadratic formula renders to a valid SVG whose overall width/height and key element positions (bar, radical, numerator/denominator centring) match reference TeX-layout values within a stated tolerance (≤ 3% of formula height).
- **SC-002**: 100% of generated outputs, across the full test corpus, contain zero text elements, zero font references, and zero script content.
- **SC-003**: Rendering the same input 100 times, yields byte-identical output. Cross-platform identity is guaranteed by design (integer fixed-point number formatting, no platform-dependent float formatting); CI currently runs on Linux only, so it is verified there.
- **SC-004**: Property testing with at least 10,000 random inputs (including hostile Unicode and deep nesting) produces zero panics and zero hangs.
- **SC-005**: A typical formula (≤ 200 characters) renders in under 50 ms on a standard developer machine; a maximum-size (8192 character) input either renders or errors in under 2 seconds.
- **SC-006**: The embedded font asset is under 250 KB and the regeneration script reproduces it byte-for-byte.
- **SC-007**: 100% of out-of-scope or malformed inputs in the error-case test list return an error carrying the offending position; none are silently ignored.
- **SC-008**: Nested fraction bars in a two-level fraction align to within 1% of font em of their expected axis positions, and a radical over tall content is at least as tall as the content plus its gap (test passes with the required growth).

## Assumptions

- **Audience**: The primary caller is another component of the agent-canvas crate and autonomous agents; no GUI is involved.
- **Input form**: Input is bare math-mode LaTeX (no `$...$` or `\[...\]` delimiters); a leading/trailing delimiter is treated as an unsupported character.
- **Initial style**: Formulas render in display style by default (so `\sum` limits and large fractions appear as in displayed equations). Inline-style selection is not exposed in v1.
- **Font size / units**: Output coordinates are in a fixed internal unit (1 em = 1000 units, scaled uniformly); callers scale the SVG via width/height. No user-chosen colour: glyphs use `currentColor` so the surrounding document controls colour.
- **Empty input**: Empty or whitespace-only input returns a typed "empty formula" error instead of an empty SVG.
- **Letters**: Latin letters render in math italic; digits and operators upright; Greek lower-case italic, upper-case upright (standard TeX behaviour).
- **Integral limits**: Following TeX's default, `\int` limits are side scripts even in display style; `\sum`/`\prod` use above/below limits.
- **Precision**: Coordinates rounded to 2 decimal places in em/1000 units; chosen to balance size and fidelity.
- **Whitespace** in the input is ignored; explicit spacing commands are out of scope.
- **Growth beyond the font**: Only the radical has a scaled-drawing fallback in v1; other stretchy constructs are out of scope.
- **Dependencies**: Reading the font tables uses a single pure-Rust font-parsing library already decided during research (narrow version/feature set); no C libraries. Adding it will be justified in the PR per the project constitution.
- **Font source & tooling**: The upstream STIX Two Math font and its licence must be supplied by the maintainers (no local path has been established yet; see plan.md open item and research R6) and its SHA-256 pinned in the script; regenerating the subset needs the Python fonttools package via `uvx`, which is a developer-only tool, not a build or runtime requirement.
- **Branching**: Work stays on the current branch `feat/latex-typesetter`; no branch is created.
- **Out of scope for v1** (later parts of issue #9): stretchy delimiters, matrices/environments, `\text`, accents, macros and `\newcommand`, explicit spacing commands, font-style commands (`\mathbf`, `\mathrm`), colour, and an inline-style option.
- **Constitution alignment**: Pure Rust, zero panics (no unwrap/expect/indexing/panic), deterministic output, test-first with user-scenario tests, typed non-exhaustive errors, minimal dependencies.
