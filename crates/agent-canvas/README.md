# agent-canvas

[![crates.io](https://img.shields.io/crates/v/agent-canvas.svg)](https://crates.io/crates/agent-canvas)
[![Documentation](https://docs.rs/agent-canvas/badge.svg)](https://docs.rs/agent-canvas)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Pure-Rust visual computing & calculation artifacts for AI Agents and LLM workflows.
Zero external runtime dependencies (no Node.js, no Chromium, no Python).

## Core Capabilities

1. **Flowcharts & Diagrams (`graph`)**: Decision trees, sequence lifelines, and system architecture blocks with Manhattan 90° routing.
2. **Executive Business Charts (`chart`)**: Donut/Pie, 2x2 Quadrant Matrix (BCG/Eisenhower), Sales Funnel, and Stacked/Grouped Bars.
3. **Math & Formulae (`math`)**: LaTeX math formula typesetting to SVG path curves + Pure-Rust Excel formula evaluator (`=SUM`, `=IF`, etc.).
   - **LaTeX → SVG**: `MathTypesetter::render_to_svg(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")` returns a deterministic, vector-only SVG (glyph outlines, no `<text>`/fonts/scripts) typeset with TeX layout rules and OpenType MATH metrics from a bundled STIX Two Math subset.
   - **v1 grammar**: letters, digits, `+ - = < > ( ) , |`, `^`/`_`, `{}` groups, `\frac`, `\sqrt[n]{}`, `\left … \right`, `\sum \int \prod`, Greek letters, `\pm \cdot \times \leq \geq \neq \infty`. Limits: 8192 chars, nesting depth 64. Out of scope: `\big/\Big` fixed-size delimiters, `\text`, matrices, accents, font/style commands, macros (errors carry a position and hint).
   - **Auto-sizing delimiters**: `\left<delim> … \right<delim>` grows each delimiter to cover its content (vertical size variants first, then a glyph assembly), centres both on the math axis, and is classified as a TeX Inner atom so the ordinary spacing table applies, e.g. `\left( \frac{a}{b} \right)`, `\left\{ \sum_i x_i \right\}`, `\left\langle u, v \right\rangle`. Delimiters: `( ) [ ] { } |`, `\{ \} \|`, `\langle \rangle`, `\lvert \rvert`, `\lVert \rVert`, and `.` (null delimiter, draws nothing). Delimiters may be mismatched as in TeX (`\left( x \right]`); an unclosed `\left`, a stray `\right` or an unknown delimiter returns a located `MathError`. The group takes scripts like any atom (`(…)^2`) and each `\left` counts toward the nesting limit.
   - **Explicit spacing**: `\,` (3mu), `\:` (4mu), `\;` (5mu), `\!` (-3mu), `\quad` (1em), `\qquad` (2em) and backslash-space (1/3em), e.g. `\int_a^b f(x)\,dx`. Mu spaces scale with the script style; em spaces do not. Spacing never changes operator classification or automatic spacing, and takes no scripts. A formula made only of spacing returns `MathError::EmptyFormula`. Still unsupported: `\hspace`, `\phantom`, `\text`, accents.
   - **Glyph assembly and kerning**: radicals and `\left`/`\right` delimiters share one stretchy-glyph builder that picks the smallest size variant that fits, and otherwise builds the glyph from the font's OpenType MATH assembly parts instead of scaling one glyph; assembly is capped at 64 extender repeats (roughly 38 em for the radical), beyond which the capped result is returned. Superscripts and subscripts on a single-character base also honour the font's MathKern corner data.
   - The bundled font is a modified, renamed subset of STIX Two Math under OFL-1.1; see [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md).
4. **Business Components (`component`)**: Pure-Rust QR Code matrix generator + KPI executive metric cards.

## QR Codes and KPI cards (`component`)

Pure-vector business artifacts for reports, dashboards and chat messages. Both components
emit deterministic SVG with no `<text>`, `font-family`, scripts or external references.

### QR Code generator

`QrCode::encode` implements ISO/IEC 18004 versions 1–10 with numeric, alphanumeric and
UTF-8 byte modes. The mode and the smallest fitting version are selected automatically,
Reed-Solomon error correction is computed per block and interleaved, and all eight data
masks are scored with the four standard penalty rules before the best one is used.

```rust
use agent_canvas::component::{EccLevel, QrCode, QrMode};
use agent_canvas::canvas::ThemeMode;

let qr = QrCode::encode("https://example.com/agent-canvas")?
    .with_size(320.0)          // square output in pixels (default 256)
    .with_quiet_zone(4)        // modules on each side (default 4)
    .with_theme(ThemeMode::Light);

let svg = qr.to_svg();         // full document, one merged <path>
let path = qr.to_path();       // just the <path> element
let rects = qr.to_rects();     // one <rect> per dark module
assert_eq!(qr.version(), 3);
assert_eq!(qr.size(), 29);     // 17 + 4 * version
```

- **Error correction**: `EccLevel::{L, M, Q, H}`; `QrCode::encode_with_ecc(data, ecc)`
  selects the level explicitly. `QrCode::encode_with_mask(data, ecc, mask)` fixes the mask
  for reproducible fixtures.
- **Capacity**: `QrCode::capacity(version, ecc, mode)` returns the character capacity
  derived from the ISO codeword tables, e.g. 16 alphanumeric characters for 1-Q.
- **Errors**: `QrError` is a `#[non_exhaustive]` `thiserror` enum — `EmptyInput`,
  `DataTooLong { .. }`, `InvalidMask(..)`, `UnsupportedVersion(..)`. Library code never
  panics; invalid sizes are ignored and quiet zones are clamped.
- **Limits**: versions 1–10 (up to 271 bytes at 10-L), no Kanji, Micro QR, ECI or
  structured append.
- **Tests**: the suite pins reference matrices (masks 0, 5 and 7) produced by two
  independent encoders, Thonky's published "HELLO WORLD" 1-M codewords, all 32 published
  format strings and versions 7–10 version strings, plus a test-only decoder that
  round-trips numeric, alphanumeric and byte payloads across every ECC level and version.

### KPI metric card

```rust
use agent_canvas::component::KpiCard;
use agent_canvas::canvas::ThemeMode;

let svg = KpiCard::new("Monthly Revenue", "Rp 1.24B")
    .with_delta("+12.4%", true)          // green badge
    .with_caption("vs. last month")
    .with_theme(ThemeMode::Dark)
    .to_svg();

let churn = KpiCard::new("Customer Churn", "2.1%")
    .with_delta("+0.4pp", false)         // red badge
    .to_svg();

let flat = KpiCard::new("Headcount", "128")
    .with_neutral_delta("0.0%")          // neutral badge
    .to_svg();
```

The card is laid out in a 320×160 design space and scaled through the SVG view box
(`with_size(640.0, 320.0)` changes only the output dimensions). Titles and labels are
rendered from a bundled DejaVu Sans subset as glyph outlines (see
[`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md)); long values shrink from 40 px
to 18 px instead of overflowing. `with_delta_trend` exposes the full
`DeltaTrend::{Positive, Negative, Neutral}` choice.

### Runnable example

```bash
cargo run -p agent-canvas --example issue_010_demo
```

Writes QR and KPI SVGs to the system temp directory and prints version, ECC, mode, mask and
capacity for each generated symbol.

## Excel formula evaluator

`ExcelEvaluator` evaluates Excel-style formulas deterministically, with Excel's operator quirks
(`=-3^2` is `9`, `=2^3^2` is `64`, `="ABC"="abc"` is `TRUE`). It never panics and `eval` never
returns `Err`: every failure is a `Value::Error` (`#DIV/0!`, `#VALUE!`, `#NAME?`, `#NUM!`).
Use `try_eval` to get a located `ParseError` (kind, byte span, expected items) for malformed input.

`Value::as_str()` returns the text of a `Text` value only (`Option<&str>`); use `to_text()` or
`Display` for the display string of any value. A reference to an empty cell evaluates to
`Value::Blank`, which reads as `0`, `""` or `FALSE` depending on context.

```rust
use agent_canvas::math::{Cell, ExcelEvaluator, RangeProvider, Value};
use agent_canvas::math::excel::ParseErrorKind;

let v = ExcelEvaluator::eval(r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#);
assert_eq!(v.as_str(), Some("Profit"));
assert_eq!(ExcelEvaluator::eval("=1+1").to_text(), "2");

// Malformed input is reported with its position.
let e = ExcelEvaluator::try_eval("=SUM(").unwrap_err();
assert_eq!(e.kind, ParseErrorKind::UnexpectedEof);
assert_eq!(e.span, 5..5);

// References such as A1:A3 resolve through a caller-supplied provider.
struct Column;
impl RangeProvider for Column {
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>> {
        (reference == "A1:A3").then(|| {
            vec![Cell::Value(Value::Number(4.0)), Cell::Empty, Cell::Value(Value::Number(6.0))]
        })
    }
}
let total = ExcelEvaluator::eval_with("=SUM(A1:A3)", &Column);
assert_eq!(total, Value::Number(10.0));
```

Supported functions: `SUM AVERAGE MIN MAX COUNT IF AND OR NOT ABS ROUND IFERROR CONCAT`.
Argument counts are checked at parse time (`SUM()` is a `WrongArgCount` error; variadic
functions need at least one argument). Input is limited to 8192 characters and 64 nesting levels.

## When to use `agent-canvas` (and what it replaces)

| You need | Use | Instead of |
| :--- | :--- | :--- |
| A chart or diagram in an agent's output | `chart::*`, `graph::Flowchart` | matplotlib (Python), Chart.js/D3 (JS runtime), Mermaid CLI (Node) |
| LaTeX math in a document, ticket or report | `math::MathTypesetter` | MathJax/KaTeX (JS), TeX Live (GBs), MathML (uneven renderer support) |
| Spreadsheet logic without a spreadsheet | `math::ExcelEvaluator` | LibreOffice headless, an XLSX engine with a workbook model |
| A scannable QR code in a receipt or ticket | `component::QrCode` | `qrcode` C bindings, an image crate + PNG post-processing |
| An executive KPI tile | `component::KpiCard` | hand-rolled SVG with system fonts |

Every artifact is deterministic vector SVG built from `<path>`/`<rect>`/`<use>`: no `<text>`, no
`font-family`, no external URLs, no runtime font files. Glyph shapes for math and UI text come from
bundled, licenced subsets (see [`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md)) and are
reproducibly regenerated by `scripts/subset-font.sh` / `scripts/subset-sans-font.sh`.

## FAQ

**Can I render LaTeX to SVG in pure Rust, without JavaScript or a TeX installation?**
Yes — `MathTypesetter::new().render_to_svg(r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}")`. Layout follows TeXbook
Appendix G; metrics and glyph outlines come from the bundled STIX Two Math subset via `ttf-parser`.

**Does the SVG depend on fonts installed on the machine?**
No. Math glyphs and UI text are emitted as outlines, so the same file renders identically everywhere.

**Is the output stable enough to snapshot in tests?**
Yes — repeated renders are byte-identical; the test suite asserts this and pins `insta` snapshots.

**What are the hard limits?**
LaTeX: 8,192 characters, nesting depth 64. Excel: 8,192 characters, nesting depth 64, ≤255 arguments,
≤1,000,000 cells per reference. QR: versions 1–10. Exceeding a limit returns a typed error; the library
never panics and never emits non-finite coordinates.

**Which LaTeX commands are *not* supported?**
`\big/\Big/\bigg`, `\text`, matrices/environments, accents, `\hspace`, `\phantom` and macros. They return
a located `Unsupported`/`UnknownCommand` error with a hint rather than rendering something wrong.

## Part of the Agent Workspace Monorepo
Maintained by Darma Cahya (`darmady6@gmail.com`). Machine-readable context for LLM crawlers:
[`llms.txt`](../../llms.txt) and [`llms-full.txt`](../../llms-full.txt).
