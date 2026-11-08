//! Excel comparison semantics.
#![forbid(unsafe_code)]

use std::cmp::Ordering;

use super::value::{ExcelError, Value};

/// Number equality at roughly Excel's 15 significant digits: exact, or within
/// `4 * EPSILON` relative. (`0.1+0.2` and `0.3` differ by about 1 ulp, so they compare equal.)
fn numbers_equal(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs())
}

fn compare_numbers(a: f64, b: f64) -> Ordering {
    if numbers_equal(a, b) {
        Ordering::Equal
    } else {
        a.partial_cmp(&b).unwrap_or(Ordering::Equal)
    }
}

fn compare_text(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}

fn rank(v: &Value) -> u8 {
    match v {
        Value::Number(_) => 0,
        Value::Text(_) => 1,
        Value::Bool(_) => 2,
        Value::Error(_) => 3,
    }
}

/// Orders two values the way Excel's comparison operators do:
/// numbers < text < booleans, text case-insensitive. An error in either operand
/// (left first) is returned instead of an ordering.
pub(crate) fn compare(a: &Value, b: &Value) -> Result<Ordering, ExcelError> {
    if let Value::Error(e) = a {
        return Err(*e);
    }
    if let Value::Error(e) = b {
        return Err(*e);
    }
    Ok(match (a, b) {
        (Value::Number(x), Value::Number(y)) => compare_numbers(*x, *y),
        (Value::Text(x), Value::Text(y)) => compare_text(x, y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )] // test code
    use super::*;

    fn num(x: f64) -> Value {
        Value::Number(x)
    }
    fn txt(s: &str) -> Value {
        Value::Text(s.into())
    }

    #[test]
    fn numbers_basic() {
        assert_eq!(compare(&num(1.0), &num(2.0)), Ok(Ordering::Less));
        assert_eq!(compare(&num(3.0), &num(2.0)), Ok(Ordering::Greater));
        assert_eq!(compare(&num(2.0), &num(2.0)), Ok(Ordering::Equal));
    }

    #[test]
    fn errors_propagate_left_first() {
        let e1 = Value::Error(ExcelError::DivZero);
        let e2 = Value::Error(ExcelError::Num);
        assert_eq!(compare(&e1, &num(1.0)), Err(ExcelError::DivZero));
        assert_eq!(compare(&num(1.0), &e2), Err(ExcelError::Num));
        assert_eq!(compare(&e1, &e2), Err(ExcelError::DivZero));
    }

    #[test]
    fn fifteen_digit_equality() {
        assert_eq!(compare(&num(0.1 + 0.2), &num(0.3)), Ok(Ordering::Equal));
        assert_eq!(
            compare(&num(1.0), &num(1.000000000000002)),
            Ok(Ordering::Less)
        );
        assert_eq!(compare(&num(0.0), &num(-0.0)), Ok(Ordering::Equal));
        assert_eq!(
            compare(&num(1e300), &num(1e300 * (1.0 + f64::EPSILON))),
            Ok(Ordering::Equal)
        );
        assert_eq!(
            compare(&num(1e300), &num(1.0000000000001e300)),
            Ok(Ordering::Less)
        );
        assert_eq!(compare(&num(0.0), &num(1e-300)), Ok(Ordering::Less));
    }

    #[test]
    fn cross_type_ordering() {
        assert_eq!(compare(&num(1e9), &txt("a")), Ok(Ordering::Less));
        assert_eq!(
            compare(&txt("zzz"), &Value::Bool(false)),
            Ok(Ordering::Less)
        );
        assert_eq!(compare(&num(1.0), &Value::Bool(true)), Ok(Ordering::Less));
        assert_eq!(
            compare(&Value::Bool(true), &num(1.0)),
            Ok(Ordering::Greater)
        );
    }

    #[test]
    fn text_is_case_insensitive_then_code_point() {
        assert_eq!(compare(&txt("ABC"), &txt("abc")), Ok(Ordering::Equal));
        assert_eq!(compare(&txt("a"), &txt("B")), Ok(Ordering::Less));
        assert_eq!(compare(&txt("b"), &txt("A")), Ok(Ordering::Greater));
        assert_eq!(compare(&txt("ab"), &txt("abc")), Ok(Ordering::Less));
        assert_eq!(compare(&txt(""), &txt("a")), Ok(Ordering::Less));
    }

    #[test]
    fn bool_false_before_true() {
        assert_eq!(
            compare(&Value::Bool(false), &Value::Bool(true)),
            Ok(Ordering::Less)
        );
        assert_eq!(
            compare(&Value::Bool(true), &Value::Bool(true)),
            Ok(Ordering::Equal)
        );
    }
}
