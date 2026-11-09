//! Box model and TeX-style layout (Appendix G rules 11, 13, 15, 18) driven by
//! OpenType MATH constants. All lengths are in em; y points up from the baseline.

use ttf_parser::GlyphId;

use super::atoms::{space, Atom, AtomClass, AtomContent, Style};
use super::error::MathError;
use super::font::{MathConsts, MathFont};
use super::parser::Glue;

/// Largest vertical stretch factor accepted for a scaled radical.
const MAX_RADICAL_STRETCH: f64 = 200.0;
/// Magnitude above which a coordinate is considered corrupt.
const MAX_COORD: f64 = 1.0e6;

/// A drawable placed inside a box.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    /// Glyph origin at `(x, y)` drawn at uniform `scale` (1.0 = 1 em).
    Glyph {
        gid: GlyphId,
        x: f64,
        y: f64,
        scale: f64,
    },
    /// Glyph stretched non-uniformly (radical fallback).
    ScaledGlyph {
        gid: GlyphId,
        x: f64,
        y: f64,
        sx: f64,
        sy: f64,
    },
    /// Filled rectangle with bottom-left corner `(x, y)`.
    Rule { x: f64, y: f64, w: f64, h: f64 },
}

impl Item {
    fn shifted(self, dx: f64, dy: f64) -> Item {
        match self {
            Item::Glyph { gid, x, y, scale } => Item::Glyph {
                gid,
                x: x + dx,
                y: y + dy,
                scale,
            },
            Item::ScaledGlyph { gid, x, y, sx, sy } => Item::ScaledGlyph {
                gid,
                x: x + dx,
                y: y + dy,
                sx,
                sy,
            },
            Item::Rule { x, y, w, h } => Item::Rule {
                x: x + dx,
                y: y + dy,
                w,
                h,
            },
        }
    }

    fn numbers(&self) -> [f64; 4] {
        match *self {
            Item::Glyph { x, y, scale, .. } => [x, y, scale, 0.0],
            Item::ScaledGlyph { x, y, sx, sy, .. } => [x, y, sx, sy],
            Item::Rule { x, y, w, h } => [x, y, w, h],
        }
    }
}

/// A laid-out box.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LBox {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// Italic correction of the rightmost glyph.
    pub italic: f64,
    pub items: Vec<Item>,
}

impl LBox {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    /// Appends `other` to the right of this box.
    pub(crate) fn append(&mut self, other: LBox) {
        let dx = self.width;
        self.items
            .extend(other.items.into_iter().map(|i| i.shifted(dx, 0.0)));
        self.width += other.width;
        self.height = self.height.max(other.height);
        self.depth = self.depth.max(other.depth);
        self.italic = other.italic;
    }

    /// Adds horizontal space.
    pub(crate) fn advance(&mut self, dx: f64) {
        self.width += dx;
        self.italic = 0.0;
    }

    /// Moves content up by `dy` (down when negative).
    pub(crate) fn shifted_up(self, dy: f64) -> LBox {
        LBox {
            width: self.width,
            height: self.height + dy,
            depth: self.depth - dy,
            italic: self.italic,
            items: self.items.into_iter().map(|i| i.shifted(0.0, dy)).collect(),
        }
    }

    /// Copies `child` into this box at `(dx, dy)` without changing the metrics.
    fn place(&mut self, child: LBox, dx: f64, dy: f64) {
        self.items
            .extend(child.items.into_iter().map(|i| i.shifted(dx, dy)));
    }

    /// Fails when any number is non-finite or absurdly large.
    pub(crate) fn finite(&self) -> Result<(), MathError> {
        let ok = |v: f64| v.is_finite() && v.abs() < MAX_COORD;
        let metrics = [self.width, self.height, self.depth, self.italic];
        if metrics.iter().all(|v| ok(*v))
            && self
                .items
                .iter()
                .all(|i| i.numbers().iter().all(|v| ok(*v)))
        {
            Ok(())
        } else {
            Err(MathError::Font {
                reason: "layout produced a non-finite coordinate".into(),
            })
        }
    }
}

/// Concatenates boxes horizontally.
#[cfg(test)]
pub(crate) fn hbox(children: Vec<LBox>) -> LBox {
    let mut out = LBox::empty();
    for c in children {
        out.append(c);
    }
    out
}

/// What a script nucleus is, which decides how scripts attach to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nucleus {
    /// A single character: scripts start at the baseline (TeX rule 18a).
    Glyph,
    /// A composite box.
    Boxed,
    /// A large operator whose scripts sit beside it; its italic correction
    /// positions the subscript to the left of the superscript.
    BigOp,
}

/// Layout engine bound to a font.
pub(crate) struct Layout<'a> {
    font: &'a MathFont,
    c: MathConsts,
}

fn font_err(reason: &str) -> MathError {
    MathError::Font {
        reason: reason.into(),
    }
}

impl<'a> Layout<'a> {
    pub(crate) fn new(font: &'a MathFont) -> Self {
        Self {
            font,
            c: *font.consts(),
        }
    }

    /// Lays out an atom list left to right with inter-atom spacing.
    pub(crate) fn layout_list(&self, atoms: &[Atom], style: Style) -> Result<LBox, MathError> {
        let mut out = LBox::empty();
        let mut prev: Option<AtomClass> = None;
        for atom in atoms {
            if let AtomContent::Space(glue) = &atom.content {
                // Glue is invisible to classification: `prev` stays the last real atom.
                out.advance(match *glue {
                    Glue::Mu(m) => m * style.mu_scale(&self.c),
                    Glue::Em(e) => e,
                });
                continue;
            }
            let b = self.layout_atom(atom, style)?;
            if let Some(p) = prev {
                let mu = space(p, atom.class, style);
                if mu > 0.0 {
                    out.advance(mu * style.mu_scale(&self.c));
                }
            }
            out.append(b);
            prev = Some(atom.class);
        }
        Ok(out)
    }

    fn layout_atom(&self, atom: &Atom, style: Style) -> Result<LBox, MathError> {
        match &atom.content {
            // Glue is applied in `layout_list`; reaching it here adds no width.
            AtomContent::Space(_) => Ok(LBox::empty()),
            AtomContent::Glyph(ch) => self.glyph_box(*ch, style),
            AtomContent::List(items) => self.layout_list(items, style),
            AtomContent::Frac { num, den } => {
                let n = self.layout_list(num, style.frac_num())?;
                let d = self.layout_list(den, style.frac_den())?;
                Ok(self.frac(n, d, style))
            }
            AtomContent::Sqrt { index, body } => {
                let b = self.layout_list(body, style.cramp())?;
                let i = match index {
                    Some(list) => Some(self.layout_list(list, style.script_script())?),
                    None => None,
                };
                self.sqrt(b, i, style)
            }
            AtomContent::Scripts { nucleus, sup, sub } => {
                let n = self.layout_atom(nucleus, style)?;
                let kind = if matches!(nucleus.content, AtomContent::Glyph(_)) {
                    Nucleus::Glyph
                } else {
                    Nucleus::Boxed
                };
                let (sup, sub) = self.layout_scripts(sup, sub, style)?;
                Ok(self.scripts(n, sup, sub, style, kind))
            }
            AtomContent::BigOp {
                ch,
                side_scripts,
                sup,
                sub,
            } => {
                let mut gid = self.font.glyph(*ch)?;
                if style.is_display() {
                    gid = self.font.display_variant(gid);
                }
                let g = self.gid_box(gid, style)?;
                let s = style.scale(&self.c);
                let centre = self.c.axis_height * s - (g.height - g.depth) / 2.0;
                let n = g.shifted_up(centre);
                let (sup, sub) = self.layout_scripts(sup, sub, style)?;
                if style.is_display() && !side_scripts {
                    Ok(self.limits(n, sup, sub, style))
                } else {
                    Ok(self.scripts(n, sup, sub, style, Nucleus::BigOp))
                }
            }
        }
    }

    fn layout_scripts(
        &self,
        sup: &Option<Vec<Atom>>,
        sub: &Option<Vec<Atom>>,
        style: Style,
    ) -> Result<(Option<LBox>, Option<LBox>), MathError> {
        let sup = match sup {
            Some(l) => Some(self.layout_list(l, style.sup())?),
            None => None,
        };
        let sub = match sub {
            Some(l) => Some(self.layout_list(l, style.sub())?),
            None => None,
        };
        Ok((sup, sub))
    }

    /// Box for one character.
    pub(crate) fn glyph_box(&self, ch: char, style: Style) -> Result<LBox, MathError> {
        let gid = self.font.glyph(ch)?;
        self.gid_box(gid, style)
    }

    pub(crate) fn gid_box(&self, gid: GlyphId, style: Style) -> Result<LBox, MathError> {
        let s = style.scale(&self.c);
        let bb = self.font.bbox(gid);
        Ok(LBox {
            width: self.font.advance(gid) * s,
            height: bb.map_or(0.0, |b| b.y_max) * s,
            depth: bb.map_or(0.0, |b| -b.y_min) * s,
            italic: self.font.italic_correction(gid) * s,
            items: vec![Item::Glyph {
                gid,
                x: 0.0,
                y: 0.0,
                scale: s,
            }],
        })
    }

    /// Rule 15: fraction with a bar centred on the math axis.
    pub(crate) fn frac(&self, num: LBox, den: LBox, style: Style) -> LBox {
        let c = &self.c;
        let s = style.scale(c);
        let display = style.is_display();
        let theta = c.fraction_rule_thickness * s;
        let axis = c.axis_height * s;
        let (num_up, den_down, num_gap, den_gap) = if display {
            (
                c.fraction_numerator_display_shift_up,
                c.fraction_denominator_display_shift_down,
                c.fraction_numerator_display_gap_min,
                c.fraction_denominator_display_gap_min,
            )
        } else {
            (
                c.fraction_numerator_shift_up,
                c.fraction_denominator_shift_down,
                c.fraction_numerator_gap_min,
                c.fraction_denominator_gap_min,
            )
        };
        let num_shift = (num_up * s).max(axis + theta / 2.0 + num_gap * s + num.depth);
        let den_shift = (den_down * s).max(den_gap * s + den.height - axis + theta / 2.0);
        let pad = 0.12 * s; // TeX \nulldelimiterspace
        let inner = num.width.max(den.width);
        let (num_dx, den_dx) = (
            pad + (inner - num.width) / 2.0,
            pad + (inner - den.width) / 2.0,
        );
        let mut out = LBox {
            width: inner + 2.0 * pad,
            height: num_shift + num.height,
            depth: den_shift + den.depth,
            italic: 0.0,
            items: vec![Item::Rule {
                x: pad,
                y: axis - theta / 2.0,
                w: inner,
                h: theta,
            }],
        };
        out.place(num, num_dx, num_shift);
        out.place(den, den_dx, -den_shift);
        out
    }

    /// Rule 11: radical. Picks the smallest sufficient size variant, else
    /// stretches the largest one vertically.
    pub(crate) fn sqrt(
        &self,
        body: LBox,
        index: Option<LBox>,
        style: Style,
    ) -> Result<LBox, MathError> {
        let c = &self.c;
        let s = style.scale(c);
        let t = c.radical_rule_thickness * s;
        let gap = if style.is_display() {
            c.radical_display_vertical_gap
        } else {
            c.radical_vertical_gap
        } * s;
        let needed = body.height + body.depth + gap + t;

        let root = self.font.glyph('\u{221A}')?;
        let sized: Vec<(GlyphId, f64)> = self
            .font
            .vertical_variants(root)
            .into_iter()
            .filter_map(|(g, _)| self.font.bbox(g).map(|b| (g, (b.y_max - b.y_min) * s)))
            .collect();
        let (gid, ink) = sized
            .iter()
            .find(|(_, e)| *e >= needed)
            .or_else(|| sized.last())
            .copied()
            .ok_or_else(|| font_err("radical glyph has no outline"))?;
        let bb = self
            .font
            .bbox(gid)
            .ok_or_else(|| font_err("radical glyph has no outline"))?;
        if ink <= 0.0 {
            return Err(font_err("radical glyph has zero height"));
        }
        let stretch = if ink < needed { needed / ink } else { 1.0 };
        if !stretch.is_finite() || stretch > MAX_RADICAL_STRETCH {
            return Err(font_err("radical stretch out of range"));
        }
        let total = ink * stretch;
        let excess = (total - needed).max(0.0);
        let bottom = -body.depth - excess / 2.0;
        let top = bottom + total;
        let sy = s * stretch;
        let glyph_y = bottom - bb.y_min * sy;
        let adv = self.font.advance(gid) * s;

        let (x_rad, deg_place) = match index {
            Some(deg) => {
                let x_deg = (c.radical_kern_before_degree * s).max(0.0);
                let x_rad = (x_deg + deg.width + c.radical_kern_after_degree * s).max(0.0);
                let raise = bottom + c.radical_degree_bottom_raise_percent / 100.0 * total;
                (x_rad, Some((deg, x_deg, raise)))
            }
            None => (0.0, None),
        };

        let overlap = 0.01 * s;
        let mut out = LBox {
            width: x_rad + adv + body.width,
            height: top + c.radical_extra_ascender * s,
            depth: -bottom,
            italic: 0.0,
            items: vec![if stretch > 1.0 {
                Item::ScaledGlyph {
                    gid,
                    x: x_rad,
                    y: glyph_y,
                    sx: s,
                    sy,
                }
            } else {
                Item::Glyph {
                    gid,
                    x: x_rad,
                    y: glyph_y,
                    scale: s,
                }
            }],
        };
        out.depth = out.depth.max(body.depth);
        out.items.push(Item::Rule {
            x: x_rad + adv - overlap,
            y: top - t,
            w: body.width + overlap,
            h: t,
        });
        out.place(body, x_rad + adv, 0.0);
        if let Some((deg, x_deg, raise)) = deg_place {
            let base = raise + deg.depth;
            out.height = out.height.max(base + deg.height);
            out.place(deg, x_deg, base);
        }
        Ok(out)
    }

    /// Rule 18: side scripts.
    pub(crate) fn scripts(
        &self,
        nucleus: LBox,
        sup: Option<LBox>,
        sub: Option<LBox>,
        style: Style,
        kind: Nucleus,
    ) -> LBox {
        let c = &self.c;
        let s = style.scale(c);
        let (mut u, mut v) = if kind == Nucleus::Glyph {
            (0.0, 0.0)
        } else {
            (
                nucleus.height - c.superscript_baseline_drop_max * style.sup().scale(c),
                nucleus.depth + c.subscript_baseline_drop_min * style.sub().scale(c),
            )
        };
        let shift_up = if style.cramped {
            c.superscript_shift_up_cramped
        } else {
            c.superscript_shift_up
        } * s;
        if let Some(p) = &sup {
            u = u.max(shift_up).max(p.depth + c.superscript_bottom_min * s);
        }
        if let Some(b) = &sub {
            v = v.max(c.subscript_shift_down * s);
            if sup.is_none() {
                v = v.max(b.height - c.subscript_top_max * s);
            }
        }
        if let (Some(p), Some(b)) = (&sup, &sub) {
            let gap = (u - p.depth) - (b.height - v);
            let min_gap = c.sub_superscript_gap_min * s;
            if gap < min_gap {
                v += min_gap - gap;
                let psi = c.superscript_bottom_max_with_subscript * s - (u - p.depth);
                if psi > 0.0 {
                    u += psi;
                    v -= psi;
                }
            }
        }
        let (sup_x, sub_x) = if kind == Nucleus::BigOp {
            (nucleus.width, (nucleus.width - nucleus.italic).max(0.0))
        } else {
            (nucleus.width + nucleus.italic, nucleus.width)
        };
        let mut right = nucleus.width;
        let mut height = nucleus.height;
        let mut depth = nucleus.depth;
        let after = c.space_after_script * s;
        if let Some(p) = &sup {
            right = right.max(sup_x + p.width + after);
            height = height.max(u + p.height);
        }
        if let Some(b) = &sub {
            right = right.max(sub_x + b.width + after);
            depth = depth.max(v + b.depth);
        }
        let mut out = LBox {
            width: right,
            height,
            depth,
            italic: 0.0,
            items: Vec::new(),
        };
        out.items = nucleus.items;
        if let Some(p) = sup {
            out.place(p, sup_x, u);
        }
        if let Some(b) = sub {
            out.place(b, sub_x, -v);
        }
        out
    }

    /// Rule 13: limits centred above and below a large operator.
    pub(crate) fn limits(
        &self,
        nucleus: LBox,
        sup: Option<LBox>,
        sub: Option<LBox>,
        style: Style,
    ) -> LBox {
        let c = &self.c;
        let s = style.scale(c);
        let delta = nucleus.italic;
        let mut width = nucleus.width;
        for b in [&sup, &sub].into_iter().flatten() {
            width = width.max(b.width);
        }
        let mut height = nucleus.height;
        let mut depth = nucleus.depth;
        let nucleus_dx = (width - nucleus.width) / 2.0;
        let (top, bottom) = (nucleus.height, nucleus.depth);
        let mut out = LBox {
            width,
            height,
            depth,
            italic: 0.0,
            items: Vec::new(),
        };
        out.place(nucleus, nucleus_dx, 0.0);
        if let Some(p) = sup {
            let base = (top + c.upper_limit_gap_min * s + p.depth)
                .max(top + c.upper_limit_baseline_rise_min * s);
            height = height.max(base + p.height);
            let dx = (width - p.width) / 2.0 + delta / 2.0;
            out.place(p, dx, base);
        }
        if let Some(b) = sub {
            let drop = (bottom + c.lower_limit_gap_min * s + b.height)
                .max(bottom + c.lower_limit_baseline_drop_min * s);
            depth = depth.max(drop + b.depth);
            let dx = (width - b.width) / 2.0 - delta / 2.0;
            out.place(b, dx, -drop);
        }
        out.height = height;
        out.depth = depth;
        out
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

    fn font() -> MathFont {
        MathFont::load().unwrap()
    }

    fn rule_of(b: &LBox) -> (f64, f64, f64, f64) {
        b.items
            .iter()
            .find_map(|i| match i {
                Item::Rule { x, y, w, h } => Some((*x, *y, *w, *h)),
                _ => None,
            })
            .unwrap()
    }

    fn glyph_x(b: &LBox) -> Vec<f64> {
        b.items
            .iter()
            .filter_map(|i| match i {
                Item::Glyph { x, .. } => Some(*x),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn hbox_sums_widths_and_maxes_extents() {
        let a = LBox {
            width: 1.0,
            height: 0.5,
            depth: 0.1,
            ..LBox::default()
        };
        let b = LBox {
            width: 2.0,
            height: 0.2,
            depth: 0.3,
            ..LBox::default()
        };
        let h = hbox(vec![a, b]);
        assert_eq!((h.width, h.height, h.depth), (3.0, 0.5, 0.3));
    }

    #[test]
    fn shift_moves_items() {
        let f = font();
        let l = Layout::new(&f);
        let g = l.glyph_box('\u{1D465}', Style::DISPLAY).unwrap();
        let (h, d) = (g.height, g.depth);
        let s = g.shifted_up(0.25);
        assert!((s.height - (h + 0.25)).abs() < 1e-12);
        assert!((s.depth - (d - 0.25)).abs() < 1e-12);
        assert!(matches!(s.items[0], Item::Glyph { y, .. } if (y - 0.25).abs() < 1e-12));
    }

    #[test]
    fn glyph_box_uses_font_advance() {
        let f = font();
        let l = Layout::new(&f);
        let ch = '\u{1D44E}';
        let g = l.glyph_box(ch, Style::DISPLAY).unwrap();
        assert!((g.width - f.advance(f.glyph(ch).unwrap())).abs() < 1e-12);
        let s = l.glyph_box(ch, Style::DISPLAY.sup()).unwrap();
        assert!((s.width / g.width - f.consts().script_percent_scale_down / 100.0).abs() < 1e-9);
    }

    #[test]
    fn frac_is_centred_with_font_bar_thickness() {
        let f = font();
        let l = Layout::new(&f);
        let n = l.glyph_box('1', Style::DISPLAY.frac_num()).unwrap();
        let d = hbox(vec![
            l.glyph_box('2', Style::DISPLAY.frac_den()).unwrap(),
            l.glyph_box('3', Style::DISPLAY.frac_den()).unwrap(),
            l.glyph_box('4', Style::DISPLAY.frac_den()).unwrap(),
        ]);
        let (nw, dw) = (n.width, d.width);
        let b = l.frac(n, d, Style::DISPLAY);
        let (rx, ry, rw, rh) = rule_of(&b);
        assert!((rh - f.consts().fraction_rule_thickness).abs() < 1e-9);
        assert!((ry + rh / 2.0 - f.consts().axis_height).abs() < 1e-9);
        let bar_centre = rx + rw / 2.0;
        let xs = glyph_x(&b);
        let num_centre = xs[0] + nw / 2.0;
        let den_centre = xs[1] + dw / 2.0;
        assert!((num_centre - bar_centre).abs() < 0.001);
        assert!((den_centre - bar_centre).abs() < 0.001);
        assert!(b.height > f.consts().axis_height && b.depth > 0.0);
    }

    #[test]
    fn sqrt_is_taller_than_body() {
        let f = font();
        let l = Layout::new(&f);
        let body = l.glyph_box('\u{1D465}', Style::DISPLAY.cramp()).unwrap();
        let (bh, bd) = (body.height, body.depth);
        let c = *f.consts();
        let r = l.sqrt(body, None, Style::DISPLAY).unwrap();
        assert!(
            r.height + r.depth
                >= bh + bd + c.radical_rule_thickness + c.radical_display_vertical_gap - 1e-9
        );
        assert!(r.items.iter().any(|i| matches!(i, Item::Rule { .. })));
    }

    #[test]
    fn sqrt_of_huge_body_scales_instead_of_failing() {
        let f = font();
        let l = Layout::new(&f);
        let body = LBox {
            width: 1.0,
            height: 40.0,
            depth: 10.0,
            ..LBox::default()
        };
        let r = l.sqrt(body, None, Style::DISPLAY).unwrap();
        assert!(r
            .items
            .iter()
            .any(|i| matches!(i, Item::ScaledGlyph { sy, .. } if *sy > 1.0)));
        assert!(r.height >= 40.0);
        r.finite().unwrap();
    }

    #[test]
    fn finite_rejects_nan_without_panicking() {
        let mut b = LBox::default();
        b.items.push(Item::Rule {
            x: f64::NAN,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        });
        assert!(b.finite().is_err());
        let b = LBox {
            width: f64::INFINITY,
            ..LBox::default()
        };
        assert!(b.finite().is_err());
        assert!(LBox::default().finite().is_ok());
    }
}
