# Contract: stretchy-glyph layout (crate-internal)

There is no new public API. The user-visible surface is unchanged: `latex` rendering to SVG. This contract is the crate-internal boundary that the next feature (`\left \right`) will depend on.

## `Layout::stretchy`

```text
fn stretchy(&self, gid: GlyphId, required: f64, style: Style) -> Result<LBox, MathError>
```

`required` is the minimum total vertical extent (height + depth) in em at the style's scale.

**Selection order** (FR-005): (1) base glyph if its ink extent ≥ `required`; (2) smallest variant with extent ≥ `required`; (3) assembly; (4) base glyph.

| Input | Result |
|---|---|
| `required` ≤ base extent | base glyph |
| between variants | smallest variant ≥ `required` |
| beyond largest variant, assembly present | assembly with smallest extender count reaching `required` (≤ `MAX_EXTENDERS`) |
| no variants and no assembly | base glyph, unchanged, `Ok` |
| NaN, ≤ 0, −∞ | base glyph |
| +∞ or absurdly large | capped assembly (tallest allowed), `Ok` |

**Postconditions**
- Returned `height + depth ≥ required`, except when capped or when the glyph cannot stretch that far (tallest achievable).
- Adjacent assembly parts overlap by ≥ `min_connector_overlap`; parts are ordered bottom to top; no gaps.
- Items are `Item::Glyph` only; the box passes `LBox::finite()`.
- Pure and deterministic; never panics.

**Errors**: `MathError::Font` only for a glyph with no outline and no usable data (existing behaviour).

## `Layout::sqrt` (consumer)

Calls `stretchy` with `required = body.height + body.depth + gap + rule`. The vertical-scaling path is used only if the radical glyph has neither variants nor assembly. Final radical height ≥ radicand height + gap (FR-006).

## `Layout::scripts` (kern)

Gains the nucleus glyph id (optional). When `Some(gid)` with `Nucleus::Glyph` and kern data, superscript x is `nucleus.width + nucleus.italic + kern_top_right`, subscript x is `nucleus.width + kern_bottom_right` (R7), where each kern is the sum of the lookups at the script height and the nucleus corner height. Otherwise placement is bit-for-bit what it is today.

## Font subset contract (`scripts/subset-font.sh`)

- Input: pinned STIX Two Math, SHA-256 `95bc2729…8d90b` (existing check).
- Output: deterministic, < 250 KB, family "AgentCanvas Math".
- Post-condition (checked by script and `tests/latex_font.rs`): every glyph referenced by retained variants, assembly parts and kern-bearing glyphs exists in the subset.
