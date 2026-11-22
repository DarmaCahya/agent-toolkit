//! `\left ... \right` auto-sizing delimiters (spec 005).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use agent_canvas::math::latex::{BoxMetrics, GlyphFacts, ItemKind, MathError, MAX_DEPTH};
use agent_canvas::math::MathTypesetter;
use common::{assert_safe_svg, assert_well_formed, view_box};

fn metrics(src: &str) -> BoxMetrics {
    MathTypesetter::new()
        .layout_metrics(src)
        .unwrap_or_else(|e| panic!("{src:?} failed: {e}"))
}

fn render(src: &str) -> String {
    MathTypesetter::new()
        .render_to_svg(src)
        .unwrap_or_else(|e| panic!("{src:?} failed: {e}"))
}

fn facts(c: char) -> GlyphFacts {
    MathTypesetter::new().glyph_facts(c).unwrap()
}

fn glyphs(b: &BoxMetrics) -> Vec<&agent_canvas::math::latex::ItemMetrics> {
    b.items
        .iter()
        .filter(|i| i.kind != ItemKind::Rule)
        .collect()
}

/// Left delimiter glyph (first item) and right one (last item).
fn delimiters(
    b: &BoxMetrics,
) -> (
    &agent_canvas::math::latex::ItemMetrics,
    &agent_canvas::math::latex::ItemMetrics,
) {
    let g = glyphs(b);
    (g[0], g[g.len() - 1])
}

// ---------------------------------------------------------------- safety

#[test]
fn renders_vector_only_safe_svg() {
    for src in [
        r"\left( x \right)",
        r"\left[ \frac{a}{b} \right]",
        r"\left\{ x \right\}",
        r"\left| x \right|",
        r"\left\| x \right\|",
        r"\left\langle x \right\rangle",
        r"\left. \frac{a}{b} \right|",
        r"\left( \sum_{i=0}^{n} i \right)",
        r"\left( \left[ \frac{a}{b} \right] \right)",
        r"\left( \frac{a}{b} \right)^2",
        r"\left( \right)",
        // Mismatched delimiters are legal, as in TeX.
        r"\left( x \right]",
        // Two Inner atoms next to each other.
        r"\left( a \right)\left[ b \right]",
        // A delimiter group inside a script.
        r"x^{\left( \frac{a}{b} \right)}",
    ] {
        let svg = render(src);
        assert!(svg.starts_with("<svg"), "{src}");
        assert!(!svg.contains("<text"), "{src}");
        assert!(!svg.contains("font-family"), "{src}");
        assert_well_formed(&svg);
        assert_safe_svg(&svg);
        let vb = view_box(&svg);
        assert!(vb.iter().all(|v| v.is_finite()), "{src}: {vb:?}");
        assert!(vb[2] > 0.0 && vb[3] > 0.0, "{src}: empty viewBox {vb:?}");
    }
}

#[test]
fn ten_renders_are_byte_identical() {
    for src in [
        r"\left( \frac{a}{b} \right)",
        r"\left( \left[ \frac{a}{b} \right] \right)^2",
    ] {
        let first = render(src);
        for i in 0..10 {
            assert_eq!(render(src), first, "{src}: render {i} differs");
        }
    }
}

// ---------------------------------------------------------------- sizing

#[test]
fn tall_fraction_grows_the_delimiters_past_the_content() {
    let content = metrics(r"\frac{a}{b}");
    let xheight = metrics("x");
    let wrapped = metrics(r"\left( \frac{a}{b} \right)");
    let (left, right) = delimiters(&wrapped);
    let base = facts('(');

    // Both delimiters are the same, taller-than-base paren variant.
    assert!(
        (left.w - right.w).abs() < 1e-9,
        "delimiters must match in size: {} vs {}",
        left.w,
        right.w
    );
    assert_ne!(left.gid, base.gid, "a tall frac needs a larger variant");
    assert_ne!(right.gid, facts(')').gid);
    assert_eq!(left.sx, right.sx);
    assert_eq!(left.sy, right.sy);

    // The construct is strictly taller and deeper than the content box and
    // much taller than an x-height glyph.
    assert!(
        wrapped.height > content.height && wrapped.depth > content.depth,
        "delimiters must extend beyond the content: {:?} vs {:?}",
        (wrapped.height, wrapped.depth),
        (content.height, content.depth)
    );
    assert!(
        wrapped.height + wrapped.depth > content.height + content.depth + 0.1,
        "delimiters should add real height (variants are quantised): {} vs {}",
        wrapped.height + wrapped.depth,
        content.height + content.depth
    );
    assert!(wrapped.height > 2.0 * xheight.height);
    assert!(wrapped.width > content.width, "delimiters add width");
}

#[test]
fn delimiters_are_centred_on_the_math_axis() {
    let axis = facts('x').axis_height;
    assert!(axis > 0.0);
    // Where a delimiter is the tallest element, centring its ink on the axis
    // makes height - depth exactly twice the axis height.
    for src in [
        r"\left( \frac{a}{b} \right)",
        r"\left( \sum_{i=0}^{n} i \right)",
        r"\left[ \frac{a}{b} \right]",
        r"\left( \frac{\frac{a}{b}}{c} \right)",
    ] {
        let b = metrics(src);
        let (left, right) = delimiters(&b);
        // Both delimiters are pushed down by the same baseline offset.
        assert!(
            (left.y - right.y).abs() < 1e-9,
            "{src}: delimiter baselines differ: {} vs {}",
            left.y,
            right.y
        );
        assert!(
            (b.height - b.depth - 2.0 * axis).abs() < 1e-6,
            "{src}: not centred on the axis: h={} d={} axis={axis}",
            b.height,
            b.depth
        );
    }
}

#[test]
fn small_content_keeps_the_natural_delimiter() {
    let wrapped = metrics(r"\left( x \right)");
    let (left, right) = delimiters(&wrapped);
    let base = facts('(');
    assert_eq!(left.gid, base.gid, "short content keeps the base glyph");
    assert_eq!(right.gid, facts(')').gid);
    // No extra glue: width is exactly the two advances plus the content.
    let x = metrics("x");
    assert!(
        (wrapped.width - (2.0 * base.advance + x.width)).abs() < 1e-9,
        "unexpected padding: {} vs {}",
        wrapped.width,
        2.0 * base.advance + x.width
    );
}

#[test]
fn nested_delimiters_wrap_outside_in() {
    let inner = metrics(r"\left[ \frac{a}{b} \right]");
    let outer = metrics(r"\left( \left[ \frac{a}{b} \right] \right)");
    let base_paren = facts('(');
    let base_bracket = facts('[');
    let g = glyphs(&outer);
    // Items: outer (, inner [, a, b, inner ], outer ).
    assert_eq!(g.len(), 6, "expected four delimiters plus the fraction");
    assert_ne!(g[0].gid, base_paren.gid, "outer paren must grow");
    assert_ne!(g[5].gid, facts(')').gid);
    assert_ne!(g[1].gid, base_bracket.gid, "inner bracket must grow too");
    assert_ne!(g[4].gid, facts(']').gid);
    // Horizontal order: ( [ a b ] )
    for w in g.windows(2) {
        assert!(w[0].x <= w[1].x, "delimiters must be ordered outside in");
    }
    assert!(
        g[1].x > g[0].x + 0.3,
        "inner delimiter inside the outer one"
    );
    // Nesting never shrinks the outer box.
    assert!(outer.height >= inner.height && outer.depth >= inner.depth);
    assert!(outer.height + outer.depth >= inner.height + inner.depth);
    assert!(outer.width > inner.width + base_paren.advance);
    assert_safe_svg(&render(r"\left( \left[ \frac{a}{b} \right] \right)"));
}

#[test]
fn null_delimiter_draws_nothing_but_still_delimits() {
    // `\left. x \right.` is exactly `x`.
    let plain = metrics("x");
    let nulls = metrics(r"\left. x \right.");
    assert_eq!(plain, nulls, "null delimiters must not change the layout");

    // One real delimiter only.
    let b = metrics(r"\left. x \right|");
    let bar = facts('|');
    assert_eq!(b.items.len(), 2, "expected x and one bar");
    assert_eq!(b.items[1].gid, bar.gid);
    assert!(b.items[1].x >= b.items[0].x + b.items[0].w - 1e-9);
    assert_safe_svg(&render(r"\left. \frac{a}{b} \right|"));
}

#[test]
fn delimiters_grow_around_a_display_sum() {
    let operator_only = metrics(r"\sum_{i=0}^{n} i");
    let wrapped = metrics(r"\left( \sum_{i=0}^{n} i \right)");
    let (left, right) = delimiters(&wrapped);
    let base = facts('(');
    assert!(
        (left.w - right.w).abs() < 1e-9,
        "delimiters must match in size"
    );
    assert_ne!(
        left.gid, base.gid,
        "the delimiter must outgrow the base paren"
    );
    assert!(
        wrapped.height >= operator_only.height && wrapped.depth >= operator_only.depth,
        "delimiters must cover the display operator and its limits"
    );
    assert!(
        wrapped.height > 1.4,
        "sum limits are tall: {}",
        wrapped.height
    );
    assert!(wrapped.width > operator_only.width);
}

// ---------------------------------------------------------------- semantics

#[test]
fn group_takes_scripts_like_a_normal_nucleus() {
    let base = metrics(r"\left( \frac{a}{b} \right)");
    let squared = metrics(r"\left( \frac{a}{b} \right)^2");
    let g = glyphs(&squared);
    let sup = g[g.len() - 1];
    let base_scale = g[0].sx;
    assert!(
        sup.y > 0.5 * base.height,
        "superscript raised above the group's centre: {} vs {}",
        sup.y,
        base.height
    );
    assert!(
        (sup.sx / base_scale - facts('2').script_scale).abs() < 1e-9,
        "superscript uses the script size"
    );
    assert!(squared.width > base.width);
    assert!(squared.height > base.height);
}

#[test]
fn inner_atom_spacing_matches_the_tex_table() {
    let mu = 1.0 / 18.0;
    let inner = metrics(r"\left( b \right)");
    let a = metrics("a");
    let rel = metrics("=");
    let bin = metrics("-");

    // Ord Inner = thin (3mu), Rel Inner = thick (5mu), Bin Inner = medium (4mu).
    let ord_inner = metrics(r"a \left( b \right)");
    assert!(
        (ord_inner.width - (a.width + 3.0 * mu + inner.width)).abs() < 1e-9,
        "Ord-Inner should be 3mu: {} vs {}",
        ord_inner.width,
        a.width + 3.0 * mu + inner.width
    );
    let rel_inner = metrics(r"= \left( b \right)");
    assert!(
        (rel_inner.width - (rel.width + 5.0 * mu + inner.width)).abs() < 1e-9,
        "Rel-Inner should be 5mu"
    );
    let bin_inner = metrics(r"a - \left( b \right)");
    assert!(
        (bin_inner.width - (a.width + bin.width + 2.0 * 4.0 * mu + inner.width)).abs() < 1e-9,
        "Bin-Inner should be 4mu: {} vs {}",
        bin_inner.width,
        a.width + bin.width + 2.0 * 4.0 * mu + inner.width
    );
    // A leading `-` is demoted to Ord, so only a thin space follows it.
    let leading_bin = metrics(r"- \left( b \right)");
    assert!(
        (leading_bin.width - (bin.width + 3.0 * mu + inner.width)).abs() < 1e-9,
        "demoted leading Bin-Inner should be 3mu"
    );

    // Inner Ord is thin as well (the table's Inner row).
    let inner_ord = metrics(r"\left( b \right) a");
    assert!(
        (inner_ord.width - (inner.width + 3.0 * mu + a.width)).abs() < 1e-9,
        "Inner-Ord should be 3mu"
    );
}

// ---------------------------------------------------------------- errors

fn err(src: &str) -> MathError {
    MathTypesetter::new()
        .render_to_svg(src)
        .expect_err(&format!("{src:?} should fail"))
}

#[test]
fn unmatched_and_invalid_delimiters_are_located_errors() {
    let e = err(r"\left( x");
    assert_eq!(e, MathError::MissingRight { pos: 0 });
    assert_eq!(e.position(), Some(0));
    assert!(e.to_string().contains("hint"));

    let e = err(r"x + \left( y");
    assert_eq!(e, MathError::MissingRight { pos: 4 }, "{e:?}");

    // A `}` closes the enclosing group, not the `\left`.
    let e = err(r"{\left( x}");
    assert_eq!(e, MathError::MissingRight { pos: 1 }, "{e:?}");

    let e = err(r"\right)");
    assert_eq!(e, MathError::UnexpectedRight { pos: 0 });
    assert_eq!(e.position(), Some(0));

    let e = err(r"x^2 \right]");
    assert_eq!(e, MathError::UnexpectedRight { pos: 4 }, "{e:?}");

    // Unknown / missing delimiters.
    let e = err(r"\left x \right)");
    assert_eq!(
        e,
        MathError::InvalidDelimiter {
            pos: 6,
            command: "left",
            found: "x".into()
        }
    );
    assert_eq!(e.position(), Some(6));

    let e = err(r"\left\foo x \right)");
    assert_eq!(
        e,
        MathError::InvalidDelimiter {
            pos: 5,
            command: "left",
            found: "\\foo".into()
        }
    );
    assert!(e.to_string().contains("hint"));

    let e = err(r"\left( x \right+");
    assert_eq!(
        e,
        MathError::InvalidDelimiter {
            pos: 15,
            command: "right",
            found: "+".into()
        }
    );

    // `\left` with no delimiter at all.
    let e = err(r"\left");
    assert_eq!(
        e,
        MathError::InvalidDelimiter {
            pos: 5,
            command: "left",
            found: "end of input".into()
        }
    );
}

#[test]
fn nesting_depth_is_bounded_for_delimiters() {
    let ok = format!(
        "{}a{}",
        r"\left(".repeat(MAX_DEPTH),
        r"\right)".repeat(MAX_DEPTH)
    );
    MathTypesetter::new()
        .render_to_svg(&ok)
        .expect("64 nested delimiters are accepted");
    let bad = format!(
        "{}a{}",
        r"\left(".repeat(MAX_DEPTH + 1),
        r"\right)".repeat(MAX_DEPTH + 1)
    );
    let e = err(&bad);
    assert!(
        matches!(e, MathError::NestingTooDeep { max: MAX_DEPTH, .. }),
        "{e:?}"
    );
    assert!(e.position().is_some());
}

#[test]
fn hostile_input_never_panics() {
    for src in [
        r"\left",
        r"\right",
        r"\left\right)",
        r"\left.\right.",
        r"\left[",
        r"\left(",
        "\\left(",
        r"\left) x \right(",
        r"\left\",
        r"\left^2",
        r"\left{ x \right}",
        r"\left| x \right| \right|",
    ] {
        // Must be a clean Result (Err or Ok), never a panic.
        let _ = MathTypesetter::new().render_to_svg(src);
    }
}

// ---------------------------------------------------------------- snapshots

#[test]
fn left_right_snapshots() {
    insta::assert_snapshot!("left_right_frac", render(r"\left( \frac{a}{b} \right)^2"));
    insta::assert_snapshot!(
        "left_right_nested",
        render(r"\left( \left[ \frac{a}{b} \right] \right)")
    );
}
