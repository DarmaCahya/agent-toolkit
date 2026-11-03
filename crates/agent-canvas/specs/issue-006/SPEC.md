# SpecKit: Issue #6 — Core Geometric Primitives, Font Metrics & Pure SVG Writer

## 1. Specify
- **Crate**: `agent-canvas`
- **Location**: `crates/agent-canvas/src/canvas/`
- **Goal**: Membangun modul geometri dasar (`Point`, `Size`, `Rect`, `Stroke`, `Fill`), modul tema/palet warna (`Color`, `ThemeMode`, Okabe-Ito 8-color palette), modul aproksimasi font metrics (`FontMetrics` proportional sans-serif), dan `SvgCanvas` (XML-safe, zero-panic, zero-allocation builder).
- **Invariants**:
  - `#![forbid(unsafe_code)]`
  - Deterministic SVG string output.
  - Zero external runtime / zero OS font requirements.
  - XML escaping for special characters (`&`, `<`, `>`, `"`, `'`).
  - Graceful math bounds: no NaN/Infinity escapes to SVG coordinates.

## 2. Plan
1. **Module `canvas::geom`**:
   - `Point { x: f32, y: f32 }`
   - `Size { width: f32, height: f32 }`
   - `Rect { origin: Point, size: Size, rx: f32, ry: f32 }`
   - `Stroke { color: Color, width: f32, dash_array: Option<String> }`
   - `Fill { color: Color, opacity: f32 }`
2. **Module `canvas::theme`**:
   - `Color { r: u8, g: u8, b: u8, a: f32 }` with hex parsing (`#RGB`, `#RRGGBB`, `#RRGGBBAA`).
   - `ThemeMode`: `Light`, `Dark`.
   - Categorical 8-color Okabe-Ito / Tremor palette.
3. **Module `canvas::font`**:
   - `FontMetrics`: Proportional character width table (ASCII + common Latin + fallback CJK width).
   - Method `estimate_text_width(text: &str, font_size: f32) -> f32`.
   - Method `estimate_text_height(font_size: f32) -> f32`.
4. **Module `canvas::svg`**:
   - `SvgCanvas`: canvas dimensions, viewbox, defs (gradients, markers), elements buffer (`rect`, `circle`, `line`, `path`, `text`, `group`).
   - Method `to_svg(&self) -> String`.
5. **Re-export in `canvas::mod`** and `lib.rs`.

## 3. Tasks
- [x] Create directory `crates/agent-canvas/src/canvas/`
- [ ] Implement `geom.rs`
- [ ] Implement `theme.rs`
- [ ] Implement `font.rs`
- [ ] Implement `svg.rs`
- [ ] Wire up `mod.rs` and `lib.rs`
- [ ] Add unit tests and integration contract tests
- [ ] Audit & verify with Claude CLI
- [ ] Commit & push via Gemini model credentials
