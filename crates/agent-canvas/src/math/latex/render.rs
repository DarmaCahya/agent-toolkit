//! SVG emission: deterministic, vector-only output.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::box_model::{Item, LBox};
use super::error::MathError;
use super::font::MathFont;

/// Output units per em.
const UNITS: f64 = 1000.0;
/// Padding around the formula, in output units.
const PAD: f64 = 20.0;

fn push(out: &mut String, s: &str) {
    out.push_str(s);
}

fn num(x: f64) -> Result<String, MathError> {
    fmt_num(x)
}

/// Renders a laid-out root box to a self-contained SVG string.
pub(crate) fn render(font: &MathFont, root: &LBox) -> Result<String, MathError> {
    root.finite()?;
    let upem = font.consts().units_per_em;
    let glyph_scale = UNITS / upem;

    let gids: BTreeSet<u16> = root
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Glyph { gid, .. } | Item::ScaledGlyph { gid, .. } => Some(gid.0),
            Item::Rule { .. } => None,
        })
        .collect();

    let min_x = -PAD;
    let min_y = -root.height * UNITS - PAD;
    let w = root.width * UNITS + 2.0 * PAD;
    let h = (root.height + root.depth) * UNITS + 2.0 * PAD;

    let mut out = String::new();
    push(
        &mut out,
        &format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" fill=\"currentColor\">\n",
            num(min_x)?,
            num(min_y)?,
            num(w)?,
            num(h)?
        ),
    );
    if !gids.is_empty() {
        push(&mut out, "<defs>\n");
        for gid in &gids {
            let d = font.outline_path(ttf_parser::GlyphId(*gid))?;
            let _ = writeln!(out, "<path id=\"g{gid}\" d=\"{d}\"/>");
        }
        push(&mut out, "</defs>\n");
    }
    for item in &root.items {
        match *item {
            Item::Glyph { gid, x, y, scale } => {
                let _ = writeln!(
                    out,
                    "<use href=\"#g{}\" transform=\"translate({} {}) scale({})\"/>",
                    gid.0,
                    num(x * UNITS)?,
                    num(-y * UNITS)?,
                    num(scale * glyph_scale)?
                );
            }
            Item::ScaledGlyph { gid, x, y, sx, sy } => {
                let _ = writeln!(
                    out,
                    "<use href=\"#g{}\" transform=\"translate({} {}) scale({} {})\"/>",
                    gid.0,
                    num(x * UNITS)?,
                    num(-y * UNITS)?,
                    num(sx * glyph_scale)?,
                    num(sy * glyph_scale)?
                );
            }
            Item::Rule { x, y, w, h } => {
                let _ = writeln!(
                    out,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    num(x * UNITS)?,
                    num(-(y + h) * UNITS)?,
                    num(w * UNITS)?,
                    num(h * UNITS)?
                );
            }
        }
    }
    push(&mut out, "</svg>\n");
    Ok(out)
}

/// Formats `x` with at most two decimals, no trailing zeros, never an exponent.
pub(crate) fn fmt_num(x: f64) -> Result<String, MathError> {
    if !x.is_finite() {
        return Err(MathError::Font {
            reason: "non-finite coordinate".into(),
        });
    }
    let scaled = (x * 100.0).round();
    if scaled.abs() >= 9.0e15 {
        return Err(MathError::Font {
            reason: "coordinate out of range".into(),
        });
    }
    #[allow(clippy::cast_possible_truncation)] // range checked above
    let n = scaled as i64;
    let sign = if n < 0 { "-" } else { "" };
    let abs = n.unsigned_abs();
    let (int, frac) = (abs / 100, abs % 100);
    Ok(match frac {
        0 => format!("{sign}{int}"),
        f if f % 10 == 0 => format!("{sign}{int}.{}", f / 10),
        f => format!("{sign}{int}.{f:02}"),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn rounds_and_strips() {
        assert_eq!(fmt_num(1.5).as_deref(), Ok("1.5"));
        assert_eq!(fmt_num(2.0).as_deref(), Ok("2"));
        assert_eq!(fmt_num(1.234).as_deref(), Ok("1.23"));
        assert_eq!(fmt_num(-1.07).as_deref(), Ok("-1.07"));
        assert_eq!(fmt_num(1234567.891).as_deref(), Ok("1234567.89"));
    }

    #[test]
    fn negative_zero_normalised() {
        assert_eq!(fmt_num(-0.0).as_deref(), Ok("0"));
        assert_eq!(fmt_num(-0.004).as_deref(), Ok("0"));
    }

    #[test]
    fn rejects_non_finite() {
        assert!(fmt_num(f64::NAN).is_err());
        assert!(fmt_num(f64::INFINITY).is_err());
        assert!(fmt_num(f64::NEG_INFINITY).is_err());
        assert!(fmt_num(1e300).is_err());
    }
}
