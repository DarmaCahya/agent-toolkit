use super::dsl::{parse_dsl, Direction};
use super::edge::{Edge, EdgeStyle};
use super::layout::compute_layout;
use super::node::{Node, NodeShape};
use super::routing::route_orthogonal_edge;
use crate::canvas::{Fill, Palette, Path, Point, Rect, Size, Stroke, SvgCanvas, ThemeMode};
use crate::error::Result;
use std::collections::HashMap;

/// High-level Flowchart & Architecture diagram generator.
#[derive(Debug, Clone)]
pub struct Flowchart {
    pub title: Option<String>,
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub theme: ThemeMode,
}

impl Flowchart {
    pub fn new(direction: Direction) -> Self {
        Self {
            title: None,
            direction,
            nodes: Vec::new(),
            edges: Vec::new(),
            theme: ThemeMode::Light,
        }
    }

    /// Parses a Mermaid-like flowchart DSL string into a fully configured `Flowchart`.
    pub fn from_dsl(dsl_str: &str) -> Result<Self> {
        let (dir, nodes, edges) = parse_dsl(dsl_str)?;
        Ok(Self {
            title: None,
            direction: dir,
            nodes,
            edges,
            theme: ThemeMode::Light,
        })
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_theme(mut self, mode: ThemeMode) -> Self {
        self.theme = mode;
        self
    }

    pub fn add_node(mut self, node: Node) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn add_edge(mut self, edge: Edge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Computes layout, orthogonal routes, and renders the diagram to deterministic SVG.
    pub fn to_svg(&self) -> Result<String> {
        let mut nodes = self.nodes.clone();
        let (canvas_w, canvas_h) = compute_layout(&mut nodes, &self.edges, self.direction);

        let palette = Palette::new(self.theme);
        let mut canvas = SvgCanvas::new(canvas_w, canvas_h)
            .with_view_box(0.0, 0.0, canvas_w, canvas_h)
            .with_background(palette.background());

        // 1. Add Arrowhead Marker in `<defs>` (Solution to Issue #6 Probe Feedback!)
        let arrow_color = palette.border().to_rgb_hex();
        let marker_xml = format!(
            r#"<marker id="arrowhead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 1.5 L 10 5 L 0 8.5 z" fill="{}" /></marker>"#,
            arrow_color
        );
        canvas.add_def(marker_xml);

        // 2. Title
        if let Some(ref t) = self.title {
            canvas.draw_text(
                Point::new(40.0, 35.0),
                t,
                18.0,
                palette.text_primary(),
                "start",
            );
        }

        // Fast node lookup map
        let node_map: HashMap<String, &Node> = nodes.iter().map(|n| (n.id.clone(), n)).collect();

        // 3. Draw Edges with Manhattan Orthogonal Routing
        for edge in &self.edges {
            if let (Some(from_node), Some(to_node)) =
                (node_map.get(&edge.from_id), node_map.get(&edge.to_id))
            {
                let (path, label_pos) = route_orthogonal_edge(from_node, to_node, self.direction);

                let dash_array = match edge.style {
                    EdgeStyle::Solid => None,
                    EdgeStyle::Dotted => Some("4 4".to_string()),
                };

                // In SVG, stroke attributes can reference marker-end
                let edge_stroke = Stroke {
                    color: palette.border(),
                    width: 1.8,
                    dash_array,
                };

                // Add path element with arrowhead marker
                let path_d = path.to_svg_d();
                let dash_attr = match edge.style {
                    EdgeStyle::Solid => String::new(),
                    EdgeStyle::Dotted => " stroke-dasharray=\"4 4\"".to_string(),
                };
                let edge_svg = format!(
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="1.8"{} marker-end="url(#arrowhead)"/>"#,
                    path_d,
                    edge_stroke.color.to_rgb_hex(),
                    dash_attr
                );
                canvas.elements.push(edge_svg);

                // Draw Edge Label badge if present
                if let Some(ref lbl) = edge.label {
                    let text_w = crate::canvas::FontMetrics::estimate_text_width(lbl, 11.0);
                    let badge_w = text_w + 14.0;
                    let badge_h = 20.0;
                    let badge_rect = Rect::new(
                        Point::new(label_pos.x - badge_w / 2.0, label_pos.y - badge_h / 2.0),
                        Size::new(badge_w, badge_h),
                    )
                    .with_radius(4.0, 4.0);

                    // Badge background to mask edge line
                    canvas.draw_rect(
                        badge_rect,
                        Some(Stroke::new(palette.border(), 1.0)),
                        Some(Fill::solid(palette.surface())),
                    );
                    canvas.draw_text(
                        Point::new(label_pos.x, label_pos.y + 4.0),
                        lbl,
                        11.0,
                        palette.text_primary(),
                        "middle",
                    );
                }
            }
        }

        // 4. Draw Nodes with Shapes & Optical Centering
        for (i, node) in nodes.iter().enumerate() {
            let fill_color = node.color.unwrap_or_else(|| palette.get_categorical(i));
            let stroke_color = fill_color;

            match node.shape {
                NodeShape::Process => {
                    // Rounded rect (rx=6)
                    let r = node.rect.with_radius(6.0, 6.0);
                    canvas.draw_rect(
                        r,
                        Some(Stroke::new(stroke_color, 1.8)),
                        Some(Fill::new(fill_color, 0.15)),
                    );
                }
                NodeShape::Terminal => {
                    // Pill stadium (rx = height / 2)
                    let rad = node.rect.size.height / 2.0;
                    let r = node.rect.with_radius(rad, rad);
                    canvas.draw_rect(
                        r,
                        Some(Stroke::new(stroke_color, 2.0)),
                        Some(Fill::new(fill_color, 0.20)),
                    );
                }
                NodeShape::Decision => {
                    // Diamond polygon path
                    let c_x = node.rect.origin.x + node.rect.size.width / 2.0;
                    let c_y = node.rect.origin.y + node.rect.size.height / 2.0;
                    let w_2 = node.rect.size.width / 2.0;
                    let h_2 = node.rect.size.height / 2.0;

                    let mut diamond = Path::new();
                    diamond = diamond.move_to(Point::new(c_x, c_y - h_2)); // Top
                    diamond = diamond.line_to(Point::new(c_x + w_2, c_y)); // Right
                    diamond = diamond.line_to(Point::new(c_x, c_y + h_2)); // Bottom
                    diamond = diamond.line_to(Point::new(c_x - w_2, c_y)); // Left
                    diamond = diamond.close();

                    canvas.draw_path(
                        &diamond,
                        Some(Stroke::new(stroke_color, 1.8)),
                        Some(Fill::new(fill_color, 0.15)),
                    );
                }
            }

            // Optical Vertical & Horizontal Center Text (Solution to Issue #6 Probe Feedback!)
            let font_size = 13.0;
            let text_x = node.rect.origin.x + node.rect.size.width / 2.0;
            // Baseline optical offset is ~0.35 * font_size below mathematical center
            let text_y = node.rect.origin.y + node.rect.size.height / 2.0 + (font_size * 0.35);

            canvas.draw_text(
                Point::new(text_x, text_y),
                &node.label,
                font_size,
                palette.text_primary(),
                "middle",
            );
        }

        Ok(canvas.to_svg())
    }
}
