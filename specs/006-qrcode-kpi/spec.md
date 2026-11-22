# Feature Specification: QR Code Generator and KPI Metric Cards

**Feature Branch**: `feat/qrcode-kpi` (existing branch; no branch created or switched)

**Created**: 2026-11-22

**Status**: Implemented

**Issue**: #10 — pure-Rust QR Code generator emitting vector SVG, plus executive KPI metric cards.

**Input**: "Implement a pure-Rust QR Code generator emitting vector SVG, plus executive KPI
metric cards. QR: byte/numeric/alphanumeric encoding with automatic best-mode selection,
versions 1..=10, ECC L/M/Q/H with correct capacity tables, GF(2^8) Reed-Solomon, block
interleaving, function patterns, format/version info, all 8 masks with standard 4-rule
penalty scoring, configurable quiet zone, `QrCode::encode(data)?.with_size(px).to_svg()`,
`to_path` and `to_rects`. KPI: `KpiCard::new(title, value).with_delta(delta, positive)
.with_theme(..).to_svg()` with semantic delta colours and light/dark themes, no `<text>`."

## Summary

Add a new `component` module to `agent-canvas` with two pure-vector business components:

1. `component::qrcode` — an ISO/IEC 18004 QR Code generator (versions 1–10, ECC L/M/Q/H,
   numeric/alphanumeric/byte modes) that renders deterministic SVG as a single merged
   `<path>` or per-module `<rect>` elements.
2. `component::kpi_card` — an executive KPI card with title, large value, caption and a
   green/red/neutral delta badge, rendered from bundled glyph outlines (no `<text>`).

Both are dependency-free at runtime beyond the crate's existing `ttf-parser` dependency and
emit byte-identical output for identical inputs.

## Reference sources

Correctness is anchored to published data rather than self-consistency:

| Reference | Used for |
| --- | --- |
| Thonky QR Code Tutorial, "Error Correction Coding" <https://www.thonky.com/qr-code-tutorial/error-correction-coding> | "HELLO WORLD" 1-M data codewords (32, 91, 11, …) and the ten EC codewords (196, 35, 39, …) |
| Thonky, "Error Correction Code Words and Block Information" <https://www.thonky.com/qr-code-tutorial/error-correction-table> | Per-version/level block layouts (EC codewords per block, group sizes) for versions 1–10 |
| Thonky, "Format and Version String Tables" <https://www.thonky.com/qr-code-tutorial/format-version-tables> | All 32 format-info bit strings and version-info strings for versions 7–10 |
| Thonky, "Character Capacities by Version, Mode, and Error Correction" <https://www.thonky.com/qr-code-tutorial/character-capacities> | Capacity assertions (1-L numeric 41, 1-M byte 14, 1-Q alphanumeric 16, 2-H alphanumeric 20, 10-H byte 119, 10-L byte 271) |
| ISO/IEC 18004 | Mask predicates, penalty rules, function-pattern geometry, dark module |

The 21×21 bitmaps in the tests were generated independently by **segno 1.6.6** and
**python-qrcode 8.2** (`version=1`, `mask_pattern` fixed, `border=0`); both agree for masks
0, 5 and 7 at 1-Q (and for all eight masks in a development-time sweep of 192
payload/ECC/mask combinations, where the implementation matched python-qrcode on every
case). Thonky publishes the 1-M codeword stream and a rendered image, but not a textual
1-Q bitmap, so the literal bitmaps are cited as cross-implementation references rather
than as a printed spec table. End-to-end output was additionally decoded by **zxing-cpp**
during development for numeric, alphanumeric, byte, dark-theme symbols and versions 1, 3,
5, 7, 9 and 10.

## User Scenarios & Testing

### User Story 1 — Link to a generated QR Code (P1)

An agent needs to attach a scannable QR Code to a report or chat message without external
services or native libraries.

**Independent test**: `QrCode::encode("https://example.com")?.to_svg()` returns a valid SVG
whose modules decode back to the payload; byte output is identical across renders.

**Acceptance scenarios**

1. Given any non-empty payload that fits version 10, when encoded, then the smallest
   version and the lowest-penalty mask are selected deterministically.
2. Given an explicit ECC level, when encoded, then the format information encodes that
   level and a decoder reports it.
3. Given an explicit mask, when rendered, then the symbol matches the published reference
   matrix for that mask.
4. Given an oversized payload, when encoded, then a typed `QrError::DataTooLong` is
   returned and no panic occurs.

### User Story 2 — Control output geometry and theme (P1)

The same symbol must be embeddable at any pixel size, with or without a quiet zone, and in
light or dark reports.

**Acceptance scenarios**

1. `with_size(px)` scales the square output; invalid sizes are ignored rather than
   producing NaN geometry.
2. `with_quiet_zone(n)` adds exactly `n` light modules on all four sides (default 4).
3. `to_path()` emits one `<path>` with merged horizontal runs; `to_rects()` emits one
   `<rect>` per dark module; `to_svg()` wraps either in a complete document.
4. Light and dark themes preserve contrast and emit no `<text>` or `font-family`.

### User Story 3 — Report a KPI at a glance (P2)

An executive summary needs a card with a title, a large value and a coloured delta badge.

**Acceptance scenarios**

1. Light and dark themes render the same content with palette-appropriate colours.
2. Positive, negative and neutral deltas use green `#059669`, red `#DC2626` and the theme's
   secondary text colour respectively.
3. Long values shrink (40 px down to 18 px) instead of overflowing the card.
4. Output is deterministic and contains only `<svg>`, `<rect>` and `<path>` elements.

### Edge cases

- Empty input: `QrError::EmptyInput`.
- Non-ASCII payload: UTF-8 byte mode (no ECI is emitted; see limitations).
- Payload exactly at capacity: succeeds; one character more: `DataTooLong`.
- `with_size(NaN)` / negative size: previous size retained.
- Quiet zone above 16 modules: clamped to 16.
- Font data unavailable at runtime: text drawing is a no-op returning zero width; no panic.

## Functional Requirements

- **FR-001**: `QrCode::encode(&str) -> Result<QrCode, QrError>` selects numeric when all
  characters are digits, alphanumeric when every character is in the QR alphanumeric set,
  and byte (UTF-8) otherwise.
- **FR-002**: Versions 1–10 are supported (`17 + 4·version` modules); capacity and block
  tables match ISO/IEC 18004 Table 9.
- **FR-003**: ECC levels L/M/Q/H use GF(2⁸) arithmetic modulo `0x11D`, the standard
  Reed-Solomon generator polynomials, per-block EC computation and codeword interleaving.
- **FR-004**: Function patterns include three finders with separators, timing patterns,
  alignment patterns (version ≥ 2), the dark module, and format (BCH 15,5) and version
  (BCH 18,6, version ≥ 7) information.
- **FR-005**: All eight masks are applied to data modules only; the symbol is scored with
  the four ISO penalty rules and the minimum-penalty mask is selected. An explicit mask can
  be requested for fixtures.
- **FR-006**: Rendering supports a configurable quiet zone (default 4 modules), arbitrary
  positive SVG size, light/dark themes and custom colours. Output contains no `<text>`,
  `font-family`, scripts or external references.
- **FR-007**: Errors are a `thiserror` enum (`QrError`), `#[non_exhaustive]`; library code
  contains no panics and no `unwrap`/`expect`. Invalid render sizes are ignored and quiet
  zones are clamped rather than producing new error variants.
- **FR-008**: `KpiCard` renders title, value, optional caption and a delta badge with
  semantic colours, in light and dark themes, from glyph outlines.
- **FR-009**: Both types are exported from `component` and re-exported at the crate root.

## Public API

```rust
use agent_canvas::component::{EccLevel, KpiCard, QrCode, QrMode};

let qr = QrCode::encode("https://example.com")?
    .with_size(320.0)
    .with_quiet_zone(4)
    .with_theme(agent_canvas::canvas::ThemeMode::Light);
let svg = qr.to_svg();
let path = qr.to_path();      // single merged <path>
let rects = qr.to_rects();    // one <rect> per dark module
let capacity = QrCode::capacity(1, EccLevel::Q, QrMode::Alphanumeric)?; // 16

let card = KpiCard::new("Monthly Revenue", "Rp 1.24B")
    .with_delta("+12.4%", true)
    .with_caption("vs. last month")
    .with_theme(agent_canvas::canvas::ThemeMode::Dark)
    .to_svg();
```

## Out of scope

- Kanji mode, Micro QR, structured append, FNC1/ECI headers.
- Versions 11–40 (the block table is deliberately limited to 1–10; adding rows is
  mechanical but requires new capacity tests).
- Error correction decoding (the round-trip reader is test-only).
- PNG/raster output, logos or rounded module shapes.

## Test plan (files)

| Requirement | Tests |
| --- | --- |
| Reference matrices (masks 0, 5, 7) | `component::qrcode::tests::reference_matrix_*`, `reference_matrices_cover_multiply_based_masks` |
| Published RS codewords / generator polynomial | `gf::tests::rs_remainder_matches_published_hello_world_1m_codewords`, `generator_polynomial_matches_published_1m_coefficients`, `tests::published_hello_world_1m_codewords_and_rs` |
| Format and version strings | `gf::tests::published_format_strings`, `published_version_information_strings`, `tests::format_information_is_written_for_every_level_and_mask` |
| Round-trip decode (numeric/alphanumeric/byte, all ECC, lengths, every version) | `round_trip_*`, `every_mask_produces_a_decodable_symbol` |
| Matrix invariants | `matrix_invariants_hold_for_every_version`, `alignment_patterns_are_present_from_version_2` |
| Capacity | `capacity_matches_published_character_capacities` |
| SVG properties | `svg_is_deterministic_and_vector_only`, `svg_quiet_zone_and_geometry`, `theme_colors_are_applied` |
| Errors | `errors_are_typed_and_never_panic` |
| KPI card | `kpi_card::tests::*`, `text::tests::*` |
| Public API surface | `tests/component_tests.rs` |

## Success criteria

- `cargo test -p agent-canvas` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo fmt -p agent-canvas --check` passes.
- QR output decodes with an independent reader (zxing-cpp verified during development).
- Ten consecutive renders of the same symbol produce identical bytes.
