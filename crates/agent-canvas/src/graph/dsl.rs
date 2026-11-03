use super::edge::{Edge, EdgeStyle};
use super::node::{Node, NodeShape};
use crate::error::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopToBottom,
    LeftToRight,
}

/// Maximum allowable DSL input length (128 KB) to guard against unbounded allocation DoS
const MAX_DSL_BYTES: usize = 128 * 1024;
const MAX_NODES: usize = 1_000;
const MAX_EDGES: usize = 2_000;

/// Tokenizes and parses a Mermaid-like flowchart DSL string into Nodes and Edges.
pub fn parse_dsl(dsl: &str) -> Result<(Direction, Vec<Node>, Vec<Edge>)> {
    if dsl.len() > MAX_DSL_BYTES {
        return Err(crate::CanvasError::InvalidArgument(format!(
            "Flowchart DSL input ({} bytes) exceeds maximum limit of {} bytes",
            dsl.len(),
            MAX_DSL_BYTES
        )));
    }
    let mut direction = Direction::TopToBottom;
    let mut nodes_map: HashMap<String, Node> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut node_order: Vec<String> = Vec::new();

    for line in dsl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") || trimmed.starts_with('#') {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        if let Some(&first) = words.first() {
            if first == "flowchart" || first == "graph" {
                if words.contains(&"lr") {
                    direction = Direction::LeftToRight;
                } else {
                    direction = Direction::TopToBottom;
                }
                continue;
            }
        }

        // Check if line contains arrows: "-->", "-.->", "->"
        if trimmed.contains("->") {
            parse_connection_line(trimmed, &mut nodes_map, &mut node_order, &mut edges);
        } else {
            // Standalone node definition: e.g. "A[Start Process]"
            if let Some(node) = parse_single_node_token(trimmed) {
                if !nodes_map.contains_key(&node.id) {
                    node_order.push(node.id.clone());
                }
                nodes_map.insert(node.id.clone(), node);
            }
        }
    }

    let nodes = node_order
        .into_iter()
        .filter_map(|id| nodes_map.remove(&id))
        .collect::<Vec<Node>>();

    if nodes.len() > MAX_NODES {
        return Err(crate::CanvasError::InvalidArgument(format!(
            "Node count ({}) exceeds maximum safe limit of {}",
            nodes.len(),
            MAX_NODES
        )));
    }
    if edges.len() > MAX_EDGES {
        return Err(crate::CanvasError::InvalidArgument(format!(
            "Edge count ({}) exceeds maximum safe limit of {}",
            edges.len(),
            MAX_EDGES
        )));
    }

    Ok((direction, nodes, edges))
}

fn parse_single_node_token(token: &str) -> Option<Node> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }

    // Process [Label]
    if let Some(open) = t.find('[') {
        if let Some(close) = t.rfind(']') {
            if close > open {
                let id = t[..open].trim();
                let label = t[open + 1..close].trim();
                let node_id = if id.is_empty() { label } else { id };
                return Some(Node::new(node_id, label, NodeShape::Process));
            }
        }
    }

    // Terminal (Label)
    if let Some(open) = t.find('(') {
        if let Some(close) = t.rfind(')') {
            if close > open {
                let id = t[..open].trim();
                let label = t[open + 1..close].trim();
                let node_id = if id.is_empty() { label } else { id };
                return Some(Node::new(node_id, label, NodeShape::Terminal));
            }
        }
    }

    // Decision {Label}
    if let Some(open) = t.find('{') {
        if let Some(close) = t.rfind('}') {
            if close > open {
                let id = t[..open].trim();
                let mut label = t[open + 1..close].trim();
                if label.ends_with('?') {
                    label = label[..label.len() - 1].trim();
                }
                let node_id = if id.is_empty() { label } else { id };
                return Some(Node::new(node_id, label, NodeShape::Decision));
            }
        }
    }

    // Bare id
    Some(Node::new(t, t, NodeShape::Process))
}

fn parse_connection_line(
    line: &str,
    nodes_map: &mut HashMap<String, Node>,
    node_order: &mut Vec<String>,
    edges: &mut Vec<Edge>,
) {
    // Splits by arrows, tracking labels: e.g. A -->|Yes| B or A -> B
    // Strategy: scan for arrow markers
    let mut parts: Vec<&str> = Vec::new();
    let mut edge_labels: Vec<Option<String>> = Vec::new();
    let mut edge_styles: Vec<EdgeStyle> = Vec::new();

    let mut remaining = line;
    while let Some(pos) = find_next_arrow(remaining) {
        let (arrow_start, arrow_end, style, label) = pos;
        let left_part = remaining[..arrow_start].trim();
        parts.push(left_part);
        edge_styles.push(style);
        edge_labels.push(label);
        remaining = remaining[arrow_end..].trim();
    }
    parts.push(remaining);

    for (i, p) in parts.iter().enumerate() {
        if let Some(node) = parse_single_node_token(p) {
            if !nodes_map.contains_key(&node.id) {
                node_order.push(node.id.clone());
            }
            nodes_map.entry(node.id.clone()).or_insert(node);
        }

        if i > 0 {
            let from_node = parse_single_node_token(parts[i - 1]);
            let to_node = parse_single_node_token(p);
            if let (Some(f), Some(t)) = (from_node, to_node) {
                let mut edge = Edge::new(&f.id, &t.id).with_style(edge_styles[i - 1]);
                if let Some(ref l) = edge_labels[i - 1] {
                    edge = edge.with_label(l);
                }
                edges.push(edge);
            }
        }
    }
}

fn find_next_arrow(s: &str) -> Option<(usize, usize, EdgeStyle, Option<String>)> {
    let arrow_patterns = ["-.->", "-->", "->"];
    let mut earliest: Option<(usize, usize, EdgeStyle, Option<String>)> = None;

    for pat in arrow_patterns {
        if let Some(idx) = s.find(pat) {
            let is_earlier = match earliest {
                Some((earliest_idx, ..)) => idx < earliest_idx,
                None => true,
            };

            if is_earlier {
                let after_pat = idx + pat.len();
                let style = if pat.contains('.') {
                    EdgeStyle::Dotted
                } else {
                    EdgeStyle::Solid
                };

                let rest = &s[after_pat..];
                if let Some(stripped) = rest.strip_prefix('|') {
                    if let Some(end_pipe) = stripped.find('|') {
                        let label = stripped[..end_pipe].trim().to_string();
                        let total_end = after_pat + 1 + end_pipe + 1;
                        earliest = Some((idx, total_end, style, Some(label)));
                        continue;
                    }
                }
                earliest = Some((idx, after_pat, style, None));
            }
        }
    }

    earliest
}
