//! Box model and TeX-style layout (Appendix G rules 11, 13, 15, 18) driven by
//! OpenType MATH constants. All lengths are in em; y points up from the baseline.

use ttf_parser::GlyphId;

use super::atoms::{space, Atom, AtomClass, AtomContent, Style};
use super::error::MathError;
use super::font::{Assembly, AssemblyPart, Corner, MathConsts, MathFont};
use super::parser::Glue;

/// Largest vertical stretch factor accepted for a scaled radical.
const MAX_RADICAL_STRETCH: f64 = 200.0;
/// Most times each extender part is repeated in a glyph assembly.
const MAX_EXTENDERS: usize = 64;
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
                let (kind, gid) = match &nucleus.content {
                    AtomContent::Glyph(ch) => (Nucleus::Glyph, self.font.glyph(*ch).ok()),
                    _ => (Nucleus::Boxed, None),
                };
                let (sup, sub) = self.layout_scripts(sup, sub, style)?;
                Ok(self.scripts(n, sup, sub, style, kind, gid))
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
                    Ok(self.scripts(n, sup, sub, style, Nucleus::BigOp, None))
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

    /// A glyph stretched to at least `required` em of total vertical extent
    /// (height + depth, at the style's scale). Order: the smallest of the base
    /// glyph and its size variants that is tall enough, then an OpenType glyph
    /// assembly, then the tallest variant available (never fails for lack of
    /// size). Non-finite or non-positive `required` gives the base glyph, except
    /// `+∞`, which gives the tallest allowed result.
    ///
    /// The ink of the result spans `-depth..height`; assemblies sit on the
    /// baseline (`depth == 0`).
    pub(crate) fn stretchy(
        &self,
        gid: GlyphId,
        required: f64,
        style: Style,
    ) -> Result<LBox, MathError> {
        let assembly = self.font.assembly(gid);
        if assembly.is_none() && self.font.bbox(gid).is_none() {
            return Err(font_err("stretchy glyph has no outline"));
        }
        if required.is_nan() || required <= 0.0 {
            return self.gid_box(gid, style);
        }
        let s = style.scale(&self.c);
        let sized: Vec<(GlyphId, f64)> = self
            .font
            .vertical_variants(gid)
            .into_iter()
            .filter_map(|(g, _)| self.font.bbox(g).map(|b| (g, (b.y_max - b.y_min) * s)))
            .collect();
        if let Some((g, _)) = sized.iter().find(|(_, e)| *e >= required) {
            return self.gid_box(*g, style);
        }
        if let Some(asm) = assembly {
            return self.assemble(&asm, required, style);
        }
        match sized.last() {
            Some((g, _)) => self.gid_box(*g, style),
            None => self.gid_box(gid, style),
        }
    }

    /// Stacks assembly parts bottom to top to reach `required` em (OpenType
    /// MATH, "Glyph assembly"). Uses the fewest extender repetitions that can
    /// reach it (capped at [`MAX_EXTENDERS`]), then spreads the overlap
    /// uniformly between its minimum and maximum to hit `required` exactly.
    fn assemble(&self, asm: &Assembly, required: f64, style: Style) -> Result<LBox, MathError> {
        let s = style.scale(&self.c);
        let min_ov = asm.min_overlap * s;
        let has_ext = asm.parts.iter().any(|p| p.extender);
        let sequence = |n: usize| -> Vec<&AssemblyPart> {
            asm.parts
                .iter()
                .flat_map(|p| {
                    let k = if p.extender { n } else { 1 };
                    (0..k).map(move |_| p)
                })
                .collect()
        };
        // Joint overlap limits (scaled): the lower bound always wins.
        let max_overlap = |a: &AssemblyPart, b: &AssemblyPart| -> f64 {
            min_ov.max(a.end_connector.min(b.start_connector) * s)
        };
        // (sum of advances, sum of maximum overlaps, joint count) for a sequence.
        let measure = |seq: &[&AssemblyPart]| -> (f64, f64, usize) {
            let adv: f64 = seq.iter().map(|p| p.advance * s).sum();
            let ov: f64 = seq
                .windows(2)
                .map(|w| match w {
                    [a, b] => max_overlap(a, b),
                    _ => 0.0,
                })
                .sum();
            (adv, ov, seq.len().saturating_sub(1))
        };
        let reach = |n: usize| -> f64 {
            let seq = sequence(n);
            let (adv, _, joints) = measure(&seq);
            adv - joints as f64 * min_ov
        };
        let n = if has_ext {
            (1..=MAX_EXTENDERS)
                .find(|&n| reach(n) >= required)
                .unwrap_or(MAX_EXTENDERS)
        } else {
            0
        };
        let seq = sequence(n);
        let (adv, ov_max_sum, joints) = measure(&seq);
        let min_h = adv - ov_max_sum;
        let max_h = adv - joints as f64 * min_ov;
        // Fraction of the way from maximum overlap (0) to minimum overlap (1).
        let t = if max_h > min_h && required.is_finite() {
            ((required - min_h) / (max_h - min_h)).clamp(0.0, 1.0)
        } else if required.is_finite() {
            0.0
        } else {
            1.0
        };
        let mut items = Vec::with_capacity(seq.len());
        let (mut cursor, mut width) = (0.0_f64, 0.0_f64);
        for (i, part) in seq.iter().enumerate() {
            let y_min = self.font.bbox(part.gid).map_or(0.0, |b| b.y_min);
            items.push(Item::Glyph {
                gid: part.gid,
                x: 0.0,
                y: cursor - y_min * s,
                scale: s,
            });
            width = width.max(self.font.advance(part.gid) * s);
            cursor += part.advance * s;
            if let Some(next) = seq.get(i + 1) {
                let o_max = max_overlap(part, next);
                cursor -= o_max + t * (min_ov - o_max);
            }
        }
        let out = LBox {
            width,
            height: cursor,
            depth: 0.0,
            italic: 0.0,
            items,
        };
        out.finite()?;
        Ok(out)
    }

    /// Rule 11: radical. Picks the smallest sufficient size variant, else
    /// assembles one from parts, else (no assembly in the font) stretches the
    /// largest variant vertically.
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
        let radical = self.stretchy(root, needed, style)?;
        let ink = radical.height + radical.depth;
        if ink <= 0.0 {
            return Err(font_err("radical glyph has zero height"));
        }
        // Only a single, still-too-short glyph (font without an assembly) is
        // scaled; assembled radicals are never distorted.
        let scalable = match radical.items.as_slice() {
            [Item::Glyph { gid, .. }] => Some(*gid),
            _ => None,
        };
        let stretch = match scalable {
            Some(_) if ink < needed => needed / ink,
            _ => 1.0,
        };
        if !stretch.is_finite() || stretch > MAX_RADICAL_STRETCH {
            return Err(font_err("radical stretch out of range"));
        }
        let total = ink * stretch;
        let excess = (total - needed).max(0.0);
        let bottom = -body.depth - excess / 2.0;
        let top = bottom + total;
        let sy = s * stretch;
        // The radical's ink bottom sits at `-radical.depth` before scaling.
        let glyph_y = bottom + radical.depth * stretch;
        let adv = radical.width;

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
        let radical_items = match scalable {
            Some(gid) if stretch > 1.0 => vec![Item::ScaledGlyph {
                gid,
                x: x_rad,
                y: glyph_y,
                sx: s,
                sy,
            }],
            _ => radical
                .items
                .into_iter()
                .map(|i| i.shifted(x_rad, glyph_y))
                .collect(),
        };
        let mut out = LBox {
            width: x_rad + adv + body.width,
            height: top + c.radical_extra_ascender * s,
            depth: -bottom,
            italic: 0.0,
            items: radical_items,
        };
        // A capped assembly can be shorter than the body; keep the box around it.
        out.height = out.height.max(body.height);
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

    /// Horizontal MathKern for a script on a single-glyph nucleus: the font's
    /// kern at the script's own edge height plus the kern at the nucleus edge
    /// height (both in em at the style scale; zero without kern data).
    fn corner_kern_shift(
        &self,
        gid: GlyphId,
        corner: Corner,
        script_edge: f64,
        nucleus_edge: f64,
        s: f64,
    ) -> f64 {
        if !(s.is_finite() && s > 0.0) {
            return 0.0;
        }
        self.font.corner_kern(gid, corner).map_or(0.0, |k| {
            let shift = (k.at(script_edge / s) + k.at(nucleus_edge / s)) * s;
            if shift.is_finite() {
                shift
            } else {
                0.0
            }
        })
    }

    /// Rule 18: side scripts. `nucleus_gid` is the nucleus glyph when it is a
    /// single character; its MathKern data then refines script placement.
    pub(crate) fn scripts(
        &self,
        nucleus: LBox,
        sup: Option<LBox>,
        sub: Option<LBox>,
        style: Style,
        kind: Nucleus,
        nucleus_gid: Option<GlyphId>,
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
        let (mut sup_x, mut sub_x) = if kind == Nucleus::BigOp {
            (nucleus.width, (nucleus.width - nucleus.italic).max(0.0))
        } else {
            (nucleus.width + nucleus.italic, nucleus.width)
        };
        // MathKern applies on top of the italic correction, never replacing it.
        if let (Nucleus::Glyph, Some(gid)) = (kind, nucleus_gid) {
            if let Some(p) = &sup {
                sup_x +=
                    self.corner_kern_shift(gid, Corner::TopRight, u - p.depth, nucleus.height, s);
            }
            if let Some(b) = &sub {
                sub_x += self.corner_kern_shift(
                    gid,
                    Corner::BottomRight,
                    b.height - v,
                    -nucleus.depth,
                    s,
                );
            }
        }
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
    fn sqrt_of_huge_body_uses_assembly() {
        let f = font();
        let l = Layout::new(&f);
        let body = LBox {
            width: 1.0,
            height: 40.0,
            depth: 10.0,
            ..LBox::default()
        };
        let r = l.sqrt(body, None, Style::DISPLAY).unwrap();
        assert!(!r
            .items
            .iter()
            .any(|i| matches!(i, Item::ScaledGlyph { .. })));
        let glyphs = r
            .items
            .iter()
            .filter(|i| matches!(i, Item::Glyph { .. }))
            .count();
        assert!(glyphs >= 3, "radical should be several assembled parts");
        r.finite().unwrap();
    }

    #[test]
    fn sqrt_of_moderate_body_covers_it_with_assembly() {
        let f = font();
        let l = Layout::new(&f);
        let body = LBox {
            width: 1.0,
            height: 9.0,
            depth: 3.0,
            ..LBox::default()
        };
        let r = l.sqrt(body, None, Style::DISPLAY).unwrap();
        assert!(!r
            .items
            .iter()
            .any(|i| matches!(i, Item::ScaledGlyph { .. })));
        assert!(r.height >= 9.0 && r.depth >= 3.0);
    }

    // ---- stretchy / glyph assembly (spec 004) ----

    const S: Style = Style::DISPLAY;

    fn ink(f: &MathFont, g: GlyphId) -> f64 {
        let b = f.bbox(g).unwrap();
        (b.y_max - b.y_min) * S.scale(f.consts())
    }

    fn only_glyph(b: &LBox) -> GlyphId {
        match b.items.as_slice() {
            [Item::Glyph { gid, .. }] => *gid,
            other => panic!("expected one glyph, got {other:?}"),
        }
    }

    fn extender_count(b: &LBox, asm: &crate::math::latex::font::Assembly) -> usize {
        // Items = required parts once + every extender part `n` times.
        let ext = asm.parts.iter().filter(|p| p.extender).count();
        let required = asm.parts.len() - ext;
        b.items.len().saturating_sub(required) / ext.max(1)
    }

    #[test]
    fn stretchy_picks_base_then_smallest_variant() {
        let f = font();
        let l = Layout::new(&f);
        let paren = f.glyph('(').unwrap();
        let v = f.vertical_variants(paren);
        assert!(v.len() >= 3);
        let below = l.stretchy(paren, ink(&f, v[0].0) * 0.5, S).unwrap();
        assert_eq!(only_glyph(&below), v[0].0);
        let mid = (ink(&f, v[0].0) + ink(&f, v[1].0)) / 2.0;
        let between = l.stretchy(paren, mid, S).unwrap();
        assert_eq!(only_glyph(&between), v[1].0);
        let exact = l.stretchy(paren, ink(&f, v[2].0), S).unwrap();
        assert_eq!(only_glyph(&exact), v[2].0);
    }

    #[test]
    fn stretchy_beyond_variants_uses_minimal_extenders() {
        let f = font();
        let l = Layout::new(&f);
        let paren = f.glyph('(').unwrap();
        let largest = f.vertical_variants(paren).last().unwrap().0;
        let required = ink(&f, largest) + 1.0;
        let b = l.stretchy(paren, required, S).unwrap();
        assert!(b.items.iter().all(|i| matches!(i, Item::Glyph { .. })));
        assert!(b.height + b.depth >= required - 1e-9);
        let asm = f.assembly(paren).unwrap();
        let n = extender_count(&b, &asm);
        assert!((1..=MAX_EXTENDERS).contains(&n));
        let s = S.scale(f.consts());
        let max_h = |n: usize| -> f64 {
            let adv: f64 = asm
                .parts
                .iter()
                .map(|p| p.advance * s * if p.extender { n as f64 } else { 1.0 })
                .sum();
            let count: usize = asm
                .parts
                .iter()
                .map(|p| if p.extender { n } else { 1 })
                .sum();
            adv - (count - 1) as f64 * asm.min_overlap * s
        };
        assert!(max_h(n) >= required - 1e-9);
        assert!(n == 1 || max_h(n - 1) < required);
    }

    #[test]
    fn stretchy_without_data_returns_base_glyph() {
        let f = font();
        let l = Layout::new(&f);
        let zero = f.glyph('0').unwrap();
        let b = l.stretchy(zero, 25.0, S).unwrap();
        assert_eq!(only_glyph(&b), zero);
    }

    #[test]
    fn stretchy_degenerate_requests_are_safe() {
        let f = font();
        let l = Layout::new(&f);
        let paren = f.glyph('(').unwrap();
        for r in [f64::NAN, 0.0, -3.0, f64::NEG_INFINITY, -0.0] {
            let b = l.stretchy(paren, r, S).unwrap();
            assert_eq!(only_glyph(&b), paren, "required {r}");
        }
        let asm = f.assembly(paren).unwrap();
        for r in [f64::INFINITY, 1e12] {
            let b = l.stretchy(paren, r, S).unwrap();
            b.finite().unwrap();
            assert!(extender_count(&b, &asm) <= MAX_EXTENDERS, "required {r}");
            assert!(b.items.len() > 1);
        }
    }

    #[test]
    fn assembly_geometry_obeys_overlap_rules() {
        let f = font();
        let l = Layout::new(&f);
        let s = S.scale(f.consts());
        for ch in ['(', '[', '{', '|', '\u{222B}', '\u{221A}'] {
            let gid = f.glyph(ch).unwrap();
            let asm = f.assembly(gid).unwrap();
            let largest = f.vertical_variants(gid).last().unwrap().0;
            for extra in [0.3, 2.0, 7.5] {
                let required = ink(&f, largest) + extra;
                let b = l.assemble(&asm, required, S).unwrap();
                let n = extender_count(&b, &asm);
                // Expected part order, bottom to top.
                let expected: Vec<GlyphId> = asm
                    .parts
                    .iter()
                    .flat_map(|p| {
                        let k = if p.extender { n } else { 1 };
                        (0..k).map(move |_| p.gid)
                    })
                    .collect();
                let got: Vec<GlyphId> = b
                    .items
                    .iter()
                    .map(|i| match i {
                        Item::Glyph { gid, .. } => *gid,
                        other => panic!("non-glyph item {other:?}"),
                    })
                    .collect();
                assert_eq!(got, expected, "{ch}: order");
                // Reconstruct each part's logical bottom from its ink bottom.
                let seq: Vec<&crate::math::latex::font::AssemblyPart> = asm
                    .parts
                    .iter()
                    .flat_map(|p| {
                        let k = if p.extender { n } else { 1 };
                        (0..k).map(move |_| p)
                    })
                    .collect();
                let bottoms: Vec<f64> = b
                    .items
                    .iter()
                    .map(|i| match i {
                        Item::Glyph { gid, y, .. } => y + f.bbox(*gid).unwrap().y_min * s,
                        _ => unreachable!(),
                    })
                    .collect();
                let (mut sum_min, mut sum_max) = (0.0, 0.0);
                for j in 0..seq.len() - 1 {
                    let overlap = bottoms[j] + seq[j].advance * s - bottoms[j + 1];
                    let o_min = asm.min_overlap * s;
                    let o_max = o_min.max(seq[j].end_connector.min(seq[j + 1].start_connector) * s);
                    assert!(overlap >= o_min - 1e-9, "{ch}: overlap {overlap} < min");
                    assert!(overlap <= o_max + 1e-9, "{ch}: overlap {overlap} > max");
                    sum_min += o_min;
                    sum_max += o_max;
                }
                let adv: f64 = seq.iter().map(|p| p.advance * s).sum();
                let (min_h, max_h) = (adv - sum_max, adv - sum_min);
                let total = b.height + b.depth;
                if (min_h..=max_h).contains(&required) {
                    assert!(
                        (total - required).abs() < 1e-9,
                        "{ch}: {total} != {required}"
                    );
                } else if required < min_h {
                    assert!((total - min_h).abs() < 1e-9, "{ch}: should use max overlap");
                } else {
                    assert!((total - max_h).abs() < 1e-9, "{ch}: should use min overlap");
                }
                b.finite().unwrap();
            }
        }
    }

    /// Maximum extent reachable by the capped assembly of `gid`, in em at `S`.
    fn max_reach(l: &Layout<'_>, gid: GlyphId) -> f64 {
        let b = l.stretchy(gid, f64::INFINITY, S).unwrap();
        b.height + b.depth
    }

    #[test]
    fn records_reachable_height_per_glyph() {
        // Research R4: record rather than assume how far MAX_EXTENDERS reaches.
        let f = font();
        let l = Layout::new(&f);
        for ch in ['(', ')', '[', ']', '{', '}', '|', '\u{222B}', '\u{221A}'] {
            let gid = f.glyph(ch).unwrap();
            let base = ink(&f, f.vertical_variants(gid)[0].0);
            let reach = max_reach(&l, gid);
            eprintln!("reach {ch}: {reach:.2} em = {:.1}x base", reach / base);
            assert!(reach > base, "{ch} cannot stretch past its base glyph");
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(512))]

        #[test]
        fn stretchy_never_panics_and_meets_request(
            idx in 0usize..9,
            factor in 0.0f64..100.0,
            special in proptest::option::of(0usize..5),
        ) {
            let f = font();
            let l = Layout::new(&f);
            let chars = ['(', ')', '[', ']', '{', '}', '|', '\u{222B}', '\u{221A}'];
            let gid = f.glyph(chars[idx]).unwrap();
            let base = ink(&f, f.vertical_variants(gid)[0].0);
            let reach = max_reach(&l, gid);
            let required = match special {
                Some(0) => f64::NAN,
                Some(1) => f64::INFINITY,
                Some(2) => f64::NEG_INFINITY,
                Some(3) => -factor,
                _ => base * factor,
            };
            let b = l.stretchy(gid, required, S).unwrap();
            b.finite().unwrap();
            if required.is_finite() && required > 0.0 && required <= reach {
                proptest::prop_assert!(
                    b.height + b.depth >= required - 1e-9,
                    "{} required {} got {}", chars[idx], required, b.height + b.depth
                );
            }
        }
    }

    // ---- MathKern corner kerning (spec 004) ----

    use crate::math::latex::atoms::math_char;

    /// Expected superscript kern for `gid` given the placed script, em at `S`.
    fn sup_kern(f: &MathFont, nucleus: &LBox, gid: GlyphId, script_bottom: f64) -> f64 {
        let s = S.scale(f.consts());
        f.corner_kern(gid, Corner::TopRight).map_or(0.0, |k| {
            (k.at(script_bottom / s) + k.at(nucleus.height / s)) * s
        })
    }

    fn sub_kern(f: &MathFont, nucleus: &LBox, gid: GlyphId, script_top: f64) -> f64 {
        let s = S.scale(f.consts());
        f.corner_kern(gid, Corner::BottomRight).map_or(0.0, |k| {
            (k.at(script_top / s) + k.at(-nucleus.depth / s)) * s
        })
    }

    /// A letter whose glyph has both corner kerns and nonzero kern when a
    /// plain script is attached.
    fn kerned_letter(f: &MathFont, l: &Layout<'_>) -> (GlyphId, LBox) {
        for c in ('a'..='z').chain('A'..='Z') {
            let gid = f.glyph(math_char(c)).unwrap();
            let n = l.gid_box(gid, S).unwrap();
            let p = l.glyph_box('2', S.sup()).unwrap();
            let b = l.glyph_box('2', S.sub()).unwrap();
            let up = l.scripts(n.clone(), Some(p.clone()), None, S, Nucleus::Glyph, None);
            let down = l.scripts(n.clone(), None, Some(b.clone()), S, Nucleus::Glyph, None);
            let u = match up.items[1] {
                Item::Glyph { y, .. } => y,
                _ => unreachable!(),
            };
            let v = match down.items[1] {
                Item::Glyph { y, .. } => -y,
                _ => unreachable!(),
            };
            if sup_kern(f, &n, gid, u - p.depth) != 0.0 && sub_kern(f, &n, gid, b.height - v) != 0.0
            {
                return (gid, n);
            }
        }
        panic!("no letter with nonzero corner kerns in the subset");
    }

    #[test]
    fn superscript_is_offset_by_exactly_the_kern() {
        let f = font();
        let l = Layout::new(&f);
        let (gid, n) = kerned_letter(&f, &l);
        let p = l.glyph_box('2', S.sup()).unwrap();
        let plain = l.scripts(n.clone(), Some(p.clone()), None, S, Nucleus::Glyph, None);
        let kerned = l.scripts(
            n.clone(),
            Some(p.clone()),
            None,
            S,
            Nucleus::Glyph,
            Some(gid),
        );
        let (Item::Glyph { x: x0, y: u0, .. }, Item::Glyph { x: x1, y: u1, .. }) =
            (plain.items[1].clone(), kerned.items[1].clone())
        else {
            panic!("expected glyph items");
        };
        assert_eq!(u0, u1, "kerning moves scripts horizontally only");
        let want = sup_kern(&f, &n, gid, u0 - p.depth);
        assert!(want != 0.0);
        // Italic correction applies once, kern is added after it (FR-011).
        assert!((x0 - (n.width + n.italic)).abs() < 1e-12);
        assert!((x1 - x0 - want).abs() < 1e-12, "{} vs {want}", x1 - x0);
    }

    #[test]
    fn subscript_is_offset_by_exactly_the_kern() {
        let f = font();
        let l = Layout::new(&f);
        let (gid, n) = kerned_letter(&f, &l);
        let b = l.glyph_box('2', S.sub()).unwrap();
        let plain = l.scripts(n.clone(), None, Some(b.clone()), S, Nucleus::Glyph, None);
        let kerned = l.scripts(
            n.clone(),
            None,
            Some(b.clone()),
            S,
            Nucleus::Glyph,
            Some(gid),
        );
        let (Item::Glyph { x: x0, y, .. }, Item::Glyph { x: x1, .. }) =
            (plain.items[1].clone(), kerned.items[1].clone())
        else {
            panic!("expected glyph items");
        };
        let want = sub_kern(&f, &n, gid, b.height + y);
        assert!(want != 0.0);
        assert!((x0 - n.width).abs() < 1e-12);
        assert!((x1 - x0 - want).abs() < 1e-12);
    }

    #[test]
    fn sup_and_sub_are_kerned_by_their_own_heights() {
        let f = font();
        let l = Layout::new(&f);
        let (gid, n) = kerned_letter(&f, &l);
        let p = l.glyph_box('2', S.sup()).unwrap();
        let b = l.glyph_box('3', S.sub()).unwrap();
        let args = |g| {
            (
                n.clone(),
                Some(p.clone()),
                Some(b.clone()),
                S,
                Nucleus::Glyph,
                g,
            )
        };
        let (n1, p1, b1, s1, k1, g1) = args(None);
        let plain = l.scripts(n1, p1, b1, s1, k1, g1);
        let (n2, p2, b2, s2, k2, g2) = args(Some(gid));
        let kerned = l.scripts(n2, p2, b2, s2, k2, g2);
        let xy = |o: &LBox, i: usize| match o.items[i] {
            Item::Glyph { x, y, .. } => (x, y),
            _ => unreachable!(),
        };
        let ((sx0, su), (sx1, _)) = (xy(&plain, 1), (xy(&kerned, 1)));
        let ((bx0, sy), (bx1, _)) = (xy(&plain, 2), (xy(&kerned, 2)));
        assert!((sx1 - sx0 - sup_kern(&f, &n, gid, su - p.depth)).abs() < 1e-12);
        assert!((bx1 - bx0 - sub_kern(&f, &n, gid, b.height + sy)).abs() < 1e-12);
    }

    #[test]
    fn no_kern_data_or_composite_nucleus_is_unchanged() {
        let f = font();
        let l = Layout::new(&f);
        let p = l.glyph_box('2', S.sup()).unwrap();
        let b = l.glyph_box('3', S.sub()).unwrap();
        // '=' carries no corner kerns in the subset.
        let eq = f.glyph('=').unwrap();
        assert!(f.corner_kern(eq, Corner::TopRight).is_none());
        let n = l.gid_box(eq, S).unwrap();
        let a = l.scripts(
            n.clone(),
            Some(p.clone()),
            Some(b.clone()),
            S,
            Nucleus::Glyph,
            None,
        );
        let c = l.scripts(
            n,
            Some(p.clone()),
            Some(b.clone()),
            S,
            Nucleus::Glyph,
            Some(eq),
        );
        assert_eq!(a, c);
        // A composite nucleus ignores any gid even when that glyph has kerns.
        let (gid, n) = kerned_letter(&f, &l);
        let a = l.scripts(
            n.clone(),
            Some(p.clone()),
            Some(b.clone()),
            S,
            Nucleus::Boxed,
            None,
        );
        let c = l.scripts(n, Some(p), Some(b), S, Nucleus::Boxed, Some(gid));
        assert_eq!(a, c);
    }

    #[test]
    fn assembly_degenerate_shapes_terminate() {
        let f = font();
        let l = Layout::new(&f);
        let g = f.glyph('[').unwrap();
        let part = |extender| crate::math::latex::font::AssemblyPart {
            gid: g,
            start_connector: 0.5,
            end_connector: 0.5,
            advance: 1.0,
            extender,
        };
        for parts in [
            vec![part(false)],
            vec![part(true)],
            vec![part(true), part(true)],
        ] {
            let asm = crate::math::latex::font::Assembly {
                parts,
                min_overlap: 0.1,
            };
            for r in [0.5, 5.0, 1e12, f64::INFINITY] {
                let b = l.assemble(&asm, r, S).unwrap();
                b.finite().unwrap();
                assert!(b.items.len() <= 2 * MAX_EXTENDERS);
            }
        }
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
