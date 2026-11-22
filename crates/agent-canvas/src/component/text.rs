#![forbid(unsafe_code)]

//! Vector text rendering from a bundled DejaVu Sans subset.
//!
//! Component cards must not emit `<text>` or depend on fonts installed on the
//! viewer machine. Glyph outlines are read with `ttf-parser` and emitted as
//! SVG paths, so output is self-contained and deterministic.
//!
//! The bundled faces are subsets of *DejaVu Sans* (Bitstream Vera licence); see
//! `THIRD_PARTY_NOTICES.md`.

use crate::canvas::{Color, Fill, Path, PathCommand, Point, SvgCanvas};
use std::sync::OnceLock;
use ttf_parser::{Face, OutlineBuilder};

const REGULAR_BYTES: &[u8] = include_bytes!("../../assets/fonts/AgentCanvasSans-subset.ttf");
const BOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/AgentCanvasSansBold-subset.ttf");

static REGULAR: OnceLock<Option<Face<'static>>> = OnceLock::new();
static BOLD: OnceLock<Option<Face<'static>>> = OnceLock::new();

/// Font weight for [`draw`] and [`measure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Weight {
    Regular,
    Bold,
}

fn face(weight: Weight) -> Option<&'static Face<'static>> {
    let cell = match weight {
        Weight::Regular => REGULAR.get_or_init(|| Face::parse(REGULAR_BYTES, 0).ok()),
        Weight::Bold => BOLD.get_or_init(|| Face::parse(BOLD_BYTES, 0).ok()),
    };
    cell.as_ref()
}

fn advance(face: &Face<'_>, character: char) -> f32 {
    let glyph = face
        .glyph_index(character)
        .or_else(|| face.glyph_index('?'));
    glyph
        .and_then(|id| face.glyph_hor_advance(id))
        .map_or(0.0, f32::from)
}

/// Width of `text` at `size` pixels.
///
/// Missing glyphs fall back to `?`; unknown characters still advance by zero
/// rather than panicking.
pub(crate) fn measure(text: &str, size: f32, weight: Weight) -> f32 {
    if !size.is_finite() || size <= 0.0 {
        return 0.0;
    }
    let Some(face) = face(weight) else {
        return 0.0;
    };
    let units = f32::from(face.units_per_em());
    if units <= 0.0 {
        return 0.0;
    }
    let scale = size / units;
    text.chars().map(|c| advance(face, c) * scale).sum()
}

/// Draws `text` with its baseline at `baseline`.
///
/// Returns the advance width of the run. Empty text or unavailable font data
/// is a no-op returning `0.0`.
pub(crate) fn draw(
    canvas: &mut SvgCanvas,
    text: &str,
    x: f32,
    baseline: f32,
    size: f32,
    weight: Weight,
    color: Color,
) -> f32 {
    if text.is_empty()
        || !size.is_finite()
        || size <= 0.0
        || !x.is_finite()
        || !baseline.is_finite()
    {
        return 0.0;
    }
    let Some(face) = face(weight) else {
        return 0.0;
    };
    let units = f32::from(face.units_per_em());
    if units <= 0.0 {
        return 0.0;
    }
    let scale = size / units;
    let width = measure(text, size, weight);
    let mut pen = x;

    for character in text.chars() {
        let glyph = face
            .glyph_index(character)
            .or_else(|| face.glyph_index('?'));
        let Some(glyph) = glyph else {
            continue;
        };
        let mut builder = OutlinePath {
            path: Path::new(),
            pen,
            baseline,
            scale,
        };
        face.outline_glyph(glyph, &mut builder);
        if !builder.path.is_empty() {
            canvas.draw_path(&builder.path, None, Some(Fill::solid(color)));
        }
        pen += advance(face, character) * scale;
    }
    width
}

/// Adapts `ttf-parser` font-unit outlines into a translated, Y-flipped path.
struct OutlinePath {
    path: Path,
    pen: f32,
    baseline: f32,
    scale: f32,
}

impl OutlinePath {
    fn point(&self, x: f32, y: f32) -> Point {
        Point::new(self.pen + x * self.scale, self.baseline - y * self.scale)
    }
}

impl OutlineBuilder for OutlinePath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path
            .push_command(PathCommand::MoveTo(self.point(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path
            .push_command(PathCommand::LineTo(self.point(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path
            .push_command(PathCommand::QuadTo(self.point(x1, y1), self.point(x, y)));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.push_command(PathCommand::CubicTo(
            self.point(x1, y1),
            self.point(x2, y2),
            self.point(x, y),
        ));
    }

    fn close(&mut self) {
        self.path.push_command(PathCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_faces_parse_and_measure() {
        assert!(face(Weight::Regular).is_some());
        assert!(face(Weight::Bold).is_some());
        let width = measure("Revenue", 24.0, Weight::Regular);
        assert!(width > 60.0 && width < 120.0, "width was {width}");
        assert_eq!(measure("Revenue", 0.0, Weight::Regular), 0.0);
        assert_eq!(measure("Revenue", f32::NAN, Weight::Regular), 0.0);
    }

    #[test]
    fn text_is_emitted_as_paths_only() {
        let mut canvas = SvgCanvas::new(200.0, 60.0);
        let width = draw(
            &mut canvas,
            "KPI 42",
            10.0,
            40.0,
            20.0,
            Weight::Bold,
            Color::from_rgb(0, 0, 0),
        );
        assert!(width > 0.0);
        let svg = canvas.to_svg();
        assert!(svg.contains("<path"));
        assert!(!svg.contains("<text"));
        assert!(!svg.contains("font-family"));
    }

    #[test]
    fn drawn_width_matches_measurement() {
        let width = measure("TOTAL", 16.0, Weight::Regular);
        let mut canvas = SvgCanvas::new(200.0, 60.0);
        let drawn = draw(
            &mut canvas,
            "TOTAL",
            10.0,
            40.0,
            16.0,
            Weight::Regular,
            Color::from_rgb(0, 0, 0),
        );
        assert!((drawn - width).abs() < 1e-5);
        assert_eq!(canvas.elements.len(), 5);
    }
}
