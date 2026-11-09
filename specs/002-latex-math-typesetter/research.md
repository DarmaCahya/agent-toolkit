# Research: LaTeX Math Typesetter

Facts verified in this repo: `thiserror = "2"` and `proptest` (std only) are already dependencies of `agent-canvas`; `ttf-parser 0.25.1` is already in `Cargo.lock` (transitive); `insta` is not present; `math/mod.rs` currently exports `excel, polar, scale, ticks`; the crate root only has `#![forbid(unsafe_code)]` (no clippy deny-lints yet); no `build.rs`.

Items marked **(verify)** are from general knowledge of the libraries and must be confirmed by a failing/passing build in the first implementation tasks.

## R1 — Layout algorithm
- **Decision**: Implement TeX Appendix G rules (11, 13, 15a–e, 18) using OpenType MATH constants, as in the spec; atoms → boxes bottom-up with an explicit `Style` (D/T/S/SS + cramped).
- **Rationale**: Spec-mandated; MATH table exposes the equivalent of TeX's sigma/xi parameters (ScriptPercentScaleDown, FractionNumeratorShiftUp, RadicalVerticalGap, etc.).
- **Alternatives**: Port a full engine (e.g. ReX/RaTeX-style) — too large; ad-hoc layout — fails FR-014–FR-017.

## R2 — ttf-parser features
- **Decision**: `ttf-parser = { version = "0.25", default-features = false, features = ["opentype-layout"] }`. **(verify)** The MATH table (`Face::tables().math`) is gated behind `opentype-layout`. If the build needs float functions, add `std` (preferred) rather than `no-std-float`.
- **Rationale**: Narrowest feature set (Principle VI); already in the lockfile so downstream lockfiles aren't constrained further. No glyph-names/variable-fonts/apple-layout needed.
- **Alternatives**: `rustybuzz`/`swash`/`skrifa` — heavier; hand-rolled MATH parser — ~hundreds of lines of unsafe-prone binary parsing, rejected.

## R3 — Glyph outlines to SVG paths
- **Decision**: Use `Face::outline_glyph` with a custom `OutlineBuilder` that emits path data in font units, y-flipped; each unique glyph id becomes one `<path id="g{gid}">` in `<defs>`, placed with `<use href="#g{gid}" transform=…>`. Store defs in a `BTreeMap<GlyphId, String>` for deterministic order.
- **Rationale**: FR-003/FR-005. `<use>` with `href` only references in-document ids (not external).
- **Alternatives**: Inline every outline — larger output; `<symbol>` — no benefit.

## R4 — Deterministic number formatting
- **Decision**: Round to 2 dp by converting to fixed-point integers (`(x*100).round() as i64`) then print integer/fraction by hand; strip trailing zeros; normalise `-0` to `0`; reject non-finite values (internal error, not output).
- **Rationale**: Avoids platform float-format differences (Principle III, SC-003) and NaN/inf in output.
- **Alternatives**: `format!("{:.2}")` — deterministic in practice but trailing zeros/`-0` need handling anyway.

## R5 — Radical stretch and fallback
- **Decision**: Read `MathVariants` vertical construction for `√`; pick the smallest variant whose advance ≥ required height; if none (and no assembly is used in v1), scale the largest variant's outline vertically via a transform. Assembly-based growth is **not** implemented in v1 (spec: scaled drawing).
- **Rationale**: FR-016, US2 scenario 6. Simple and bounded.
- **Alternatives**: Full glyph assembly — more code, not required.

## R6 — Font subset, renaming, licence
- **Decision**: `scripts/subset-font.sh` uses `uvx --from fonttools pyftsubset` with an explicit unicode list (derived from FR-007–FR-009 plus size-variant/extender glyphs via `--glyphs`/layout closure as needed), keeps the `MATH` table, drops hinting and unneeded tables, then renames name records (family "AgentCanvas Math", no "STIX Two" Reserved Font Name) with fontTools `ttx`/`TTFont` script, pinning options and `SOURCE_DATE_EPOCH`-style fixed `head.modified` for reproducibility. Ship `OFL-1.1.txt` and a THIRD_PARTY_NOTICES entry.
- **Rationale**: FR-021–FR-024; STIX Two Math is OFL-1.1 with Reserved Font Names, so a modified subset must be renamed.
- **Blocker/assumption**: The spec's "stated local cache path" for the upstream font is not actually stated. Needed from the user: path to `STIXTwoMath-Regular.otf`, or approval to fetch it from the upstream STIX Fonts repository. Pin its SHA-256 in the script.
- **Alternatives**: Latin Modern Math (GUST licence) / Fira Math — spec fixed STIX Two Math.

## R7 — Reproducibility of the subset
- **Decision**: Script writes to a temp path, then the test `font_subset_matches_script_output` is optional/CI-skipped without `uvx`; the always-run test checks committed asset properties (size, MATH constants, renamed family, glyph coverage). Byte-for-byte regeneration (SC-006) is checked by running the script twice and comparing hashes.
- **Rationale**: `uvx` is developer-only (Principle I); tests must not require Python.

## R8 — Error design
- **Decision**: `#[non_exhaustive] enum MathError` with variants: `EmptyFormula`, `InputTooLong{len,max}`, `NestingTooDeep{pos,max}`, `UnexpectedEof{pos,expected}`, `UnexpectedChar{pos,ch}`, `UnknownCommand{pos,name}`, `Unsupported{pos,name,hint}`, `MissingArgument{pos,command}`, `DoubleScript{pos,kind}`, `MissingGlyph{ch}`, `Font{reason}` (bad embedded font; should be unreachable). Positions are char (not byte) indices. A `position()` accessor returns `Option<usize>`.
- **Rationale**: FR-002, FR-010, Principle V; SC-007.

## R9 — Recursion safety
- **Decision**: Depth counter in the parser (≤ 64, counting groups, fraction/root arguments, and script arguments); layout is recursive over a tree already bounded by that depth, so no extra guard is needed. Reject length > 8192 chars before lexing.
- **Rationale**: FR-018/FR-019. Stack use at depth 64 is small.

## R10 — Testing approach
- **Decision**: `insta` snapshots of full SVG for a handful of formulas (quadratic, `x_i^2`, `\sum`, `\sqrt[3]{x}`); bbox golden tests with tolerance using a layout-level API (`#[doc(hidden)] pub fn layout_metrics` on `MathTypesetter`, since integration tests in `tests/` cannot see `pub(crate)` items) for `test_math_nested_fractions` and `test_math_sqrt_stretch`; proptest for arbitrary strings (≥10,000 cases, case count set explicitly) and for generated deeply nested input; scan of outputs for `<text`, `font-family`, `<script`, `href="http`, `on\w+=`.
- **Rationale**: FR-025, SC-001/002/004.
- **Note**: `insta` default features pull in extra deps; use `default-features = false` and confirm snapshot assertions still work **(verify)**.
