"""
Claude runner to generate robust flowchart graph engine:
- node.rs
- edge.rs
- dsl.rs
- layout.rs
- routing.rs
- render.rs
"""
import subprocess, os

CLAUDE_BIN = "/home/agent/.hermes/bin/claude"
BASE_DIR = "/srv/hermes/workspace/unified-crate/crates/agent-canvas"

prompt = """
You are a Principal Rust Systems Engineer building `agent-canvas::graph`.
It compiles under #![forbid(unsafe_code)], zero external runtimes, zero unhandled panics.
Uses `crate::canvas::{SvgCanvas, Point, Rect, Size, Path, Stroke, Fill, Color, Palette, ThemeMode, FontMetrics}`.

Please generate the following 6 files for `crates/agent-canvas/src/graph/`:

=== FILE: node.rs ===
- `NodeShape`: `Process` (rounded rect rx=6), `Terminal` (stadium pill rx=height/2), `Decision` (diamond polygon).
- `Node`: id: String, label: String, shape: NodeShape, color: Option<Color>, rect: Rect.
- Methods: new(id, label, shape), with_color(self, color).

=== FILE: edge.rs ===
- `EdgeStyle`: `Solid`, `Dotted`.
- `Edge`: from_id: String, to_id: String, label: Option<String>, style: EdgeStyle.
- Methods: new(from, to), with_label(self, label), with_style(self, style).

=== FILE: dsl.rs ===
- AI-forgiving DSL parser for Mermaid-like flowchart syntax:
  - Header: `flowchart TD`, `flowchart LR`, `graph TD`, `graph LR` (case-insensitive).
  - Node definitions / references:
    - `id[Label text]` -> Process
    - `id(Label text)` -> Terminal
    - `id{Label text}` -> Decision
    - Bare `id` -> Process with label = id
  - Connection syntax:
    - `A --> B`, `A -> B`, `A -.-> B`
    - With labels: `A -->|Yes| B` or `A -- Yes --> B` or `A --> B : Yes`
    - Chaining: `A --> B --> C`
  - Resilient: Skips empty lines, ignores invalid tokens gracefully without panicking.
  - Returns `Result<(Direction, Vec<Node>, Vec<Edge>), crate::CanvasError>`.
  - Direction: enum `Direction { TopToBottom, LeftToRight }`.

=== FILE: layout.rs ===
- Layered rank assignment & auto-padding layout solver:
  - Assigns nodes to discrete ranks (layers 0, 1, 2...) using topological sort / longest path from roots.
  - Computes node dimensions dynamically using `FontMetrics::estimate_text_width(label, 14.0)` + padding (min width 120, height 45 for Process/Pill; min 140x70 for Diamond).
  - For `TopToBottom`:
    - Ranks stacked vertically (layer_gap ~60px).
    - Nodes in each layer arranged horizontally (node_gap ~40px) centered relative to widest layer.
  - For `LeftToRight`:
    - Ranks arranged horizontally (layer_gap ~80px).
    - Nodes in each layer arranged vertically (node_gap ~35px).
  - Returns updated `Vec<Node>` with exact `Rect` coordinates.

=== FILE: routing.rs ===
- Manhattan 90° orthogonal arrow routing solver:
  - For edge connecting source `Node` to target `Node`:
    - In `TopToBottom`:
      - If source is directly above target: simple vertical line from `(src.center_x, src.bottom)` to `(tgt.center_x, tgt.top)`.
      - If target is on different column: exit `(src.center_x, src.bottom)` -> vertical down by half gap -> horizontal to `tgt.center_x` -> vertical down to `(tgt.center_x, tgt.top)`.
      - If target is on same rank or upstream (loop back): exit side -> route around bounding boxes with 90° corners.
    - In `LeftToRight`:
      - Horizontal exit from `src.right` -> 90° elbow -> entry at `tgt.left`.
  - Returns `(Path, Option<(Point, String)>)` where the second element is the midpoint position for the edge label text.

=== FILE: render.rs ===
- `Flowchart`:
  - title: Option<String>, direction: Direction, nodes: Vec<Node>, edges: Vec<Edge>, theme: ThemeMode
  - Methods:
    - new(direction)
    - from_dsl(dsl_str) -> Result<Self, crate::CanvasError>
    - title(self, title), with_theme(self, mode)
    - add_node(mut self, node), add_edge(mut self, edge)
    - to_svg(&self) -> Result<String, crate::CanvasError>
  - In `to_svg`:
    - Adds SVG arrowhead `<marker id="arrowhead" ...>` in `<defs>`.
    - Renders all orthogonal edge paths with `marker-end="url(#arrowhead)"`.
    - Renders edge label badges with background rect to ensure readable text over lines.
    - Renders all node shapes (Rect with rx=6, Pill with rx=h/2, Diamond polygon path).
    - Uses exact optical vertical centering for node text: `text_y = rect.origin.y + rect.size.height / 2.0 + font_size * 0.35` with `text-anchor="middle"`.
"""

print("Running Claude CLI for flowchart graph engine...")
res = subprocess.run([CLAUDE_BIN, "-p", prompt], capture_output=True, text=True, timeout=300)
if res.returncode != 0:
    print("Claude failed:", res.stderr)
    exit(1)

content = res.stdout
parts = content.split("=== FILE: ")
for part in parts[1:]:
    lines = part.split("\n", 1)
    file_rel = lines[0].strip().replace("===", "").strip()
    file_code = lines[1].strip()
    if file_code.startswith("```rust"):
        file_code = file_code[7:]
    if file_code.startswith("```"):
        file_code = file_code[3:]
    if file_code.endswith("```"):
        file_code = file_code[:-3]
    file_code = file_code.strip()

    target_path = os.path.join(BASE_DIR, "src/graph", file_rel)
    os.makedirs(os.path.dirname(target_path), exist_ok=True)
    with open(target_path, "w") as f:
        f.write(file_code + "\n")
    print(f"Wrote {target_path} ({len(file_code)} chars)")
