//! Property tests: arbitrary input never panics, hangs or emits unsafe output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use agent_canvas::math::MathTypesetter;
use common::{assert_finite_numbers, assert_safe_svg, assert_well_formed};
use proptest::prelude::*;
use std::time::{Duration, Instant};

/// 10_000 cases by default; `PROPTEST_CASES` overrides.
fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    ProptestConfig::with_cases(cases)
}

fn token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r"\frac".to_string()),
        Just("{".to_string()),
        Just("}".to_string()),
        Just("^".to_string()),
        Just("_".to_string()),
        Just(r"\sqrt".to_string()),
        Just("[".to_string()),
        Just("]".to_string()),
        Just(r"\sum".to_string()),
        Just(r"\int".to_string()),
        Just(r"\prod".to_string()),
        Just(r"\alpha".to_string()),
        Just(r"\Omega".to_string()),
        Just(r"\pm".to_string()),
        Just(r"\infty".to_string()),
        Just(r"\left".to_string()),
        Just(r"\".to_string()),
        Just("\u{200B}".to_string()),
        Just("\u{E0041}".to_string()),
        Just("é".to_string()),
        "[a-zA-Z]",
        "[0-9]",
        "[-+=<>(), ]",
    ]
}

fn latexish() -> impl Strategy<Value = String> {
    prop::collection::vec(token(), 0..60).prop_map(|v| v.concat())
}

fn check(src: &str) {
    let start = Instant::now();
    let res = MathTypesetter::new().render_to_svg(src);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "slow input {src:?}"
    );
    if let Ok(svg) = res {
        assert_safe_svg(&svg);
        assert_well_formed(&svg);
        assert_finite_numbers(&svg);
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn arbitrary_strings_never_panic(s in any::<String>()) {
        check(&s);
    }

    #[test]
    fn nesting_accepted_iff_within_limit(depth in 0usize..200) {
        let src = format!("{}a{}", "{".repeat(depth), "}".repeat(depth));
        let res = MathTypesetter::new().render_to_svg(&src);
        prop_assert_eq!(res.is_ok(), depth <= 64);
    }
}

proptest! {
    // Successful renders are ~100x costlier than rejections (glyph outlines),
    // so the token-mix strategy runs a fifth of the cases in unoptimised builds.
    #![proptest_config(ProptestConfig::with_cases(config().cases / 5))]

    #[test]
    fn latexish_strings_never_panic(s in latexish()) {
        check(&s);
    }
}

#[test]
fn worst_case_inputs_finish_quickly() {
    let start = Instant::now();
    // 64 nested groups, then script chains.
    let mut a = "{".repeat(64);
    a.push_str(&"a^".repeat((8192 - 128) / 2));
    check(&a);
    // Long fraction run.
    check(&r"\frac{a}{b}".repeat(8192 / 11));
    // Deep script chain within the limit.
    check(&nested_scripts(64));
    // Maximum-length mix of big operators with scripts.
    check(&r"\sum_{i}^{n}".repeat(8192 / 12));
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
}

fn nested_scripts(n: usize) -> String {
    format!("{}a{}", "a^{".repeat(n), "}".repeat(n))
}
