# Data Model

All lengths are em (font units ÷ units-per-em), y up. Types are `pub(crate)` in `crates/agent-canvas/src/math/latex/`.

## `AssemblyPart` (font.rs)

| Field | Type | Source / rule |
|---|---|---|
| `gid` | `GlyphId` | `GlyphPart.glyph_id` |
| `start_connector` | `f64` | `start_connector_length / upem`, ≥ 0 |
| `end_connector` | `f64` | `end_connector_length / upem`, ≥ 0 |
| `advance` | `f64` | `full_advance / upem`, parts with advance ≤ 0 are dropped |
| `extender` | `bool` | `part_flags.extender()` |

## `Assembly` (font.rs)

| Field | Type | Rule |
|---|---|---|
| `parts` | `Vec<AssemblyPart>` | font order, bottom to top; never empty (else `assembly()` returns `None`) |
| `min_overlap` | `f64` | `Variants.min_connector_overlap / upem` |

Accessor: `MathFont::assembly(gid) -> Option<Assembly>`. It returns `None` when the glyph has no vertical construction, no assembly, or an empty or unusable part list.

## `CornerKern` (font.rs)

| Field | Type | Rule |
|---|---|---|
| `heights` | `Vec<f64>` | ascending correction heights, em |
| `kerns` | `Vec<f64>` | `heights.len() + 1` entries, em |

Accessor: `MathFont::corner_kern(gid, Corner) -> Option<CornerKern>` with `Corner::{TopRight, BottomRight}`. Empty height tables with no kerns return `None`. `CornerKern::at(h) -> f64` applies the boundary rule from research R7 (first index `i` with `h < heights[i]`, else last kern; non-finite `h` returns the last kern for `+∞`, the first for `−∞`, 0 for NaN).

## `Stretch` result (box_model.rs)

`stretchy` returns an `LBox` (existing type). Items are `Item::Glyph` entries only, in the assembly case stacked bottom to top. No new `Item` variant.

## Constants (box_model.rs)

| Name | Value | Purpose |
|---|---|---|
| `MAX_EXTENDERS` | 64 | extender repeat cap (research R4) |
| `MAX_RADICAL_STRETCH` | 200.0 (unchanged) | guard for the remaining scaled fallback |

## Validation rules

- Joint overlap ∈ [`min_overlap`, `max(min_overlap, min(A.end, B.start))`].
- Required parts appear exactly once; extenders `n` times, `n ≤ MAX_EXTENDERS`.
- Non-finite or non-positive `required` → base glyph.
- Every produced coordinate passes the existing `LBox::finite()` check.

## State transitions

None; every function is pure over the immutable shared font.
