//! User story 1: render the quadratic formula to a self-contained SVG.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use agent_canvas::math::MathTypesetter;
use common::{assert_safe_svg, assert_well_formed, use_xs, view_box};

const QUADRATIC: &str = r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}";

fn render(s: &str) -> String {
    MathTypesetter::new().render_to_svg(s).unwrap()
}

#[test]
fn quadratic_is_well_formed_svg() {
    let svg = render(QUADRATIC);
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("viewBox=\""));
    assert!(svg.contains("fill=\"currentColor\""));
    assert_well_formed(&svg);
}

#[test]
fn quadratic_is_vector_only_and_safe() {
    assert_safe_svg(&render(QUADRATIC));
}

#[test]
fn renders_are_byte_identical() {
    let first = render(QUADRATIC);
    for _ in 0..100 {
        assert_eq!(render(QUADRATIC), first);
    }
}

#[test]
fn view_box_is_tight_around_formula() {
    let t = MathTypesetter::new();
    let m = t.layout_metrics(QUADRATIC).unwrap();
    let svg = t.render_to_svg(QUADRATIC).unwrap();
    let [x, y, w, h] = view_box(&svg);
    let tol = 0.05;
    assert!((x + 20.0).abs() < tol);
    assert!((y + m.height * 1000.0 + 20.0).abs() < tol, "{y}");
    assert!((w - (m.width * 1000.0 + 40.0)).abs() < tol, "{w}");
    assert!(
        (h - ((m.height + m.depth) * 1000.0 + 40.0)).abs() < tol,
        "{h}"
    );
}

#[test]
fn unary_minus_is_tighter_than_binary_minus() {
    // `-b`: minus then b with no space. `a-b`: a, minus, b with medium spaces.
    let unary = use_xs(&render("-b"));
    let binary = use_xs(&render("a-b"));
    assert_eq!(unary.len(), 2);
    assert_eq!(binary.len(), 3);
    let gap_unary = unary[1] - unary[0];
    let gap_binary = binary[2] - binary[1];
    assert!(
        gap_unary < gap_binary,
        "unary gap {gap_unary} should be < binary gap {gap_binary}"
    );
}

#[test]
fn repeated_glyphs_share_one_path() {
    let svg = render("aaaa");
    assert_eq!(svg.matches("<path").count(), 1);
    assert_eq!(svg.matches("<use").count(), 4);
}

#[test]
fn quadratic_snapshot() {
    insta::assert_snapshot!("quadratic", render(QUADRATIC));
}
