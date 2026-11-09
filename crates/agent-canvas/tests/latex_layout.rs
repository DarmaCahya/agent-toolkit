//! User story 2: TeX-quality layout of scripts, fractions, roots and operators.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use agent_canvas::math::latex::{BoxMetrics, ItemKind, ItemMetrics};
use agent_canvas::math::MathTypesetter;

fn m(s: &str) -> BoxMetrics {
    MathTypesetter::new().layout_metrics(s).unwrap()
}

fn glyphs(b: &BoxMetrics) -> Vec<ItemMetrics> {
    b.items
        .iter()
        .copied()
        .filter(|i| i.kind != ItemKind::Rule)
        .collect()
}

fn rules(b: &BoxMetrics) -> Vec<ItemMetrics> {
    b.items
        .iter()
        .copied()
        .filter(|i| i.kind == ItemKind::Rule)
        .collect()
}

fn facts(c: char) -> agent_canvas::math::latex::GlyphFacts {
    MathTypesetter::new().glyph_facts(c).unwrap()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn scripts_use_font_scale_and_offsets() {
    let f = facts('x');
    // (a) superscript
    let g = glyphs(&m("x^2"));
    assert_eq!(g.len(), 2);
    assert!(close(
        g[1].sx / g[0].sx,
        f.script_scale,
        0.01 * f.script_scale
    ));
    assert!(g[1].y > g[0].y, "superscript raised");
    assert!(
        close(g[1].x, g[0].w + f.italic_correction, 1e-9),
        "superscript offset by italic correction: {} vs {}",
        g[1].x,
        g[0].w + f.italic_correction
    );
    // subscript
    let g = glyphs(&m("x_i"));
    assert!(g[1].y < g[0].y, "subscript lowered");
    assert!(
        close(g[1].x, g[0].w, 1e-9),
        "no italic correction on subscripts"
    );
    // both
    let g = glyphs(&m("x_i^2"));
    assert_eq!(g.len(), 3);
    // Items are in placement order: nucleus, superscript, subscript.
    let (sup, sub) = (g[1], g[2]);
    assert!(sup.y > g[0].y && sub.y < g[0].y);
    assert!(close(
        sup.sx / g[0].sx,
        f.script_scale,
        0.01 * f.script_scale
    ));
    // sup/sub keep a minimum gap: superscript above subscript
    assert!(sup.y - sub.y > 0.3);
}

#[test]
fn test_math_nested_fractions() {
    let f = facts('a');
    let ax = f.axis_height;
    // Numerator list = [inner fraction, x]: both share the numerator baseline.
    let b = m(r"\frac{\frac{a}{b}x}{c}");
    let bars = rules(&b);
    assert_eq!(bars.len(), 2);
    let outer = bars.iter().max_by(|p, q| p.w.total_cmp(&q.w)).unwrap();
    let inner = bars.iter().min_by(|p, q| p.w.total_cmp(&q.w)).unwrap();
    // Outer bar sits on the outer axis (baseline y = 0), within 1% of an em.
    assert!(close(outer.y + outer.h / 2.0, ax, 0.01));
    // `x` is the last glyph of the numerator: its baseline is the numerator's.
    let x_item = glyphs(&b)
        .into_iter()
        .find(|g| g.gid == facts('x').gid)
        .unwrap();
    // Inner fraction is in text style (scale 1): its bar is one axis above that baseline.
    assert!(close(inner.y + inner.h / 2.0 - x_item.y, ax, 0.01));
    assert!(inner.w < outer.w);
    // Numerator content is centred on the outer bar.
    let g = glyphs(&b);
    let bar_centre = outer.x + outer.w / 2.0;
    let c_item = g.iter().find(|i| i.gid == facts('c').gid).unwrap();
    assert!(close(c_item.x + c_item.w / 2.0, bar_centre, 0.001));
    // Inner numerator/denominator are smaller than the outer denominator `c`.
    let a_item = g.iter().find(|i| i.gid == f.gid).unwrap();
    assert!(a_item.sx < c_item.sx);
    // Inner bar is centred over inner content.
    let inner_centre = inner.x + inner.w / 2.0;
    assert!(close(a_item.x + a_item.w / 2.0, inner_centre, 0.001));
    let b_item = g.iter().find(|i| i.gid == facts('b').gid).unwrap();
    assert!(close(b_item.x + b_item.w / 2.0, inner_centre, 0.001));
    // Three levels: the innermost fraction is in script style; its axis scales.
    let b3 = m(r"\frac{\frac{\frac{a}{b}y}{c}x}{d}");
    let bars3 = rules(&b3);
    assert_eq!(bars3.len(), 3);
    let y_item = glyphs(&b3)
        .into_iter()
        .find(|g| g.gid == facts('y').gid)
        .unwrap();
    let deepest = bars3.iter().min_by(|p, q| p.w.total_cmp(&q.w)).unwrap();
    let scale_s = f.script_scale;
    assert!(
        close(deepest.y + deepest.h / 2.0 - y_item.y, ax * scale_s, 0.01),
        "deepest bar on the script-style axis"
    );
}

#[test]
fn test_math_sqrt_stretch() {
    let f = facts('x');
    let plain = m(r"\sqrt{x}");
    let tall = m(r"\sqrt{\frac{a}{b}}");
    let content = m(r"\frac{a}{b}");
    assert!(tall.height + tall.depth > plain.height + plain.depth);
    assert!(
        tall.height + tall.depth + 1e-9
            >= content.height
                + content.depth
                + f.radical_display_vertical_gap
                + f.radical_rule_thickness
    );
    // Top rule covers the content width.
    let top = rules(&tall)
        .into_iter()
        .find(|r| r.h < 0.1 && r.w > 0.3)
        .unwrap();
    assert!(close(top.x + top.w, tall.width, 1e-9));
    let first_content_x = glyphs(&tall)
        .into_iter()
        .skip(1)
        .map(|g| g.x)
        .fold(f64::INFINITY, f64::min);
    assert!(top.x <= first_content_x);
}

#[test]
fn root_index_is_scriptscript_left_and_raised() {
    let f = facts('x');
    let b = m(r"\sqrt[3]{x}");
    let g = glyphs(&b);
    let root = g[0];
    let index = g
        .iter()
        .find(|i| close(i.sx, f.script_script_scale, 0.01 * f.script_script_scale))
        .copied()
        .unwrap();
    assert!(index.x < root.x + root.w, "index left of the radical body");
    assert!(index.y > 0.0, "index raised above the baseline");
    let body = g.iter().find(|i| i.gid == f.gid).unwrap();
    assert!(index.x < body.x);
}

#[test]
fn display_limits_are_centred_and_script_limits_go_to_the_side() {
    let b = m(r"\sum_{i=0}^{n} i");
    let g = glyphs(&b);
    let sum = g[0];
    let upper = g[1];
    let (l0, l_last) = (g[2], g[4]);
    assert!(upper.y > 0.0 && l0.y < 0.0);
    let sum_centre = sum.x + sum.w / 2.0;
    assert!(close(upper.x + upper.w / 2.0, sum_centre, 0.01));
    let lower_centre = (l0.x + l_last.x + l_last.w) / 2.0;
    assert!(close(lower_centre, sum_centre, 0.01));

    // Nested in a script: limits become side scripts.
    let g = glyphs(&m(r"x^{\sum_i}"));
    let (sum, sub) = (g[1], g[2]);
    assert!(sub.x >= sum.x + sum.w - 1e-9, "side subscript");
}

#[test]
fn integral_keeps_side_scripts_in_display_style() {
    let g = glyphs(&m(r"\int_a^b"));
    let (int, b, a) = (g[0], g[1], g[2]); // nucleus, superscript, subscript
    assert!(
        close(b.x, int.x + int.w, 1e-9),
        "superscript at the right edge"
    );
    assert!(
        a.x > int.x + 0.3 * int.w,
        "subscript beside, not centred below"
    );
    assert!(b.y > 0.0 && a.y < 0.0);
}

#[test]
fn very_tall_radicand_is_stretched_not_clipped() {
    let formula = format!(r"\sqrt{{{}a{}}}", r"\frac{".repeat(12), r"}{b}".repeat(12));
    let t = MathTypesetter::new();
    let b = t.layout_metrics(&formula).unwrap();
    let stretched = b
        .items
        .iter()
        .find(|i| i.kind == ItemKind::ScaledGlyph)
        .expect("radical drawn with a vertical stretch");
    assert!(stretched.sy > stretched.sx);
    let content = t
        .layout_metrics(&formula[r"\sqrt{".len()..formula.len() - 1])
        .unwrap();
    assert!(b.height >= content.height && b.depth >= content.depth);
    let svg = t.render_to_svg(&formula).unwrap();
    assert!(
        svg.lines().any(|l| {
            l.split("scale(")
                .nth(1)
                .is_some_and(|s| s.split(')').next().unwrap().contains(' '))
        }),
        "non-uniform scale in SVG"
    );
}

/// Golden extents for the quadratic formula, in em.
///
/// Recorded from the first approved render (engine output rasterised with
/// resvg and inspected by eye). No independent TeX reference render was
/// available, so this guards against regressions rather than proving
/// agreement with TeX (SC-001 tolerance: 3% of formula height).
#[test]
fn quadratic_golden_extents() {
    let b = m(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}");
    let total = b.height + b.depth;
    let tol = 0.03 * total;
    assert!(close(b.width, 6.974, tol), "width {}", b.width);
    assert!(close(b.height, 1.7345, tol), "height {}", b.height);
    assert!(close(b.depth, 0.652, tol), "depth {}", b.depth);
}

#[test]
fn layout_snapshots() {
    let svg = |s: &str| MathTypesetter::new().render_to_svg(s).unwrap();
    insta::assert_snapshot!("scripts_xi2", svg("x_i^2"));
    insta::assert_snapshot!("sum_limits", svg(r"\sum_{i=0}^{n} i"));
    insta::assert_snapshot!("sqrt_index", svg(r"\sqrt[3]{x}"));
}
