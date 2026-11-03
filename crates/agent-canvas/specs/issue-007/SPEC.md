# SpecKit: Issue #7 — Executive Business Charts

## 1. Specify
- **Crate**: `agent-canvas`
- **Location**:
  - `crates/agent-canvas/src/math/` (Isolated math foundation)
  - `crates/agent-canvas/src/chart/` (Public charting engine & components)
- **Goal**: Membangun mesin grafik bisnis komprehensif 7 tipe (Donut/Pie, Bar/Column grouped & stacked, Line/Area, Waterfall, Sparkline, Quadrant Matrix 2x2, Sales Funnel).
- **Invariants**:
  - `#![forbid(unsafe_code)]`
  - Zero external runtime / pure Rust / sub-millisecond execution.
  - Zero `NaN` or `Infinity` float panics (`is_finite()` guards everywhere).
  - Clean separation: `src/math/` contains NO graphics/SVG code (preserves Issue #12 future extraction).
  - Composable API on `SvgCanvas` + clean one-liner shorthands for agents.

## 2. Plan
1. **Module `math/`**:
   - `scale.rs`: `LinearScale` (continuous numerical mapping) & `CategoryScale` (discrete band slotting).
   - `ticks.rs`: Heckbert's "Nice Numbers" algorithm for automated round tick generation.
   - `polar.rs`: Angle conversions, radians, and arc path trigonometry.
2. **Module `chart/cartesian/`**:
   - `plot.rs`: `CartesianPlot` frame (axes, gridlines, nice number labels, margins).
   - `bar.rs`: `BarSeries` (single, grouped, stacked).
   - `line.rs`: `LineSeries` (bezier smooth curve, area fill).
   - `waterfall.rs`: `WaterfallSeries` (financial bridge floating bars).
   - `sparkline.rs`: `Sparkline` (frameless inline mini trends).
3. **Module `chart/polar/`**:
   - `donut.rs`: `DonutChart` & `PieChart` with center metric summary & auto percentage legend.
4. **Module `chart/matrix/`**:
   - `quadrant.rs`: `QuadrantMatrix` (2x2 BCG / Eisenhower / Risk decision matrix).
5. **Module `chart/funnel/`**:
   - `funnel.rs`: `FunnelChart` (stepped conversion trapezoids with drop-off tracking).

## 3. Tasks
- [x] Create directory layout for `math/` and `chart/`
- [ ] Implement `math/scale.rs`, `math/ticks.rs`, `math/polar.rs`, `math/mod.rs`
- [ ] Implement `chart/cartesian/` (plot, bar, line, waterfall, sparkline)
- [ ] Implement `chart/polar/donut.rs`
- [ ] Implement `chart/matrix/quadrant.rs`
- [ ] Implement `chart/funnel/funnel.rs`
- [ ] Wire up `chart/mod.rs` and `lib.rs`
- [ ] Comprehensive unit & integration tests
- [ ] Claude CLI adversarial review & bug fix loop
- [ ] Mini-project user-centric real-world probe & evaluation
- [ ] PR, CI verification & auto-merge to `main`
