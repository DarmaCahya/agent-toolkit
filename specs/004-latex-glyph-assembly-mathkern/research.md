# Research: Glyph Assembly and Math Kerning

No `NEEDS CLARIFICATION` items remain. Sources: `ttf-parser` 0.25.1 source (`src/tables/math.rs`, read directly from the local cargo registry) and the OpenType MATH specification.

## R1. Font data API

- **Decision**: Use `ttf_parser::math` already enabled in `Cargo.toml`: `Table.variants: Option<Variants>` (`min_connector_overlap: u16`, `vertical_constructions.get(gid) -> Option<GlyphConstruction>`); `GlyphConstruction { assembly: Option<GlyphAssembly>, variants }`; `GlyphAssembly { italics_correction, parts: LazyArray16<GlyphPart> }`; `GlyphPart { glyph_id, start_connector_length, end_connector_length, full_advance, part_flags.extender() }`; `Table.glyph_info.kern_infos.get(gid) -> Option<KernInfo { top_right, bottom_right, .. }>`; `Kern { count(), height(i), kern(i) }` with `count` heights and `count + 1` kerns.
- **Rationale**: Verified from source, not assumed (spec Assumption). No dependency change.
- **Alternatives**: parsing MATH by hand, rejected as duplicated work and more panic surface.

## R2. Assembly algorithm

- **Decision**: Follow the OpenType MATH spec and TeX. Parts are ordered bottom to top. Required parts appear once; extenders repeat `n` times. Let `o_min = min_connector_overlap` and, per joint between parts A (below) and B (above), `o_max = min(A.end_connector_length, B.start_connector_length)`. Joint overlap is clamped to `[o_min, o_max]`, and `o_max` is raised to at least `o_min` so the lower bound always wins.
  - `min_height(n) = Σ full_advance − Σ o_max`; `max_height(n) = Σ full_advance − Σ o_min`.
  - Choose the smallest `n ≥ 1` (or `0` when the assembly has no extenders) with `max_height(n) ≥ required`.
  - When `min_height(n) ≤ required ≤ max_height(n)`, overlaps are reduced from their maximum by the same fraction at every joint to reach `required` exactly. When `required < min_height(n)`, use `o_max` everywhere, so the result is slightly taller than requested.
- **Rationale**: Matches FR-003/FR-004. The spec calls for uniform distribution and a smallest-count rule.
- **Alternatives**: always `o_min` (visibly gappy on curved parts), per-joint greedy fill (non-uniform, harder to test).

## R3. Variant choice and the existing byte-identical requirement

- **Decision**: `stretchy` first compares `required` against the base glyph ink height, then against the sorted variant list, using the **same measure the radical uses today** (ink height `y_max − y_min` times style scale) so that fit-in-variant inputs choose the identical glyph and position (FR-008). Assembly is only used when no variant is tall enough.
- **Rationale**: keeps current snapshots unchanged.
- **Alternatives**: switch to the `advance_measurement` in the variant record (the spec-intended measure), rejected here because it would change currently selected variants and break FR-008.

## R4. Caps and degenerate input

- **Decision**: `MAX_EXTENDERS = 64`. `required` that is NaN, negative, zero or infinite maps to the base glyph (infinite is treated as "tallest allowed", capped). Assemblies with no extenders, or all parts flagged extender, return the single/maximal arrangement and never loop. A non-positive total advance returns the base glyph.
- **Rationale**: Output size and time stay bounded. Whether 64 extenders reach 100× the base height depends on the font's extender advances and is **not assumed**: a test records the maximum reachable height per glyph, and SC-003's height guarantee applies up to that cap. Beyond it the capped assembly is returned.
- **Alternatives**: return an error on huge heights, rejected by FR-014 and spec edge cases (capped result, not an error).

## R5. Font subset closure

- **Decision**: Extend the unicode list in `scripts/subset-font.sh` with the delimiter and brace-family base characters `[ ] { } |` (U+005B, 005D, 007B, 007D, 007C) alongside the existing `( ) √ ∑ ∫ ∏`. `pyftsubset` is expected to close over MATH variants and assembly parts; this is **verified, not assumed**: after subsetting, the script (and the `latex_font.rs` test) walks every retained variant/assembly part id and fails if any is missing from the glyph set. If fontTools does not close, add `--glyphs` for the missing parts. Remove `GDEF` from the drop list only if kern data proves to depend on it; MathKern lives inside `MATH`, so this is not expected.
- **Rationale**: Closure is the exact failure mode that would silently break assembly.
- **Alternatives**: pull in the whole STIX variant range, rejected on size (current file is 21.8 KB, budget 250 KB, so there is plenty of room if a few extra glyphs are needed).

## R6. Radical rendering shape

- **Decision**: Assembly parts are emitted as normal `Item::Glyph` items at `scale = style scale`, stacked vertically, with the x origin aligned. `Item::ScaledGlyph` remains in the enum for glyphs with neither variants nor assembly (FR-007 allows this), so `render.rs` changes are none or minimal. `MAX_RADICAL_STRETCH` stays as the guard for that remaining path.
- **Rationale**: no SVG schema change, smallest blast radius.
- **Alternatives**: delete `ScaledGlyph`, rejected: STIX variants/assembly may be missing for some glyph and the fallback is permitted.

## R7. Kern lookup rule

- **Decision**: For corner `top_right` (superscript) and `bottom_right` (subscript), read the `Kern` table for the nucleus gid at height `h` (em, converted to font units). Select index `i` = number of table heights `≤ h` **excluding equality**, i.e. the first `i` where `h < height(i)`; if none, `i = count`; kern = `kern(i)`. This is the spec's rule "height exactly equal to a table height uses the entry just above." An empty table (`count == 0`) with a single kern value applies that value; a missing table means zero kern.
  - Superscript height: bottom of the superscript relative to the baseline (`u − sup.depth`). The OpenType spec also adds the nucleus-top-relative lookup. Following the spec Assumption, both lookups are summed: `kern(corner, script_bottom) + kern(corner, nucleus_top)` for the superscript, and `kern(bottom_right, script_top) + kern(bottom_right, nucleus_bottom)` for the subscript, using the sign convention from the MATH spec.
  - Kern is added to the script x offset on top of existing italic correction handling. The existing code already shifts a glyph-nucleus superscript by italic correction, so kern is added after it and nothing is applied twice (FR-011).
- **Rationale**: Directly encodes FR-009 to FR-011 and the boundary rule.
- **Alternatives**: replace italic correction with kern when present, rejected: the spec says combine, and TeX/MathML engines add both.

## R8. Determinism and tests

- **Decision**: No hash maps. Part order is font order, variant sort already stable (`total_cmp` then gid). Byte-identity tests render each formula several times. The subset script is run twice and compared in the maintainer checklist (quickstart).
- **Rationale**: Principle III.
