use super::dsl::Direction;
use super::edge::Edge;
use super::node::{Node, NodeShape};
use crate::canvas::{FontMetrics, Point, Rect, Size};
use std::collections::{HashMap, HashSet, VecDeque};

/// Solves discrete ranks and bounding box coordinates for each node in the flowchart.
pub fn compute_layout(
    nodes: &mut [Node],
    edges: &[Edge],
    direction: Direction,
) -> (f32, f32) {
    if nodes.is_empty() {
        return (400.0, 300.0);
    }

    // 1. Calculate dimensions for each node based on label text width
    for node in nodes.iter_mut() {
        let text_w = FontMetrics::estimate_text_width(&node.label, 14.0);
        let (min_w, min_h) = match node.shape {
            NodeShape::Process => ((text_w + 40.0).max(120.0), 45.0),
            NodeShape::Terminal => ((text_w + 50.0).max(120.0), 45.0),
            NodeShape::Decision => ((text_w + 60.0).max(140.0), 65.0),
        };
        node.rect = Rect::new(Point::new(0.0, 0.0), Size::new(min_w, min_h));
    }

    // 2. Rank assignment (topological layers via BFS from in-degree 0 nodes)
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for node in nodes.iter() {
        in_degree.insert(node.id.clone(), 0);
        adj.insert(node.id.clone(), Vec::new());
    }

    for edge in edges {
        if in_degree.contains_key(&edge.to_id) {
            *in_degree.entry(edge.to_id.clone()).or_insert(0) += 1;
        }
        adj.entry(edge.from_id.clone()).or_default().push(edge.to_id.clone());
    }

    let mut ranks: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    let mut in_queue: HashSet<String> = HashSet::new();

    for (id, deg) in &in_degree {
        if *deg == 0 {
            queue.push_back(id.clone());
            in_queue.insert(id.clone());
            ranks.insert(id.clone(), 0);
        }
    }

    // Fallback if graph is completely cyclic: take first node as rank 0
    if queue.is_empty() && !nodes.is_empty() {
        let first_id = nodes[0].id.clone();
        queue.push_back(first_id.clone());
        in_queue.insert(first_id.clone());
        ranks.insert(first_id, 0);
    }

    let mut visited: HashSet<String> = HashSet::new();
    while let Some(curr) = queue.pop_front() {
        in_queue.remove(&curr);
        visited.insert(curr.clone());
        let curr_rank = ranks.get(&curr).copied().unwrap_or(0);

        if let Some(neighbors) = adj.get(&curr) {
            for neighbor in neighbors {
                let neighbor_rank = ranks.entry(neighbor.clone()).or_insert(0);
                // Advance rank but guard maximum rank depth to prevent cycle loops
                if *neighbor_rank < curr_rank + 1 && curr_rank < 500 {
                    *neighbor_rank = curr_rank + 1;
                }

                if !visited.contains(neighbor) && !in_queue.contains(neighbor) {
                    queue.push_back(neighbor.clone());
                    in_queue.insert(neighbor.clone());
                }
            }
        }
    }

    // Ensure all nodes have a rank
    for (i, node) in nodes.iter().enumerate() {
        ranks.entry(node.id.clone()).or_insert(i);
    }

    // Group nodes by rank
    let mut max_rank = 0;
    for &r in ranks.values() {
        max_rank = max_rank.max(r);
    }

    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, node) in nodes.iter().enumerate() {
        let r = ranks.get(&node.id).copied().unwrap_or(0);
        layers[r].push(i);
    }

    // 3. Assign X, Y coordinates
    let margin_l = 60.0;
    let margin_t = 60.0;
    let node_gap = 35.0;
    let layer_gap = 60.0;

    let total_width: f32;
    let total_height: f32;

    match direction {
        Direction::TopToBottom => {
            // Calculate widest layer
            let mut layer_widths: Vec<f32> = Vec::new();
            for layer in &layers {
                let mut w = 0.0;
                for &idx in layer {
                    w += nodes[idx].rect.size.width + node_gap;
                }
                layer_widths.push(w - node_gap);
            }
            let max_layer_w = layer_widths.iter().fold(300.0f32, |a, &b| a.max(b));
            total_width = max_layer_w + margin_l * 2.0;

            let mut curr_y = margin_t;
            for (r, layer) in layers.iter().enumerate() {
                let l_w = layer_widths[r];
                let start_x = margin_l + (max_layer_w - l_w) / 2.0;
                let mut curr_x = start_x;
                let mut layer_max_h = 45.0f32;

                for &idx in layer {
                    let w = nodes[idx].rect.size.width;
                    let h = nodes[idx].rect.size.height;
                    layer_max_h = layer_max_h.max(h);

                    nodes[idx].rect.origin = Point::new(curr_x, curr_y);
                    curr_x += w + node_gap;
                }
                curr_y += layer_max_h + layer_gap;
            }
            total_height = curr_y - layer_gap + margin_t;
        }
        Direction::LeftToRight => {
            let mut curr_x = margin_l;
            let mut max_y_reached = 300.0f32;

            // First find maximum height of all layers to vertically center single nodes
            let layer_heights: Vec<f32> = layers.iter().map(|layer| {
                let mut h = 0.0;
                for &idx in layer {
                    h += nodes[idx].rect.size.height + node_gap;
                }
                (h - node_gap).max(0.0)
            }).collect();
            let max_total_h = layer_heights.iter().fold(200.0f32, |a, &b| a.max(b));

            for (r, layer) in layers.iter().enumerate() {
                let l_h = layer_heights[r];
                // Vertically center layers that have fewer nodes so horizontal lines stay straight
                let start_y = margin_t + (max_total_h - l_h) / 2.0;
                let mut curr_y = start_y;
                let mut layer_max_w = 120.0f32;

                for &idx in layer {
                    let w = nodes[idx].rect.size.width;
                    let h = nodes[idx].rect.size.height;
                    layer_max_w = layer_max_w.max(w);

                    nodes[idx].rect.origin = Point::new(curr_x, curr_y);
                    curr_y += h + node_gap;
                }
                max_y_reached = max_y_reached.max(curr_y - node_gap + margin_t);
                curr_x += layer_max_w + layer_gap;
            }
            total_width = curr_x - layer_gap + margin_l;
            total_height = max_y_reached.max(max_total_h + margin_t * 2.0);
        }
    }

    (total_width.max(300.0), total_height.max(200.0))
}
