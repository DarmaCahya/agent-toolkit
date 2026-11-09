//! Explicit LaTeX spacing commands: `\,` `\:` `\;` `\!` `\ ` `\quad` `\qquad`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use agent_canvas::math::latex::{MathError, MAX_DEPTH, MAX_INPUT_CHARS};
use agent_canvas::math::MathTypesetter;
use common::{assert_finite_numbers, assert_safe_svg, assert_well_formed, view_box};

const TOL: f64 = 0.01;

fn render(s: &str) -> Result<String, MathError> {
    MathTypesetter::new().render_to_svg(s)
}

fn svg(s: &str) -> String {
    render(s).unwrap_or_else(|e| panic!("{s:?} failed: {e}"))
}

fn width(src: &str) -> f64 {
    MathTypesetter::new()
        .layout_metrics(src)
        .unwrap_or_else(|e| panic!("{src:?} failed: {e}"))
        .width
}

fn delta(spaced: &str, plain: &str) -> f64 {
    width(spaced) - width(plain)
}

fn assert_delta(spaced: &str, plain: &str, want: f64) {
    let d = delta(spaced, plain);
    assert!(
        (d - want).abs() < TOL,
        "{spaced:?} vs {plain:?}: delta {d}, want {want}"
    );
}

fn assert_sane(svg: &str) {
    let vb = view_box(svg);
    assert!(vb.iter().all(|v| v.is_finite()), "{svg}");
    assert!(vb[2] >= 0.0 && vb[3] >= 0.0, "negative viewBox: {vb:?}");
    assert!(!svg.contains("NaN") && !svg.contains("inf"), "{svg}");
    assert_finite_numbers(svg);
}

// ---------------------------------------------------------------- US1

#[test]
fn integral_with_thin_space_renders_ok() {
    let out = svg(r"\int_a^b f(x)\,dx");
    assert!(!out.contains("<text"));
    assert_safe_svg(&out);
    assert_well_formed(&out);
}

#[test]
fn thin_space_widens_by_three_eighteenths_em() {
    assert_delta(r"f(x)\,dx", "f(x)dx", 3.0 / 18.0);
}

#[test]
fn integral_render_is_deterministic() {
    let a = svg(r"\int_a^b f(x)\,dx");
    let b = svg(r"\int_a^b f(x)\,dx");
    assert_eq!(a, b);
}

#[test]
fn integral_snapshot() {
    insta::assert_snapshot!("integral", svg(r"\int_a^b f(x)\,dx"));
}

// ---------------------------------------------------------------- US2

#[test]
fn each_space_command_has_specified_width() {
    for (spaced, want) in [
        (r"a\,b", 3.0 / 18.0),
        (r"a\:b", 4.0 / 18.0),
        (r"a\;b", 5.0 / 18.0),
        (r"a\!b", -3.0 / 18.0),
        (r"a\quad b", 1.0),
        (r"a\qquad b", 2.0),
        (r"a\ b", 1.0 / 3.0),
    ] {
        assert_delta(spaced, "ab", want);
    }
}

#[test]
fn repeated_spaces_accumulate() {
    assert_delta(r"a\,\,\,b", "ab", 9.0 / 18.0);
}

#[test]
fn quad_whitespace_after_word_command_is_ignored() {
    assert!((width(r"a\quad    b") - width(r"a\quad b")).abs() < 1e-9);
}

#[test]
fn spacing_allowed_in_every_atom_position() {
    for src in [
        r"{\,x}",
        r"x^{a\,b}",
        r"x_{a\,b}",
        r"\frac{a\,b}{c\quad d}",
        r"\sqrt{a\,b}",
        r"\sqrt[a\,b]{c}",
    ] {
        assert!(render(src).is_ok(), "{src}");
    }
}

#[test]
fn lexer_does_not_swallow_following_chars() {
    assert!(render(r"\,dx").is_ok());
    assert!(render(r"\,\alpha").is_ok());
    assert_delta(r"a\,dx", "adx", 3.0 / 18.0);
    match render(r"\quadx") {
        Err(MathError::UnknownCommand { name, .. }) => assert_eq!(name, "quadx"),
        other => panic!("expected UnknownCommand, got {other:?}"),
    }
}

// ---------------------------------------------------------------- US3

#[test]
fn spacing_is_classification_transparent() {
    let third = 3.0 / 18.0;
    assert_delta(r"\,-b", "-b", third); // minus stays unary
    assert_delta(r"a+\,b", "a+b", third);
    assert_delta(r"a\,+b", "a+b", third);
    assert_delta(r"a+\,=b", "a+=b", third); // demotion identical
    assert_delta(r"(\,+b)", "(+b)", third); // after open bracket
    assert_delta(r"a+\,", "a+", third); // trailing
}

// ---------------------------------------------------------------- US4

#[test]
fn leading_space_shifts_content() {
    assert_delta(r"\quad x", "x", 1.0);
    assert_sane(&svg(r"\quad x"));
}

#[test]
fn trailing_space_widens_canvas() {
    assert_delta(r"x\quad", "x", 1.0);
    assert_sane(&svg(r"x\quad"));
}

#[test]
fn negative_total_width_is_clamped() {
    for src in [r"x\!", r"\!x", r"\!\!\!x", r"x\!\!\!\!\!"] {
        assert_sane(&svg(src));
    }
    assert!(width(r"x\!\!\!\!\!") >= 0.0);
}

#[test]
fn spacing_only_formula_is_empty_formula_error() {
    for src in [r"\,", r"\quad", r"{\,}", r"\!"] {
        assert_eq!(render(src), Err(MathError::EmptyFormula), "{src}");
    }
}

#[test]
fn plain_empty_group_unchanged() {
    // `{}` is not spacing-only: behaviour is whatever it was before the feature.
    assert!(render("{}").is_ok());
}

#[test]
fn script_adjacent_glue_does_not_change_attachment() {
    assert!(render(r"x\,^2").is_ok());
    assert!(render(r"x^2\,").is_ok());
    assert_delta(r"x^2\,", "x^2", 3.0 / 18.0);
}

#[test]
fn mu_scales_in_script_but_em_does_not() {
    let s = MathTypesetter::new().glyph_facts('a').unwrap().script_scale;
    let thin = delta(r"x^{a\,b}", "x^{ab}");
    assert!(thin < 3.0 / 18.0, "{thin}");
    assert!((thin - 3.0 / 18.0 * s).abs() < TOL, "{thin}");
    assert_delta(r"x^{a\quad b}", "x^{ab}", 1.0);
}

#[test]
fn spacing_in_fraction_arguments_is_centred_and_sane() {
    assert_sane(&svg(r"\frac{\,a}{b}"));
}

#[test]
fn limits_and_operand_spacing_ok() {
    for src in [r"\sum\,_i", r"\int\,f", r"\sum_{i}\,x"] {
        assert_sane(&svg(src));
    }
}

#[test]
fn many_spaces_respect_limits() {
    let big = format!("{}x", r"\,".repeat(MAX_INPUT_CHARS / 2 - 1));
    assert_sane(&svg(&big));
    let too_long = format!("{}x", r"\,".repeat(MAX_INPUT_CHARS));
    assert!(matches!(
        render(&too_long),
        Err(MathError::InputTooLong { .. })
    ));
    let deep = format!(
        "{}\\,{}",
        "{".repeat(MAX_DEPTH + 1),
        "}".repeat(MAX_DEPTH + 1)
    );
    assert!(matches!(
        render(&deep),
        Err(MathError::NestingTooDeep { .. })
    ));
}

#[test]
fn trailing_backslash_space_vs_lone_backslash() {
    assert!(render("x\\ ").is_ok());
    assert!(matches!(
        render("x\\"),
        Err(MathError::UnexpectedEof { .. })
    ));
}

#[test]
fn unsupported_commands_still_error_with_new_hint() {
    for (src, name, pos) in [
        (r"\left", "left", 0),
        (r"\text", "text", 0),
        (r"\hat", "hat", 0),
    ] {
        match render(src) {
            Err(MathError::Unsupported {
                name: n, pos: p, ..
            }) => {
                assert_eq!((n.as_str(), p), (name, pos), "{src}");
            }
            other => panic!("{src}: {other:?}"),
        }
    }
    for src in [r"\@", r"\%"] {
        let e = render(src).unwrap_err();
        assert!(matches!(e, MathError::Unsupported { .. }), "{src}: {e:?}");
        let msg = e.to_string();
        assert!(
            !msg.contains("spacing and symbol commands are out of scope"),
            "{msg}"
        );
        assert!(msg.contains(r"\quad"), "{msg}");
    }
}

// ---------------------------------------------------------------- Polish

/// FR-014 / SC-005: formulas without spacing render byte-identically to the
/// committed snapshots of the earlier features.
#[test]
fn existing_formulas_render_identically() {
    let snaps = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots");
    for (file, src) in [
        ("latex_layout__scripts_xi2.snap", "x_i^2"),
        ("latex_layout__sum_limits.snap", r"\sum_{i=0}^{n} i"),
        ("latex_layout__sqrt_index.snap", r"\sqrt[3]{x}"),
        (
            "latex_quadratic__quadratic.snap",
            r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        ),
    ] {
        let raw = std::fs::read_to_string(format!("{snaps}/{file}")).unwrap();
        let body = raw.splitn(3, "---\n").nth(2).unwrap();
        assert_eq!(body.trim_end(), svg(src).trim_end(), "{file}");
    }
}

/// Regression: negative spacing pushes ink left of the origin; the viewBox must follow it.
#[test]
fn negative_leading_space_is_not_clipped() {
    let svg = MathTypesetter::new()
        .render_to_svg(r"\!a")
        .expect("renders");
    let vb = svg
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("viewBox");
    let min_x: f64 = vb
        .split(' ')
        .next()
        .and_then(|v| v.parse().ok())
        .expect("min_x");
    assert!(
        min_x < -150.0,
        "viewBox min_x {min_x} must include the glyph at x = -1/6 em"
    );
}
