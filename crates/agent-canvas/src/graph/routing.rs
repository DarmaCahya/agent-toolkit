use super::dsl::Direction;
use super::node::Node;
use crate::canvas::{Path, Point};

/// Computes an orthogonal 90° Manhattan path between two nodes.
/// Returns `(path, midpoint_for_label)`.
pub fn route_orthogonal_edge(from: &Node, to: &Node, direction: Direction) -> (Path, Point) {
    let mut path = Path::new();

    match direction {
        Direction::TopToBottom => {
            // Source exit point: bottom center
            let src_x = from.rect.origin.x + from.rect.size.width / 2.0;
            let src_y = from.rect.origin.y + from.rect.size.height;

            // Target entry point: top center
            let tgt_x = to.rect.origin.x + to.rect.size.width / 2.0;
            let tgt_y = to.rect.origin.y;

            // Self-loop edge: route loop on the right side
            if from.id == to.id {
                let right_edge = from.rect.origin.x + from.rect.size.width;
                let mid_y = from.rect.origin.y + from.rect.size.height / 2.0;
                path = path.move_to(Point::new(right_edge, mid_y - 10.0));
                path = path.line_to(Point::new(right_edge + 25.0, mid_y - 10.0));
                path = path.line_to(Point::new(right_edge + 25.0, mid_y + 10.0));
                path = path.line_to(Point::new(right_edge, mid_y + 10.0));
                return (path, Point::new(right_edge + 30.0, mid_y));
            }

            // Check if this edge skips layers vertically (> 1 layer gap)
            let is_layer_skip = (tgt_y - src_y) > 130.0;

            path = path.move_to(Point::new(src_x, src_y));

            if !is_layer_skip && (src_x - tgt_x).abs() < 5.0 && tgt_y > src_y {
                // Collinear straight vertical line down
                path = path.line_to(Point::new(src_x, tgt_y));
                let mid_y = (src_y + tgt_y) / 2.0;
                (path, Point::new(src_x, mid_y))
            } else if !is_layer_skip && tgt_y > src_y {
                // Standard forward step-down (two 90° elbows)
                let mid_y = (src_y + tgt_y) / 2.0;
                path = path.line_to(Point::new(src_x, mid_y));
                path = path.line_to(Point::new(tgt_x, mid_y));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new((src_x + tgt_x) / 2.0, mid_y))
            } else if is_layer_skip && tgt_y > src_y {
                // Vertical layer skip bypass around right side
                let right_bypass = (from.rect.origin.x + from.rect.size.width)
                    .max(to.rect.origin.x + to.rect.size.width)
                    + 35.0;
                path = path.line_to(Point::new(src_x, src_y + 15.0));
                path = path.line_to(Point::new(right_bypass, src_y + 15.0));
                path = path.line_to(Point::new(right_bypass, tgt_y - 15.0));
                path = path.line_to(Point::new(tgt_x, tgt_y - 15.0));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new(right_bypass, (src_y + tgt_y) / 2.0))
            } else {
                // Loop back / upward edge: route out to side
                let right_bypass = (from.rect.origin.x + from.rect.size.width)
                    .max(to.rect.origin.x + to.rect.size.width)
                    + 35.0;
                path = path.line_to(Point::new(src_x, src_y + 15.0));
                path = path.line_to(Point::new(right_bypass, src_y + 15.0));
                path = path.line_to(Point::new(right_bypass, tgt_y - 15.0));
                path = path.line_to(Point::new(tgt_x, tgt_y - 15.0));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new(right_bypass, (src_y + tgt_y) / 2.0))
            }
        }
        Direction::LeftToRight => {
            // Source exit point: right center
            let src_x = from.rect.origin.x + from.rect.size.width;
            let src_y = from.rect.origin.y + from.rect.size.height / 2.0;

            // Target entry point: left center
            let tgt_x = to.rect.origin.x;
            let tgt_y = to.rect.origin.y + to.rect.size.height / 2.0;

            // Self-loop edge in LR: route loop below node
            if from.id == to.id {
                let bot_edge = from.rect.origin.y + from.rect.size.height;
                let mid_x = from.rect.origin.x + from.rect.size.width / 2.0;
                path = path.move_to(Point::new(mid_x - 10.0, bot_edge));
                path = path.line_to(Point::new(mid_x - 10.0, bot_edge + 25.0));
                path = path.line_to(Point::new(mid_x + 10.0, bot_edge + 25.0));
                path = path.line_to(Point::new(mid_x + 10.0, bot_edge));
                return (path, Point::new(mid_x, bot_edge + 30.0));
            }

            // Check if this edge skips layers (span > 1 layer / intermediate node obstacle)
            let is_layer_skip = (tgt_x - src_x) > 160.0;

            path = path.move_to(Point::new(src_x, src_y));

            if !is_layer_skip && (src_y - tgt_y).abs() < 5.0 && tgt_x > src_x {
                // Collinear straight line without obstacles
                path = path.line_to(Point::new(tgt_x, src_y));
                let mid_x = (src_x + tgt_x) / 2.0;
                (path, Point::new(mid_x, src_y))
            } else if !is_layer_skip && tgt_x > src_x {
                // Normal adjacent layer step
                let mid_x = (src_x + tgt_x) / 2.0;
                path = path.line_to(Point::new(mid_x, src_y));
                path = path.line_to(Point::new(mid_x, tgt_y));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new(mid_x, (src_y + tgt_y) / 2.0))
            } else if is_layer_skip && tgt_x > src_x {
                // Layer skip / bypass forward: Route around below intermediate nodes!
                let bypass_y = from.rect.origin.y.max(to.rect.origin.y)
                    + from.rect.size.height.max(to.rect.size.height)
                    + 35.0;
                path = path.line_to(Point::new(src_x + 20.0, src_y));
                path = path.line_to(Point::new(src_x + 20.0, bypass_y));
                path = path.line_to(Point::new(tgt_x - 20.0, bypass_y));
                path = path.line_to(Point::new(tgt_x - 20.0, tgt_y));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new((src_x + tgt_x) / 2.0, bypass_y))
            } else {
                let bottom_bypass = (from.rect.origin.y + from.rect.size.height)
                    .max(to.rect.origin.y + to.rect.size.height)
                    + 30.0;
                path = path.line_to(Point::new(src_x + 15.0, src_y));
                path = path.line_to(Point::new(src_x + 15.0, bottom_bypass));
                path = path.line_to(Point::new(tgt_x - 15.0, bottom_bypass));
                path = path.line_to(Point::new(tgt_x - 15.0, tgt_y));
                path = path.line_to(Point::new(tgt_x, tgt_y));
                (path, Point::new((src_x + tgt_x) / 2.0, bottom_bypass))
            }
        }
    }
}
