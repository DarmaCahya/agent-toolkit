//! Values, errors, display and coercion.
#![forbid(unsafe_code)]

use std::borrow::Cow;

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
pub enum Value {
    /// A finite number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// Text.
    Text(String),
    /// An Excel error.
    Error(ExcelError),
}

impl Value {
    /// Canonical display string: whole numbers without `.0`, `TRUE`/`FALSE`,
    /// text as itself, errors as their Excel code.
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            Value::Number(x) => Cow::Owned(format_number(*x)),
            Value::Bool(true) => Cow::Borrowed("TRUE"),
            Value::Bool(false) => Cow::Borrowed("FALSE"),
            Value::Text(s) => Cow::Borrowed(s.as_str()),
            Value::Error(e) => Cow::Borrowed(e.code()),
        }
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
    }
}

/// Text form of a value; errors propagate.
pub(crate) fn to_text(v: &Value) -> Result<Cow<'_, str>, ExcelError> {
    match v {
        Value::Error(e) => Err(*e),
        other => Ok(other.as_str()),
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
        Value::Number(x).as_str().into_owned()
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
        assert_eq!(Value::Bool(true).as_str(), "TRUE");
        assert_eq!(Value::Bool(false).as_str(), "FALSE");
        assert_eq!(Value::Text("hi there".into()).as_str(), "hi there");
        assert_eq!(Value::Error(ExcelError::DivZero).as_str(), "#DIV/0!");
        assert_eq!(Value::Error(ExcelError::Value).as_str(), "#VALUE!");
        assert_eq!(Value::Error(ExcelError::Name).as_str(), "#NAME?");
        assert_eq!(Value::Error(ExcelError::Num).as_str(), "#NUM!");
        assert_eq!(ExcelError::DivZero.code(), "#DIV/0!");
        assert_eq!(ExcelError::Num.to_string(), "#NUM!");
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
        assert_eq!(to_text(&Value::Number(1.0)).unwrap(), "1");
        assert_eq!(to_text(&Value::Bool(true)).unwrap(), "TRUE");
        assert_eq!(
            to_text(&Value::Error(ExcelError::Num)),
            Err(ExcelError::Num)
        );
    }
}
