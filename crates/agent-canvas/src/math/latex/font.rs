//! Access to the bundled math font: glyph lookup, metrics, MATH constants and
//! outlines. All lengths returned are in em (font units divided by units-per-em).

use std::sync::OnceLock;

use ttf_parser::math::{Constants, MathValue};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};

use super::error::MathError;
use super::render::fmt_num;

static FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/AgentCanvasMath-subset.otf");

/// Snapshot of the MATH constants used by the layout, lengths in em.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MathConsts {
    pub units_per_em: f64,
    /// Percent (e.g. 75).
    pub script_percent_scale_down: f64,
    /// Percent (e.g. 60).
    pub script_script_percent_scale_down: f64,
    pub display_operator_min_height: f64,
    pub axis_height: f64,
    pub subscript_shift_down: f64,
    pub subscript_top_max: f64,
    pub subscript_baseline_drop_min: f64,
    pub superscript_shift_up: f64,
    pub superscript_shift_up_cramped: f64,
    pub superscript_bottom_min: f64,
    pub superscript_baseline_drop_max: f64,
    pub sub_superscript_gap_min: f64,
    pub superscript_bottom_max_with_subscript: f64,
    pub space_after_script: f64,
    pub upper_limit_gap_min: f64,
    pub upper_limit_baseline_rise_min: f64,
    pub lower_limit_gap_min: f64,
    pub lower_limit_baseline_drop_min: f64,
    pub fraction_numerator_shift_up: f64,
    pub fraction_numerator_display_shift_up: f64,
    pub fraction_denominator_shift_down: f64,
    pub fraction_denominator_display_shift_down: f64,
    pub fraction_numerator_gap_min: f64,
    pub fraction_numerator_display_gap_min: f64,
    pub fraction_rule_thickness: f64,
    pub fraction_denominator_gap_min: f64,
    pub fraction_denominator_display_gap_min: f64,
    pub radical_vertical_gap: f64,
    pub radical_display_vertical_gap: f64,
    pub radical_rule_thickness: f64,
    pub radical_extra_ascender: f64,
    pub radical_kern_before_degree: f64,
    pub radical_kern_after_degree: f64,
    /// Percent.
    pub radical_degree_bottom_raise_percent: f64,
}

/// Vertical ink extent in em, y up.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphBox {
    pub y_min: f64,
    pub y_max: f64,
}

/// The bundled font.
pub(crate) struct MathFont {
    face: Face<'static>,
    consts: MathConsts,
}

fn font_err(reason: &str) -> MathError {
    MathError::Font {
        reason: reason.into(),
    }
}

static SHARED: OnceLock<Result<MathFont, MathError>> = OnceLock::new();

impl MathFont {
    /// Process-wide parsed font (parsed once; immutable afterwards).
    pub(crate) fn shared() -> Result<&'static MathFont, MathError> {
        SHARED
            .get_or_init(MathFont::load)
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Parses the embedded font.
    pub(crate) fn load() -> Result<Self, MathError> {
        let face = Face::parse(FONT_BYTES, 0).map_err(|_| font_err("embedded font is invalid"))?;
        let upem = f64::from(face.units_per_em());
        if upem <= 0.0 {
            return Err(font_err("units per em is zero"));
        }
        let constants = face
            .tables()
            .math
            .and_then(|m| m.constants)
            .ok_or_else(|| font_err("MATH constants missing"))?;
        let consts = read_consts(&constants, upem);
        Ok(Self { face, consts })
    }

    pub(crate) fn consts(&self) -> &MathConsts {
        &self.consts
    }

    fn upem(&self) -> f64 {
        self.consts.units_per_em
    }

    /// Glyph for `c`, or `MissingGlyph`.
    pub(crate) fn glyph(&self, c: char) -> Result<GlyphId, MathError> {
        self.face
            .glyph_index(c)
            .ok_or(MathError::MissingGlyph { ch: c })
    }

    /// Horizontal advance in em.
    pub(crate) fn advance(&self, gid: GlyphId) -> f64 {
        self.face
            .glyph_hor_advance(gid)
            .map_or(0.0, |a| f64::from(a) / self.upem())
    }

    /// Italic correction in em (0 when absent).
    pub(crate) fn italic_correction(&self, gid: GlyphId) -> f64 {
        self.face
            .tables()
            .math
            .and_then(|m| m.glyph_info)
            .and_then(|g| g.italic_corrections)
            .and_then(|ic| ic.get(gid))
            .map_or(0.0, |v| f64::from(v.value) / self.upem())
    }

    /// Vertical size variants `(glyph, extent in em)`, ascending by extent.
    /// The base glyph is included when the font does not list it.
    pub(crate) fn vertical_variants(&self, gid: GlyphId) -> Vec<(GlyphId, f64)> {
        let mut out: Vec<(GlyphId, f64)> = self
            .face
            .tables()
            .math
            .and_then(|m| m.variants)
            .and_then(|v| v.vertical_constructions.get(gid))
            .map(|c| {
                c.variants
                    .into_iter()
                    .map(|v| {
                        (
                            v.variant_glyph,
                            f64::from(v.advance_measurement) / self.upem(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !out.iter().any(|(g, _)| *g == gid) {
            let extent = self.bbox(gid).map_or(0.0, |b| b.y_max - b.y_min);
            out.push((gid, extent));
        }
        out.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0 .0.cmp(&b.0 .0)));
        out
    }

    /// Display-style variant of a big operator: the smallest variant at least
    /// `display_operator_min_height` tall, else the largest.
    pub(crate) fn display_variant(&self, gid: GlyphId) -> GlyphId {
        let variants = self.vertical_variants(gid);
        let min = self.consts.display_operator_min_height;
        variants
            .iter()
            .find(|(_, e)| *e >= min)
            .or_else(|| variants.last())
            .map_or(gid, |(g, _)| *g)
    }

    /// Ink bounding box in em.
    pub(crate) fn bbox(&self, gid: GlyphId) -> Option<GlyphBox> {
        let Rect { y_min, y_max, .. } = self.face.glyph_bounding_box(gid)?;
        let u = self.upem();
        Some(GlyphBox {
            y_min: f64::from(y_min) / u,
            y_max: f64::from(y_max) / u,
        })
    }

    /// SVG path data for the glyph in font units, y flipped (y down).
    pub(crate) fn outline_path(&self, gid: GlyphId) -> Result<String, MathError> {
        let mut b = PathBuilder {
            d: String::new(),
            err: None,
        };
        self.face.outline_glyph(gid, &mut b);
        if let Some(e) = b.err {
            return Err(e);
        }
        Ok(b.d)
    }
}

struct PathBuilder {
    d: String,
    err: Option<MathError>,
}

impl PathBuilder {
    fn push(&mut self, cmd: char, coords: &[f32]) {
        if self.err.is_some() {
            return;
        }
        self.d.push(cmd);
        for (i, v) in coords.chunks(2).enumerate() {
            let (Some(&x), Some(&y)) = (v.first(), v.get(1)) else {
                continue;
            };
            let (x, y) = (f64::from(x), -f64::from(y));
            match (fmt_num(x), fmt_num(y)) {
                (Ok(xs), Ok(ys)) => {
                    if i > 0 {
                        self.d.push(' ');
                    }
                    self.d.push_str(&xs);
                    self.d.push(' ');
                    self.d.push_str(&ys);
                }
                (Err(e), _) | (_, Err(e)) => {
                    self.err = Some(e);
                    return;
                }
            }
        }
    }
}

impl OutlineBuilder for PathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push('M', &[x, y]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push('L', &[x, y]);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.push('Q', &[x1, y1, x, y]);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.push('C', &[x1, y1, x2, y2, x, y]);
    }
    fn close(&mut self) {
        if self.err.is_none() {
            self.d.push('Z');
        }
    }
}

fn read_consts(c: &Constants<'_>, upem: f64) -> MathConsts {
    let m = |v: MathValue<'_>| f64::from(v.value) / upem;
    MathConsts {
        units_per_em: upem,
        script_percent_scale_down: f64::from(c.script_percent_scale_down()),
        script_script_percent_scale_down: f64::from(c.script_script_percent_scale_down()),
        display_operator_min_height: f64::from(c.display_operator_min_height()) / upem,
        axis_height: m(c.axis_height()),
        subscript_shift_down: m(c.subscript_shift_down()),
        subscript_top_max: m(c.subscript_top_max()),
        subscript_baseline_drop_min: m(c.subscript_baseline_drop_min()),
        superscript_shift_up: m(c.superscript_shift_up()),
        superscript_shift_up_cramped: m(c.superscript_shift_up_cramped()),
        superscript_bottom_min: m(c.superscript_bottom_min()),
        superscript_baseline_drop_max: m(c.superscript_baseline_drop_max()),
        sub_superscript_gap_min: m(c.sub_superscript_gap_min()),
        superscript_bottom_max_with_subscript: m(c.superscript_bottom_max_with_subscript()),
        space_after_script: m(c.space_after_script()),
        upper_limit_gap_min: m(c.upper_limit_gap_min()),
        upper_limit_baseline_rise_min: m(c.upper_limit_baseline_rise_min()),
        lower_limit_gap_min: m(c.lower_limit_gap_min()),
        lower_limit_baseline_drop_min: m(c.lower_limit_baseline_drop_min()),
        fraction_numerator_shift_up: m(c.fraction_numerator_shift_up()),
        fraction_numerator_display_shift_up: m(c.fraction_numerator_display_style_shift_up()),
        fraction_denominator_shift_down: m(c.fraction_denominator_shift_down()),
        fraction_denominator_display_shift_down: m(
            c.fraction_denominator_display_style_shift_down()
        ),
        fraction_numerator_gap_min: m(c.fraction_numerator_gap_min()),
        fraction_numerator_display_gap_min: m(c.fraction_num_display_style_gap_min()),
        fraction_rule_thickness: m(c.fraction_rule_thickness()),
        fraction_denominator_gap_min: m(c.fraction_denominator_gap_min()),
        fraction_denominator_display_gap_min: m(c.fraction_denom_display_style_gap_min()),
        radical_vertical_gap: m(c.radical_vertical_gap()),
        radical_display_vertical_gap: m(c.radical_display_style_vertical_gap()),
        radical_rule_thickness: m(c.radical_rule_thickness()),
        radical_extra_ascender: m(c.radical_extra_ascender()),
        radical_kern_before_degree: m(c.radical_kern_before_degree()),
        radical_kern_after_degree: m(c.radical_kern_after_degree()),
        radical_degree_bottom_raise_percent: f64::from(c.radical_degree_bottom_raise_percent()),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::math::latex::atoms::math_char;

    #[test]
    fn loads_and_has_consts() {
        let f = MathFont::load().unwrap();
        let c = f.consts();
        assert!(c.units_per_em > 0.0);
        assert!(c.axis_height > 0.0);
        assert!((60.0..=85.0).contains(&c.script_percent_scale_down));
        assert!(c.script_script_percent_scale_down > 30.0);
        assert!(c.script_script_percent_scale_down < c.script_percent_scale_down);
        assert!(c.fraction_rule_thickness > 0.0);
        assert!(c.radical_rule_thickness > 0.0);
        assert!(c.radical_vertical_gap > 0.0);
        assert!(c.display_operator_min_height > 1.0);
    }

    #[test]
    fn glyph_lookup() {
        let f = MathFont::load().unwrap();
        assert_eq!(
            f.glyph(math_char('a')).unwrap(),
            f.glyph('\u{1D44E}').unwrap()
        );
        assert_eq!(
            f.glyph(math_char('h')).unwrap(),
            f.glyph('\u{210E}').unwrap()
        );
        assert_eq!(f.glyph('€'), Err(MathError::MissingGlyph { ch: '€' }));
    }

    #[test]
    fn metrics_plausible() {
        let f = MathFont::load().unwrap();
        let a = f.glyph(math_char('a')).unwrap();
        assert!(f.advance(a) > 0.2 && f.advance(a) < 1.0);
        let x = f.glyph(math_char('f')).unwrap();
        assert!(f.italic_correction(x) >= 0.0);
        let root = f.glyph('√').unwrap();
        let v = f.vertical_variants(root);
        assert!(v.len() >= 2);
        assert!(v.windows(2).all(|w| w[0].1 <= w[1].1));
        let sum = f.glyph('∑').unwrap();
        let d = f.display_variant(sum);
        let b = f.bbox(d).unwrap();
        assert!(b.y_max - b.y_min > 1.0);
        let b0 = f.bbox(sum).unwrap();
        assert!(b.y_max - b.y_min >= b0.y_max - b0.y_min);
    }

    #[test]
    fn outline_is_finite_path() {
        let f = MathFont::load().unwrap();
        let p = f.outline_path(f.glyph('∑').unwrap()).unwrap();
        assert!(p.starts_with('M') && p.ends_with('Z'));
        assert!(!p.contains("NaN") && !p.contains("inf") && !p.contains('e'));
    }
}
