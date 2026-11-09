//! Values, errors, display and coercion.
#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::fmt;

/// An Excel error code carried as an ordinary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExcelError {
    /// Division by zero, `0^-n`, or an empty `AVERAGE`.
    #[error("#DIV/0!")]
    DivZero,
    /// Bad coercion, syntax error, limit violation, wrong arity, or a text `IF` condition.
    #[error("#VALUE!")]
    Value,
    /// Unknown function (including volatile ones) or unresolved reference.
    #[error("#NAME?")]
    Name,
    /// NaN, infinity, overflow, `0^0`, or a negative base with a fractional exponent.
    #[error("#NUM!")]
    Num,
}

impl ExcelError {
    /// The Excel display code, e.g. `#DIV/0!`.
    pub fn code(&self) -> &'static str {
        match self {
            ExcelError::DivZero => "#DIV/0!",
            ExcelError::Value => "#VALUE!",
            ExcelError::Name => "#NAME?",
            ExcelError::Num => "#NUM!",
        }
    }
}

/// The result of evaluating a formula or sub-expression.
///
/// A `Number` is never NaN or infinite; such results become `Error(Num)`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// A finite number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// Text.
    Text(String),
    /// An Excel error.
    Error(ExcelError),
    /// An empty cell used as a scalar: reads as 0 in arithmetic, `""` in text and
    /// FALSE in logic; displays as `0`.
    Blank,
}

impl Value {
    /// The text content of a `Text` value; `None` for every other variant.
    ///
    /// Use [`to_text`](Self::to_text) (or `Display`) for the coerced display string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Canonical display string: whole numbers without `.0`, `TRUE`/`FALSE`,
    /// text as itself, errors as their Excel code, blank as `0`.
    pub fn to_text(&self) -> Cow<'_, str> {
        match self {
            Value::Number(x) => Cow::Owned(format_number(*x)),
            Value::Bool(true) => Cow::Borrowed("TRUE"),
            Value::Bool(false) => Cow::Borrowed("FALSE"),
            Value::Text(s) => Cow::Borrowed(s.as_str()),
            Value::Error(e) => Cow::Borrowed(e.code()),
            Value::Blank => Cow::Borrowed("0"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

/// One element of a range supplied by a [`RangeProvider`].
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// An empty cell; skipped by aggregates.
    Empty,
    /// A cell holding a value.
    Value(Value),
}

/// Supplies cell and range contents for references such as `A1` or `A1:B3`.
pub trait RangeProvider {
    /// `reference` is the uppercased reference text, e.g. `A1` or `A1:B3`.
    /// Return the cells flattened row-major, or `None` to make the reference `#NAME?`.
    fn resolve(&self, reference: &str) -> Option<Vec<Cell>>;
}

/// `Number(x)` for finite `x`, otherwise `Error(Num)`.
pub(crate) fn finite_or_num(x: f64) -> Value {
    if x.is_finite() {
        Value::Number(x)
    } else {
        Value::Error(ExcelError::Num)
    }
}

fn format_number(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    if x.fract() == 0.0 && x.abs() < 1e15 {
        // Safe: |x| < 1e15 fits i64 exactly.
        return format!("{}", x as i64);
    }
    let sci = format!("{x:e}");
    let (mantissa, exp) = match sci.split_once('e') {
        Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
        None => return sci,
    };
    if (-7..=20).contains(&exp) {
        return format!("{x}");
    }
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}E{sign}{:02}", exp.unsigned_abs())
}

/// Parses plain decimal/scientific text (`[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?`)
/// after trimming; rejects `inf`, `nan`, hex, separators, and non-finite results.
pub(crate) fn parse_number_text(text: &str) -> Option<f64> {
    let t = text.trim();
    let mut it = t.chars().peekable();
    if matches!(it.peek(), Some('+' | '-')) {
        it.next();
    }
    let mut int_digits = 0usize;
    while it.next_if(char::is_ascii_digit).is_some() {
        int_digits += 1;
    }
    let mut frac_digits = 0usize;
    if it.next_if_eq(&'.').is_some() {
        while it.next_if(char::is_ascii_digit).is_some() {
            frac_digits += 1;
        }
    }
    if int_digits + frac_digits == 0 {
        return None;
    }
    if it.next_if(|c| matches!(c, 'e' | 'E')).is_some() {
        if matches!(it.peek(), Some('+' | '-')) {
            it.next();
        }
        let mut exp_digits = 0usize;
        while it.next_if(char::is_ascii_digit).is_some() {
            exp_digits += 1;
        }
        if exp_digits == 0 {
            return None;
        }
    }
    if it.next().is_some() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Direct-argument numeric coercion.
pub(crate) fn to_number(v: &Value) -> Result<f64, ExcelError> {
    match v {
        Value::Number(x) => Ok(*x),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::Text(s) => parse_number_text(s).ok_or(ExcelError::Value),
        Value::Error(e) => Err(*e),
        Value::Blank => Ok(0.0),
    }
}

/// Text form of a value for `&` and `CONCAT`: a blank reads as `""`; errors propagate.
pub(crate) fn coerce_text(v: &Value) -> Result<Cow<'_, str>, ExcelError> {
    match v {
        Value::Error(e) => Err(*e),
        Value::Blank => Ok(Cow::Borrowed("")),
        other => Ok(other.to_text()),
    }
}

/// Logical coercion of a direct argument: a blank reads as FALSE; numeric text coerces;
/// other text is `#VALUE!`; errors propagate.
pub(crate) fn coerce_logical(v: &Value) -> Result<bool, ExcelError> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Number(x) => Ok(*x != 0.0),
        Value::Text(s) => parse_number_text(s)
            .map(|x| x != 0.0)
            .ok_or(ExcelError::Value),
        Value::Error(e) => Err(*e),
        Value::Blank => Ok(false),
    }
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

    fn n(x: f64) -> String {
        Value::Number(x).to_text().into_owned()
    }

    #[test]
    fn display_whole_numbers_have_no_decimal_point() {
        assert_eq!(n(3.0), "3");
        assert_eq!(n(-0.0), "0");
        assert_eq!(n(0.0), "0");
        assert_eq!(n(0.5), "0.5");
        assert_eq!(n(-12.0), "-12");
    }

    #[test]
    fn display_plain_decimal_within_exponent_window() {
        assert_eq!(n(1e15), "1000000000000000");
        assert_eq!(n(1e20), "100000000000000000000");
        assert_eq!(n(1.5e-7), "0.00000015");
        assert_eq!(n(0.30000000000000004), "0.30000000000000004");
    }

    #[test]
    fn display_scientific_outside_window() {
        assert_eq!(n(1e21), "1E+21");
        assert_eq!(n(1.5e21), "1.5E+21");
        assert_eq!(n(1e-8), "1E-08");
        assert_eq!(n(-2.5e-10), "-2.5E-10");
        assert_eq!(n(1e300), "1E+300");
    }

    #[test]
    fn display_other_kinds() {
        assert_eq!(Value::Bool(true).to_text(), "TRUE");
        assert_eq!(Value::Bool(false).to_text(), "FALSE");
        assert_eq!(Value::Text("hi there".into()).to_text(), "hi there");
        assert_eq!(Value::Error(ExcelError::DivZero).to_text(), "#DIV/0!");
        assert_eq!(Value::Error(ExcelError::Value).to_text(), "#VALUE!");
        assert_eq!(Value::Error(ExcelError::Name).to_text(), "#NAME?");
        assert_eq!(Value::Error(ExcelError::Num).to_text(), "#NUM!");
        assert_eq!(Value::Blank.to_text(), "0");
        assert_eq!(ExcelError::DivZero.code(), "#DIV/0!");
        assert_eq!(ExcelError::Num.to_string(), "#NUM!");
    }

    fn every_variant() -> Vec<Value> {
        vec![
            Value::Number(2.5),
            Value::Number(3.0),
            Value::Bool(true),
            Value::Bool(false),
            Value::Text("hi".into()),
            Value::Text(String::new()),
            Value::Error(ExcelError::DivZero),
            Value::Error(ExcelError::Num),
            Value::Blank,
        ]
    }

    #[test]
    fn as_str_is_some_for_text_only() {
        assert_eq!(Value::Text("hi".into()).as_str(), Some("hi"));
        assert_eq!(Value::Text(String::new()).as_str(), Some(""));
        for v in every_variant() {
            if !matches!(v, Value::Text(_)) {
                assert_eq!(v.as_str(), None, "{v:?}");
            }
        }
    }

    #[test]
    fn display_equals_to_text_for_every_variant() {
        for v in every_variant() {
            assert_eq!(format!("{v}"), v.to_text(), "{v:?}");
        }
        assert_eq!(Value::Number(3.0).to_string(), "3");
    }

    #[test]
    fn text_for_concat_reads_blank_as_empty() {
        assert_eq!(coerce_text(&Value::Blank).unwrap(), "");
        assert_eq!(coerce_text(&Value::Number(1.0)).unwrap(), "1");
        assert_eq!(coerce_text(&Value::Bool(true)).unwrap(), "TRUE");
        assert_eq!(
            coerce_text(&Value::Error(ExcelError::Num)),
            Err(ExcelError::Num)
        );
    }

    #[test]
    fn logical_coercion_reads_blank_as_false() {
        assert_eq!(coerce_logical(&Value::Blank), Ok(false));
        assert_eq!(coerce_logical(&Value::Bool(true)), Ok(true));
        assert_eq!(coerce_logical(&Value::Number(0.0)), Ok(false));
        assert_eq!(coerce_logical(&Value::Number(2.0)), Ok(true));
        assert_eq!(coerce_logical(&Value::Text("1".into())), Ok(true));
        assert_eq!(
            coerce_logical(&Value::Text("x".into())),
            Err(ExcelError::Value)
        );
        assert_eq!(
            coerce_logical(&Value::Error(ExcelError::Name)),
            Err(ExcelError::Name)
        );
    }

    #[test]
    fn numeric_coercion_reads_blank_as_zero() {
        assert_eq!(to_number(&Value::Blank), Ok(0.0));
    }

    #[test]
    fn finite_or_num_rejects_non_finite() {
        assert_eq!(finite_or_num(1.5), Value::Number(1.5));
        assert_eq!(finite_or_num(f64::NAN), Value::Error(ExcelError::Num));
        assert_eq!(finite_or_num(f64::INFINITY), Value::Error(ExcelError::Num));
        assert_eq!(
            finite_or_num(f64::NEG_INFINITY),
            Value::Error(ExcelError::Num)
        );
    }

    #[test]
    fn coercion_accepts_plain_numeric_text() {
        assert_eq!(parse_number_text("3"), Some(3.0));
        assert_eq!(parse_number_text(" 3 "), Some(3.0));
        assert_eq!(parse_number_text("1E3"), Some(1000.0));
        assert_eq!(parse_number_text(".5"), Some(0.5));
        assert_eq!(parse_number_text("-2.5e-1"), Some(-0.25));
        assert_eq!(parse_number_text("+4."), Some(4.0));
    }

    #[test]
    fn coercion_rejects_everything_else() {
        for s in [
            "", " ", "abc", "inf", "nan", "-inf", "0x10", "1,000", "1e", "e3", "--1", ".", "1e999",
            "1 2",
        ] {
            assert_eq!(parse_number_text(s), None, "{s:?}");
        }
    }

    #[test]
    fn to_number_and_to_text() {
        assert_eq!(to_number(&Value::Number(2.0)), Ok(2.0));
        assert_eq!(to_number(&Value::Bool(true)), Ok(1.0));
        assert_eq!(to_number(&Value::Bool(false)), Ok(0.0));
        assert_eq!(to_number(&Value::Text("3".into())), Ok(3.0));
        assert_eq!(to_number(&Value::Text("x".into())), Err(ExcelError::Value));
        assert_eq!(
            to_number(&Value::Error(ExcelError::Name)),
            Err(ExcelError::Name)
        );
        assert_eq!(coerce_text(&Value::Number(1.0)).unwrap(), "1");
        assert_eq!(coerce_text(&Value::Bool(true)).unwrap(), "TRUE");
        assert_eq!(
            coerce_text(&Value::Error(ExcelError::Num)),
            Err(ExcelError::Num)
        );
    }
}
