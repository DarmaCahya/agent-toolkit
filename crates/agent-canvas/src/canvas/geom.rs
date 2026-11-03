#![forbid(unsafe_code)]

//! Core geometric primitives: points, sizes, rectangles, strokes, fills and paths.

use super::theme::Color;
use std::fmt::Write as _;

/// Formats a float compactly for SVG output (max 3 decimals, no trailing zeros).
/// Non-finite values collapse to `0` so output is always valid SVG.
pub(crate) fn fmt_num(v: f32) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let mut s = format!("{:.3}", v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: Point) -> f32 {
        (other.x - self.x).hypot(other.y - self.y)
    }

    /// Linear interpolation; `t` is not clamped, so extrapolation is allowed.
    pub fn lerp(&self, other: Point, t: f32) -> Point {
        Point::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }

    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_finite(&self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
    pub rx: f32,
    pub ry: f32,
}

impl Rect {
    pub const fn new(origin: Point, size: Size) -> Self {
        Self { origin, size, rx: 0.0, ry: 0.0 }
    }

    /// Sets corner radii; negative or NaN values are treated as zero.
    pub fn with_radius(mut self, rx: f32, ry: f32) -> Self {
        self.rx = if rx > 0.0 { rx } else { 0.0 };
        self.ry = if ry > 0.0 { ry } else { 0.0 };
        self
    }

    pub fn is_finite(&self) -> bool {
        self.origin.is_finite() && self.size.is_finite() && self.rx.is_finite() && self.ry.is_finite()
    }

    /// Edge-inclusive containment test.
    pub fn contains_point(&self, p: Point) -> bool {
        let br = self.bottom_right();
        p.x >= self.origin.x && p.x <= br.x && p.y >= self.origin.y && p.y <= br.y
    }

    /// Returns true when the interiors overlap (rectangles that merely touch do not intersect).
    pub fn intersects(&self, other: &Rect) -> bool {
        let (a, b) = (self.bottom_right(), other.bottom_right());
        self.origin.x < b.x && other.origin.x < a.x && self.origin.y < b.y && other.origin.y < a.y
    }

    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width / 2.0,
            self.origin.y + self.size.height / 2.0,
        )
    }

    pub fn top_left(&self) -> Point {
        self.origin
    }

    pub fn bottom_right(&self) -> Point {
        Point::new(self.origin.x + self.size.width, self.origin.y + self.size.height)
    }

    /// Grows the rect by `amount` on every side. Negative values shrink it;
    /// the size never drops below zero. Returns `*self` if inputs are non-finite.
    pub fn expand(&self, amount: f32) -> Rect {
        if !self.is_finite() || !amount.is_finite() {
            return *self;
        }
        let width = (self.size.width + 2.0 * amount).max(0.0);
        let height = (self.size.height + 2.0 * amount).max(0.0);
        // Re-center when clamped so shrinking past zero collapses to the center.
        let c = self.center();
        Rect {
            origin: Point::new(c.x - width / 2.0, c.y - height / 2.0),
            size: Size::new(width, height),
            rx: self.rx,
            ry: self.ry,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
    pub dash_array: Option<String>,
}

impl Stroke {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width, dash_array: None }
    }

    pub fn with_dash(mut self, dash_array: impl Into<String>) -> Self {
        self.dash_array = Some(dash_array.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    pub color: Color,
    pub opacity: f32,
}

impl Fill {
    /// `opacity` is clamped to `0.0..=1.0` (NaN becomes `1.0`).
    pub fn new(color: Color, opacity: f32) -> Self {
        let opacity = if opacity.is_nan() { 1.0 } else { opacity.clamp(0.0, 1.0) };
        Self { color, opacity }
    }

    pub fn solid(color: Color) -> Self {
        Self::new(color, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CubicTo(Point, Point, Point),
    ArcTo {
        rx: f32,
        ry: f32,
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        end: Point,
    },
    Close,
}

impl PathCommand {
    fn is_finite(&self) -> bool {
        match *self {
            PathCommand::MoveTo(p) | PathCommand::LineTo(p) => p.is_finite(),
            PathCommand::QuadTo(a, b) => a.is_finite() && b.is_finite(),
            PathCommand::CubicTo(a, b, c) => a.is_finite() && b.is_finite() && c.is_finite(),
            PathCommand::ArcTo { rx, ry, x_axis_rotation, end, .. } => {
                rx.is_finite() && ry.is_finite() && x_axis_rotation.is_finite() && end.is_finite()
            }
            PathCommand::Close => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn is_finite(&self) -> bool {
        self.commands.iter().all(PathCommand::is_finite)
    }

    pub fn move_to(mut self, p: Point) -> Self {
        self.commands.push(PathCommand::MoveTo(p));
        self
    }

    pub fn line_to(mut self, p: Point) -> Self {
        self.commands.push(PathCommand::LineTo(p));
        self
    }

    pub fn quad_to(mut self, ctrl: Point, end: Point) -> Self {
        self.commands.push(PathCommand::QuadTo(ctrl, end));
        self
    }

    pub fn cubic_to(mut self, c1: Point, c2: Point, end: Point) -> Self {
        self.commands.push(PathCommand::CubicTo(c1, c2, end));
        self
    }

    pub fn arc_to(
        mut self,
        rx: f32,
        ry: f32,
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        end: Point,
    ) -> Self {
        self.commands.push(PathCommand::ArcTo { rx, ry, x_axis_rotation, large_arc, sweep, end });
        self
    }

    pub fn close(mut self) -> Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// Serializes to an SVG path `d` attribute value.
    /// Prepends `M0 0` if the first command is not a MoveTo to ensure valid SVG path syntax.
    pub fn to_svg_d(&self) -> String {
        let mut out = String::new();
        let mut has_moved = false;
        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo(p) => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    has_moved = true;
                    let _ = write!(out, "M{} {}", fmt_num(p.x), fmt_num(p.y));
                }
                PathCommand::LineTo(p) => {
                    if !has_moved {
                        out.push_str("M0 0 ");
                        has_moved = true;
                    } else if !out.is_empty() {
                        out.push(' ');
                    }
                    let _ = write!(out, "L{} {}", fmt_num(p.x), fmt_num(p.y));
                }
                PathCommand::QuadTo(c, p) => {
                    if !has_moved {
                        out.push_str("M0 0 ");
                        has_moved = true;
                    } else if !out.is_empty() {
                        out.push(' ');
                    }
                    let _ = write!(
                        out,
                        "Q{} {} {} {}",
                        fmt_num(c.x),
                        fmt_num(c.y),
                        fmt_num(p.x),
                        fmt_num(p.y)
                    );
                }
                PathCommand::CubicTo(c1, c2, p) => {
                    if !has_moved {
                        out.push_str("M0 0 ");
                        has_moved = true;
                    } else if !out.is_empty() {
                        out.push(' ');
                    }
                    let _ = write!(
                        out,
                        "C{} {} {} {} {} {}",
                        fmt_num(c1.x),
                        fmt_num(c1.y),
                        fmt_num(c2.x),
                        fmt_num(c2.y),
                        fmt_num(p.x),
                        fmt_num(p.y)
                    );
                }
                PathCommand::ArcTo { rx, ry, x_axis_rotation, large_arc, sweep, end } => {
                    if !has_moved {
                        out.push_str("M0 0 ");
                        has_moved = true;
                    } else if !out.is_empty() {
                        out.push(' ');
                    }
                    let _ = write!(
                        out,
                        "A{} {} {} {} {} {} {}",
                        fmt_num(rx),
                        fmt_num(ry),
                        fmt_num(x_axis_rotation),
                        u8::from(large_arc),
                        u8::from(sweep),
                        fmt_num(end.x),
                        fmt_num(end.y)
                    );
                }
                PathCommand::Close => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push('Z');
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_math() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert_eq!(a.distance_to(b), 5.0);
        assert_eq!(a.lerp(b, 0.5), Point::new(1.5, 2.0));
        assert!(!Point::new(f32::NAN, 0.0).is_finite());
    }

    #[test]
    fn rect_ops() {
        let r = Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        assert!(r.contains_point(Point::new(10.0, 10.0)));
        assert!(!r.contains_point(Point::new(10.1, 5.0)));
        let touching = Rect::new(Point::new(10.0, 0.0), Size::new(5.0, 5.0));
        assert!(!r.intersects(&touching));
        let overlap = Rect::new(Point::new(9.0, 9.0), Size::new(5.0, 5.0));
        assert!(r.intersects(&overlap));
        assert_eq!(r.expand(2.0).size, Size::new(14.0, 14.0));
        assert_eq!(r.expand(-20.0).size, Size::new(0.0, 0.0));
    }

    #[test]
    fn path_d() {
        let d = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(10.5, -0.0))
            .arc_to(5.0, 5.0, 0.0, true, false, Point::new(1.0, 2.0))
            .close()
            .to_svg_d();
        assert_eq!(d, "M0 0 L10.5 0 A5 5 0 1 0 1 2 Z");
    }
}
