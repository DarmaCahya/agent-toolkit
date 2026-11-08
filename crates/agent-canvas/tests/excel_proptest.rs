// Relaxed lints are allowed in tests: failing loudly is the point.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)] // test code

use agent_canvas::math::{Cell, ExcelError, ExcelEvaluator, RangeProvider, Value};
use proptest::prelude::*;

/// 10_000 cases by default; `PROPTEST_CASES=100000` runs the full SC-004 fuzz.
fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    ProptestConfig::with_cases(cases)
}

/// Resolves every valid-looking reference to a small mixed range.
struct Mixed;

impl RangeProvider for Mixed {
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>> {
        if reference == "A1:B3" {
            return None;
        }
        Some(vec![
            Cell::Value(Value::Number(2.0)),
            Cell::Empty,
            Cell::Value(Value::Text("3".into())),
            Cell::Value(Value::Bool(true)),
            Cell::Value(Value::Error(ExcelError::Num)),
        ])
    }
}

const TOKENS: &[&str] = &[
    "1",
    "2.5",
    ".5",
    "1E3",
    "0",
    "1E308",
    "\"a\"",
    "\"\"",
    "\"x\"\"y\"",
    "TRUE",
    "false",
    "+",
    "-",
    "*",
    "/",
    "^",
    "&",
    "=",
    "<>",
    "<",
    ">",
    "<=",
    ">=",
    "%",
    "(",
    ")",
    ",",
    " ",
    "A1",
    "A1:B3",
    "$A$1",
    "Sheet1!A1",
    "SUM(",
    "AVERAGE(",
    "MIN(",
    "MAX(",
    "COUNT(",
    "IF(",
    "AND(",
    "OR(",
    "NOT(",
    "ABS(",
    "ROUND(",
    "IFERROR(",
    "CONCAT(",
    "NOW(",
    "sum(",
    "\u{200B}",
    "\u{E0041}",
    "é",
    "#",
    "'",
    "!",
];

fn structured() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(TOKENS), 0..80).prop_map(|t| t.concat())
}

proptest! {
    #![proptest_config(config())]

    // (a) Any text at all returns a value and never panics.
    #[test]
    fn arbitrary_text_never_panics(s in any::<String>()) {
        let _ = ExcelEvaluator::eval(&s);
        let _ = ExcelEvaluator::eval_with(&s, &Mixed);
    }

    // (b) Grammar-shaped soup exercises the parser and evaluator much deeper.
    #[test]
    fn structured_input_never_panics(s in structured()) {
        let _ = ExcelEvaluator::eval(&s);
        let _ = ExcelEvaluator::eval_with(&s, &Mixed);
        let _ = ExcelEvaluator::eval(&format!("={s}"));
    }

    // (c) Length boundary: 2 + 2k chars is accepted up to 8192 and rejected beyond.
    #[test]
    fn length_boundary_holds(k in 4090usize..4105) {
        let f = format!("=1{}", "+0".repeat(k));
        let want = if f.chars().count() <= 8192 {
            Value::Number(1.0)
        } else {
            Value::Error(ExcelError::Value)
        };
        prop_assert_eq!(ExcelEvaluator::eval(&f), want);
    }

    // (c) Depth boundary: up to 64 nested parentheses, errors beyond.
    #[test]
    fn depth_boundary_holds(n in 40usize..90) {
        let f = format!("={}1{}", "(".repeat(n), ")".repeat(n));
        let want = if n <= 64 { Value::Number(1.0) } else { Value::Error(ExcelError::Value) };
        prop_assert_eq!(ExcelEvaluator::eval(&f), want);
    }

    // (d) Same input, same output.
    #[test]
    fn evaluation_is_idempotent(s in structured()) {
        prop_assert_eq!(ExcelEvaluator::eval(&s), ExcelEvaluator::eval(&s));
        prop_assert_eq!(
            ExcelEvaluator::eval_with(&s, &Mixed),
            ExcelEvaluator::eval_with(&s, &Mixed)
        );
    }

    // A returned number is always finite.
    #[test]
    fn numbers_are_always_finite(s in structured()) {
        if let Value::Number(x) = ExcelEvaluator::eval_with(&s, &Mixed) {
            prop_assert!(x.is_finite());
        }
    }
}
