//! Structured, located parse errors for the fallible entry points.
#![forbid(unsafe_code)]

use std::fmt;
use std::ops::Range;

/// What went wrong while parsing a formula.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The input ended while more was required.
    #[error("unexpected end of input")]
    UnexpectedEof,
    /// A token (or character) appeared where it is not allowed.
    #[error("unexpected token")]
    UnexpectedToken,
    /// A known function was called with too few or too many arguments.
    #[error("wrong argument count for {name}: allowed {}, got {got}", allowed(*min, *max))]
    WrongArgCount {
        /// Canonical (uppercase) function name.
        name: String,
        /// Minimum allowed argument count.
        min: usize,
        /// Maximum allowed argument count, `None` when unbounded.
        max: Option<usize>,
        /// Number of arguments supplied.
        got: usize,
    },
    /// Input longer than 8192 characters, or nesting deeper than 64 levels.
    #[error("limit exceeded")]
    LimitExceeded,
}

fn allowed(min: usize, max: Option<usize>) -> String {
    match max {
        Some(m) if m == min => min.to_string(),
        Some(m) => format!("{min} to {m}"),
        None => format!("at least {min}"),
    }
}

/// A located formula syntax error.
///
/// `span` is a half-open byte range into the string the caller passed in; it is empty
/// when the problem is the end of the input. `expected` lists what would have been
/// accepted, in a fixed order, and is empty when that is not applicable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind} at {}{}", span.start, Expected(expected))]
#[non_exhaustive]
pub struct ParseError {
    /// The failure category.
    pub kind: ParseErrorKind,
    /// Half-open byte offsets into the original input.
    pub span: Range<usize>,
    /// Labels of what would have been accepted instead.
    pub expected: Vec<&'static str>,
}

impl ParseError {
    /// Builds an error; needed because the struct is `#[non_exhaustive]`.
    pub fn new(kind: ParseErrorKind, span: Range<usize>, expected: Vec<&'static str>) -> Self {
        ParseError {
            kind,
            span,
            expected,
        }
    }
}

/// Renders `, expected a, b or c` (nothing when the list is empty).
struct Expected<'a>(&'a [&'static str]);

impl fmt::Display for Expected<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((last, init)) = self.0.split_last() else {
            return Ok(());
        };
        f.write_str(", expected ")?;
        if init.is_empty() {
            return f.write_str(last);
        }
        f.write_str(&init.join(", "))?;
        write!(f, " or {last}")
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

    fn assert_traits<T: Send + Sync + std::error::Error>() {}

    #[test]
    fn error_is_send_sync_error() {
        assert_traits::<ParseError>();
        assert_traits::<ParseErrorKind>();
    }

    #[test]
    fn display_eof_with_two_expected() {
        let e = ParseError::new(ParseErrorKind::UnexpectedEof, 5..5, vec!["expression", ")"]);
        assert_eq!(
            e.to_string(),
            "unexpected end of input at 5, expected expression or )"
        );
    }

    #[test]
    fn display_one_three_and_none_expected() {
        let one = ParseError::new(ParseErrorKind::UnexpectedToken, 2..3, vec!["expression"]);
        assert_eq!(
            one.to_string(),
            "unexpected token at 2, expected expression"
        );
        let three = ParseError::new(ParseErrorKind::UnexpectedToken, 0..1, vec!["a", "b", "c"]);
        assert_eq!(
            three.to_string(),
            "unexpected token at 0, expected a, b or c"
        );
        let none = ParseError::new(ParseErrorKind::LimitExceeded, 0..0, vec![]);
        assert_eq!(none.to_string(), "limit exceeded at 0");
    }

    #[test]
    fn display_wrong_arg_count() {
        let kind = |min, max| ParseErrorKind::WrongArgCount {
            name: "IF".into(),
            min,
            max,
            got: 1,
        };
        assert_eq!(
            ParseError::new(kind(2, Some(3)), 0..5, vec![]).to_string(),
            "wrong argument count for IF: allowed 2 to 3, got 1 at 0"
        );
        assert!(kind(2, Some(2)).to_string().contains("allowed 2,"));
        assert!(kind(1, None).to_string().contains("at least 1"));
    }
}
