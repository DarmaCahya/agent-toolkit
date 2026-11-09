//! User story 3: positioned errors and safe behaviour on hostile input.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use agent_canvas::math::{MathError, MathTypesetter};
use std::time::Instant;

fn err(s: &str) -> MathError {
    MathTypesetter::new().render_to_svg(s).unwrap_err()
}

/// Asserts the message is non-empty and mentions `needle` (SC-007).
fn assert_message(e: &MathError, needle: &str) {
    let msg = e.to_string();
    assert!(!msg.is_empty());
    assert!(msg.contains(needle), "{msg:?} should contain {needle:?}");
    assert!(msg.contains("hint"), "{msg:?} should carry a hint");
}

#[test]
fn empty_and_blank_formulas() {
    for s in ["", "   ", "\n\t "] {
        let e = err(s);
        assert_eq!(e, MathError::EmptyFormula, "{s:?}");
        assert_eq!(e.position(), None);
        assert_message(&e, "empty");
    }
}

#[test]
fn eof_inside_a_group() {
    let e = err(r"\frac{a}{");
    assert!(
        matches!(e, MathError::UnexpectedEof { pos: 9, .. }),
        "{e:?}"
    );
    assert_eq!(e.position(), Some(9));
    assert_message(&e, "9");
}

#[test]
fn unknown_command() {
    let e = err(r"\foo");
    assert_eq!(
        e,
        MathError::UnknownCommand {
            pos: 0,
            name: "foo".into()
        }
    );
    assert_message(&e, "\\foo");
}

#[test]
fn unsupported_commands_are_named() {
    for (src, name, pos) in [
        (r"\left( x \right)", "left", 0),
        (r"\text{hi}", "text", 0),
        (r"\begin{matrix}", "begin", 0),
        (r"\hat{x}", "hat", 0),
        (r"\newcommand", "newcommand", 0),
        (r"\mathbf{x}", "mathbf", 0),
        (r"a\@b", "@", 1),
        (r"x + \displaystyle y", "displaystyle", 4),
    ] {
        let e = err(src);
        match &e {
            MathError::Unsupported {
                pos: p, name: n, ..
            } => {
                assert_eq!((n.as_str(), *p), (name, pos), "{src}");
            }
            other => panic!("{src}: expected Unsupported, got {other:?}"),
        }
        assert_eq!(e.position(), Some(pos));
        assert_message(&e, name);
    }
}

#[test]
fn double_scripts() {
    for (src, pos, kind) in [("x^2^3", 3, "superscript"), ("x_1_2", 3, "subscript")] {
        let e = err(src);
        assert_eq!(e, MathError::DoubleScript { pos, kind }, "{src}");
        assert_eq!(e.position(), Some(pos));
        assert_message(&e, kind);
    }
}

#[test]
fn missing_arguments() {
    let e = err(r"\frac{a}");
    assert_eq!(
        e,
        MathError::MissingArgument {
            pos: 0,
            command: "frac".into()
        }
    );
    assert_message(&e, "frac");
    let e = err(r"\sqrt");
    assert!(
        matches!(e, MathError::MissingArgument { pos: 0, .. }),
        "{e:?}"
    );
    assert_message(&e, "sqrt");
    let e = err("x^");
    assert!(
        matches!(e, MathError::UnexpectedEof { pos: 2, .. }),
        "{e:?}"
    );
    assert_message(&e, "2");
}

#[test]
fn unexpected_characters_report_char_positions() {
    for (src, pos, ch) in [
        ("$x$", 0, '$'),
        ("x%", 1, '%'),
        ("é", 0, 'é'),
        ("a b%", 3, '%'),
        ("a\u{200B}b", 1, '\u{200B}'),
        ("a\u{E0041}", 1, '\u{E0041}'),
        ("ab日", 2, '日'),
    ] {
        let e = err(src);
        assert_eq!(e, MathError::UnexpectedChar { pos, ch }, "{src:?}");
        assert_eq!(e.position(), Some(pos));
        assert_message(&e, &format!("{ch:?}"));
    }
}

#[test]
fn input_length_is_checked_before_lexing() {
    // Would be an UnexpectedChar at position 0 if it were lexed.
    let too_long = "é".repeat(8193);
    let e = err(&too_long);
    assert!(
        matches!(e, MathError::InputTooLong { max: 8192, .. }),
        "{e:?}"
    );
    assert_eq!(e.position(), None);
    assert_message(&e, "8192");
    // Exactly the limit is accepted (or fails for another reason).
    let ok = "a".repeat(8192);
    let res = MathTypesetter::new().render_to_svg(&ok);
    assert!(!matches!(res, Err(MathError::InputTooLong { .. })));
    assert!(res.is_ok());
}

fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
}

#[test]
fn nesting_limit_is_64() {
    let t = MathTypesetter::new();
    let cases = [
        ("braces", "{", "a", "}"),
        ("frac numerators", r"\frac{", "a", "}{b}"),
        ("sqrt bodies", r"\sqrt{", "a", "}"),
        ("script chains", "a^{", "b", "}"),
        ("root indices", r"\sqrt[", "a", r"]{b}"),
    ];
    for (name, open, core, close) in cases {
        let ok = nested(open, core, close, 64);
        assert!(t.render_to_svg(&ok).is_ok(), "{name}: 64 levels accepted");
        let bad = nested(open, core, close, 65);
        let e = t.render_to_svg(&bad).unwrap_err();
        assert!(
            matches!(e, MathError::NestingTooDeep { max: 64, .. }),
            "{name}: {e:?}"
        );
        assert!(e.position().is_some());
        assert_message(&e, "64");
    }
}

#[test]
fn realistic_formula_is_fast() {
    // Release target: < 50 ms. Debug builds are allowed 10x that.
    let f = r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a} + \sum_{i=0}^{n} x_i^2 - \int_a^b f(x), \alpha \leq \beta";
    assert!(f.chars().count() <= 200);
    let start = Instant::now();
    MathTypesetter::new().render_to_svg(f).unwrap();
    assert!(start.elapsed().as_millis() < 500, "{:?}", start.elapsed());
}

#[test]
fn large_input_has_bounded_output() {
    let unit = r"\frac{a}{b}";
    let src = unit.repeat(8192 / unit.len());
    assert!(src.chars().count() <= 8192);
    let start = Instant::now();
    let svg = MathTypesetter::new().render_to_svg(&src).unwrap();
    assert!(svg.len() < 8 * 1024 * 1024, "{} bytes", svg.len());
    assert!(start.elapsed().as_secs() < 2, "{:?}", start.elapsed());
}
