# SpecKit: Issue #8 — AI-Friendly Flowchart DSL Parser & Manhattan 90° Routing

## 1. Specify
- **Crate**: `agent-canvas`
- **Location**: `crates/agent-canvas/src/graph/`
- **Goal**: Membangun modul parsing flowchart DSL toleran kesalahan AI dan layout solver bertingkat dengan Manhattan 90° orthogonal arrow routing serta arrowhead marker terintegrasi.
- **Invariants**:
  - `#![forbid(unsafe_code)]`
  - Zero external runtime / zero browser / pure Rust.
  - Zero unhandled panics on malformed DSL (unclosed brackets, trailing arrows, cycle branches).
  - Arrowheads `<marker>` generated deterministically in `<defs>`.
  - Automatic optical vertical & horizontal centering of node labels.
  - Orthogonal routing ensures connecting lines never penetrate node bodies.

## 2. Plan
1. **Module `graph/node.rs`**:
   - `NodeShape`: `Process` (Rect with rounded corners), `Terminal` (Stadium/Pill), `Decision` (Diamond rhombus).
   - `Node`: id, text, shape, explicit color, computed rect/bounding box.
2. **Module `graph/edge.rs`**:
   - `Edge`: from_id, to_id, label (optional e.g. "Yes", "No"), style (Solid, Dotted).
3. **Module `graph/dsl.rs`**:
   - Resilient tokenizer & line-by-line parser for `flowchart TD` / `flowchart LR`.
   - Node syntax: `[Text]` (rect), `(Text)` (pill), `{Text}` (diamond).
   - Connection syntax: `-->`, `->`, `-.->` with optional `|Label|`.
4. **Module `graph/layout.rs`**:
   - Layered rank assignment (topological ordering / breadth-first levels).
   - Dynamic auto-padding based on `FontMetrics::estimate_text_width`.
   - Optical centering calculation.
5. **Module `graph/routing.rs`**:
   - Orthogonal Manhattan 90° route calculator: connects exit anchor of source node to entry anchor of target node with intermediate right-angle elbows avoiding collision.
6. **Module `graph/render.rs`**:
   - `Flowchart` struct: parses DSL or constructs programmatically, calculates layout & routing, emits deterministic `<svg>`.

## 3. Tasks
- [x] Create directory layout `crates/agent-canvas/src/graph/` and specs
- [ ] Implement `node.rs`, `edge.rs`
- [ ] Implement `dsl.rs` (tokenizer & resilient parser)
- [ ] Implement `layout.rs` (layered ranking & box sizing)
- [ ] Implement `routing.rs` (Manhattan 90° elbows)
- [ ] Implement `render.rs` (`Flowchart` facade to SVG with `<marker>`)
- [ ] Wire up `graph/mod.rs` and `lib.rs`
- [ ] Unit & integration tests in `crates/agent-canvas/tests/graph_tests.rs`
- [ ] Claude adversarial audit loop & fix
- [ ] Real-world user probe mini-project & multimodal visual evaluation
- [ ] PR, CI verification & auto-merge to `main`
