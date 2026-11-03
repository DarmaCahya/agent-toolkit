#![forbid(unsafe_code)]

//! Dependency-free SVG document writer.
//!
//! Draw calls with non-finite geometry are skipped so the output is always
//! well-formed. `add_def` accepts raw, trusted XML; everything else is escaped.

use super::geom::{fmt_num, Fill, Path, Point, Rect, Stroke};
use super::theme::Color;

const FONT_FAMILY: &str = "'DejaVu Sans', sans-serif";

/// Escapes the five XML special characters and strips XML-illegal control characters.
pub fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if c < ' ' || ('\u{7F}'..='\u{84}').contains(&c) || ('\u{86}'..='\u{9F}').contains(&c) => {
                // Drop XML-illegal control characters
            }
            '\u{FFFE}' | '\u{FFFF}' => {
                // Drop Unicode non-characters
            }
            _ => out.push(c),
        }
    }
    out
}

fn stroke_attrs(stroke: Option<&Stroke>) -> String {
    let Some(s) = stroke else { return String::new() };
    let mut out = format!(
        " stroke=\"{}\" stroke-width=\"{}\"",
        s.color.to_rgb_hex(),
        fmt_num(s.width)
    );
    if s.color.a < 1.0 {
        out.push_str(&format!(" stroke-opacity=\"{}\"", fmt_num(s.color.a)));
    }
    if let Some(dash) = &s.dash_array {
        out.push_str(&format!(" stroke-dasharray=\"{}\"", escape_xml(dash)));
    }
    out
}

fn fill_attrs(fill: Option<&Fill>) -> String {
    let Some(f) = fill else {
        return " fill=\"none\"".to_string();
    };
    let mut out = format!(" fill=\"{}\"", f.color.to_rgb_hex());
    let effective_opacity = (f.color.a * f.opacity).clamp(0.0, 1.0);
    if effective_opacity < 1.0 {
        out.push_str(&format!(" fill-opacity=\"{}\"", fmt_num(effective_opacity)));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SvgCanvas {
    pub width: f32,
    pub height: f32,
    pub view_box: Option<(f32, f32, f32, f32)>,
    pub background: Option<Color>,
    pub defs: Vec<String>,
    pub elements: Vec<String>,
}

impl SvgCanvas {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, ..Self::default() }
    }

    pub fn with_view_box(mut self, min_x: f32, min_y: f32, width: f32, height: f32) -> Self {
        self.view_box = Some((min_x, min_y, width, height));
        self
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Adds a raw XML snippet to `<defs>`. The caller is responsible for its validity.
    pub fn add_def(&mut self, xml_snippet: impl Into<String>) {
        self.defs.push(xml_snippet.into());
    }

    pub fn draw_rect(&mut self, rect: Rect, stroke: Option<Stroke>, fill: Option<Fill>) {
        if !rect.is_finite() || rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let mut el = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            fmt_num(rect.origin.x),
            fmt_num(rect.origin.y),
            fmt_num(rect.size.width),
            fmt_num(rect.size.height)
        );
        if rect.rx > 0.0 {
            el.push_str(&format!(" rx=\"{}\"", fmt_num(rect.rx)));
        }
        if rect.ry > 0.0 {
            el.push_str(&format!(" ry=\"{}\"", fmt_num(rect.ry)));
        }
        el.push_str(&fill_attrs(fill.as_ref()));
        el.push_str(&stroke_attrs(stroke.as_ref()));
        el.push_str("/>");
        self.elements.push(el);
    }

    pub fn draw_circle(
        &mut self,
        center: Point,
        radius: f32,
        stroke: Option<Stroke>,
        fill: Option<Fill>,
    ) {
        if !center.is_finite() || !radius.is_finite() || radius < 0.0 {
            return;
        }
        let mut el = format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"",
            fmt_num(center.x),
            fmt_num(center.y),
            fmt_num(radius)
        );
        el.push_str(&fill_attrs(fill.as_ref()));
        el.push_str(&stroke_attrs(stroke.as_ref()));
        el.push_str("/>");
        self.elements.push(el);
    }

    pub fn draw_line(&mut self, from: Point, to: Point, stroke: Stroke) {
        if !from.is_finite() || !to.is_finite() {
            return;
        }
        let mut el = format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
            fmt_num(from.x),
            fmt_num(from.y),
            fmt_num(to.x),
            fmt_num(to.y)
        );
        el.push_str(&stroke_attrs(Some(&stroke)));
        el.push_str("/>");
        self.elements.push(el);
    }

    pub fn draw_path(&mut self, path: &Path, stroke: Option<Stroke>, fill: Option<Fill>) {
        if path.is_empty() || !path.is_finite() {
            return;
        }
        let mut el = format!("<path d=\"{}\"", path.to_svg_d());
        el.push_str(&fill_attrs(fill.as_ref()));
        el.push_str(&stroke_attrs(stroke.as_ref()));
        el.push_str("/>");
        self.elements.push(el);
    }

    /// `text_anchor` must be `start`, `middle` or `end`; anything else falls back to `start`.
    pub fn draw_text(
        &mut self,
        pos: Point,
        text: &str,
        font_size: f32,
        color: Color,
        text_anchor: &str,
    ) {
        if !pos.is_finite() || !font_size.is_finite() || font_size <= 0.0 {
            return;
        }
        let anchor = match text_anchor {
            "middle" | "end" => text_anchor,
            _ => "start",
        };
        let mut el = format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" text-anchor=\"{}\"{}",
            fmt_num(pos.x),
            fmt_num(pos.y),
            FONT_FAMILY,
            fmt_num(font_size),
            anchor,
            fill_attrs(Some(&Fill::solid(color)))
        );
        el.push('>');
        el.push_str(&escape_xml(text));
        el.push_str("</text>");
        self.elements.push(el);
    }

    /// Draws rotated text (e.g. for vertical Y-axis labels, slanting X-axis categories).
    pub fn draw_text_rotated(
        &mut self,
        pos: Point,
        text: &str,
        font_size: f32,
        color: Color,
        text_anchor: &str,
        angle_deg: f32,
    ) {
        if !pos.is_finite() || !font_size.is_finite() || font_size <= 0.0 || !angle_deg.is_finite() {
            return;
        }
        let anchor = match text_anchor {
            "middle" | "end" => text_anchor,
            _ => "start",
        };
        let mut el = format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" text-anchor=\"{}\" transform=\"rotate({}, {}, {})\"{}",
            fmt_num(pos.x),
            fmt_num(pos.y),
            FONT_FAMILY,
            fmt_num(font_size),
            anchor,
            fmt_num(angle_deg),
            fmt_num(pos.x),
            fmt_num(pos.y),
            fill_attrs(Some(&Fill::solid(color)))
        );
        el.push('>');
        el.push_str(&escape_xml(text));
        el.push_str("</text>");
        self.elements.push(el);
    }

    pub fn to_svg(&self) -> String {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"",
            fmt_num(self.width),
            fmt_num(self.height)
        );
        if let Some((x, y, w, h)) = self.view_box {
            out.push_str(&format!(
                " viewBox=\"{} {} {} {}\"",
                fmt_num(x),
                fmt_num(y),
                fmt_num(w),
                fmt_num(h)
            ));
        }
        out.push_str(">\n");

        if !self.defs.is_empty() {
            out.push_str("<defs>\n");
            for def in &self.defs {
                out.push_str(def);
                out.push('\n');
            }
            out.push_str("</defs>\n");
        }

        if let Some(bg) = self.background {
            let (x, y, w, h) = self.view_box.unwrap_or((0.0, 0.0, self.width, self.height));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>\n",
                fmt_num(x),
                fmt_num(y),
                fmt_num(w),
                fmt_num(h),
                fill_attrs(Some(&Fill::solid(bg)))
            ));
        }

        for el in &self.elements {
            out.push_str(el);
            out.push('\n');
        }
        out.push_str("</svg>\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::geom::Size;

    #[test]
    fn escapes() {
        assert_eq!(escape_xml("<a href=\"x\">&'</a>"), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;");
        assert_eq!(escape_xml("Hello\x00World\x07!"), "HelloWorld!");
    }

    #[test]
    fn renders_document() {
        let red = Color::from_rgb(255, 0, 0);
        let mut c = SvgCanvas::new(100.0, 50.0)
            .with_view_box(0.0, 0.0, 200.0, 100.0)
            .with_background(Color::from_rgb(255, 255, 255));
        c.add_def("<clipPath id=\"c\"/>");
        c.draw_rect(
            Rect::new(Point::new(1.0, 2.0), Size::new(3.0, 4.0)).with_radius(1.0, 1.0),
            Some(Stroke::new(red, 2.0).with_dash("4 2")),
            Some(Fill::new(red, 0.5)),
        );
        c.draw_text(Point::new(5.0, 5.0), "a<b", 12.0, red, "bogus");
        c.draw_circle(Point::new(f32::NAN, 0.0), 1.0, None, None);
        let svg = c.to_svg();
        assert!(svg.contains("viewBox=\"0 0 200 100\""));
        assert!(svg.contains("<defs>"));
        assert!(svg.contains("fill-opacity=\"0.5\""));
        assert!(svg.contains("stroke-dasharray=\"4 2\""));
        assert!(svg.contains("text-anchor=\"start\""));
        assert!(svg.contains(">a&lt;b</text>"));
        assert!(!svg.contains("<circle"));
        assert!(svg.ends_with("</svg>\n"));
    }
}
