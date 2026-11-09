# Snapshot notes (SC-005)

Five insta snapshots changed. In every case the **only** difference is glyph-id renumbering (`g31` → `g36`, etc.) because the rebuilt font subset gained glyphs (`[ ] { } |` plus their variants and assembly parts), which shifts glyph indices. Each old/new pair was compared with ids normalised by order of use: path data (`<defs>`) and every placement in the body are identical. No snapshot moved because of MathKern.

| Snapshot | Change | Justification |
|---|---|---|
| `latex_layout__scripts_xi2` | glyph ids only | outlines and `translate/scale` identical; `x_i^2` shows no kern shift |
| `latex_layout__sum_limits` | glyph ids only | identical outlines and placements |
| `latex_layout__sqrt_index` | glyph ids only | identical; also confirms the radical now routed through `stretchy` is positioned exactly as before (FR-008) |
| `latex_quadratic__quadratic` | glyph ids only | identical outlines and placements |
| `latex_spacing__integral` | glyph ids only | identical outlines and placements |

Caveat for FR-008 ("byte-identical"): the SVG bytes of these five differ by the `gNN` id numbers only. That follows from extending the subset and cannot be avoided without keeping the old glyph order, which `pyftsubset` does not offer.
