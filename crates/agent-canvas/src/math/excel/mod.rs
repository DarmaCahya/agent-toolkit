//! Deterministic, panic-free Excel formula evaluator.
//!
//! ```
//! use agent_canvas::math::ExcelEvaluator;
//!
//! let v = ExcelEvaluator::eval(r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#);
//! assert_eq!(v.as_str(), Some("Profit"));
//! assert_eq!(ExcelEvaluator::eval("=-3^2").to_text(), "9");
//! assert_eq!(ExcelEvaluator::eval("=1/0").to_text(), "#DIV/0!");
//! ```
//!
//! [`Value::as_str`] returns the text of a `Text` value only; use [`Value::to_text`] (or
//! `Display`) for the display string of any value.
//!
//! # Operator precedence (tightest first)
//!
//! | Operators | Notes |
//! |---|---|
//! | prefix `+` `-` | tighter than `^`, so `-3^2` is `9` |
//! | postfix `%` | divides by 100; stackable |
//! | `^` | left-associative, so `2^3^2` is `64` |
//! | `*` `/` | |
//! | `+` `-` | |
//! | `&` | text concatenation |
//! | `=` `<>` `<` `>` `<=` `>=` | text compares case-insensitively; numbers equal at ~15 digits; numbers < text < booleans |
//!
//! # Functions
//!
//! `SUM AVERAGE MIN MAX COUNT IF AND OR NOT ABS ROUND IFERROR CONCAT` (case-insensitive).
//! `IF` and `IFERROR` evaluate only the branch they need. Arguments written directly are
//! coerced (`SUM("3", TRUE)` is `4`); values inside a range are not (text, booleans and
//! empty cells are skipped by numeric aggregates).
//!
//! # Errors
//!
//! [`ExcelEvaluator::eval`] is total: failures are [`Value::Error`] values, never panics.
//! [`ExcelEvaluator::try_eval`] instead reports malformed input (syntax, wrong argument
//! count, limits) as a located [`ParseError`]; see [`ExcelEvaluator`] for the mapping.
//!
//! # Empty cells
//!
//! A single reference to an empty cell evaluates to [`Value::Blank`]: `0` in arithmetic and
//! number comparison, `""` in `&`/`CONCAT` and text comparison, `FALSE` in logic, and `0`
//! when displayed. Inside a range, empty cells are skipped.
//!
//! # Limits and determinism
//!
//! Input is limited to 8192 characters and 64 nesting levels (parentheses, call argument
//! lists and prefix-operator chains). Output depends only on the input and the provider:
//! no time, randomness, locale or environment. Number comparison approximates Excel's
//! 15 significant digits; results beyond that precision are not guaranteed to match Excel
//! bit for bit, and fractional powers use the platform's `powf`.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod compare;
mod error;
mod eval;
mod functions;
mod lexer;
mod parser;
mod value;

pub use error::{ParseError, ParseErrorKind};
pub use value::{Cell, ExcelError, RangeProvider, Value};

pub(crate) const MAX_INPUT_CHARS: usize = 8192;
pub(crate) const MAX_DEPTH: usize = 64;

/// Stateless entry point for evaluating Excel formula strings.
///
/// `eval` and `eval_with` are total: they never panic and never return `Result`. Every failure
/// is a [`Value::Error`] (`try_eval*` expose syntax failures as [`ParseError`] instead):
///
/// - syntax error, over-long input, nesting deeper than 64, wrong arity, bad coercion,
///   text `IF` condition → `#VALUE!`
/// - unknown function (including volatile ones), unresolved reference → `#NAME?`
/// - NaN, infinity, overflow, `0^0` → `#NUM!`
/// - division by zero → `#DIV/0!`
#[derive(Debug, Clone, Copy, Default)]
pub struct ExcelEvaluator;

impl ExcelEvaluator {
    /// Evaluates `formula` (leading `=` optional). References evaluate to `#NAME?`.
    pub fn eval(formula: &str) -> Value {
        Self::try_eval(formula).unwrap_or(Value::Error(ExcelError::Value))
    }

    /// Like [`eval`](Self::eval), resolving references such as `A1:B3` through `provider`.
    pub fn eval_with(formula: &str, provider: &dyn RangeProvider) -> Value {
        Self::try_eval_with(formula, provider).unwrap_or(Value::Error(ExcelError::Value))
    }

    /// Like [`eval`](Self::eval), but reports malformed input as a located [`ParseError`]
    /// instead of folding it into `#VALUE!`.
    ///
    /// Only syntax, argument-count and limit failures are `Err`; evaluation failures
    /// (`#DIV/0!`, `#NAME?`, ...) are still `Ok(Value::Error(..))`. Spans are byte offsets
    /// into `formula` itself.
    ///
    /// ```
    /// use agent_canvas::math::excel::{ExcelEvaluator, ParseErrorKind};
    ///
    /// let e = ExcelEvaluator::try_eval("=SUM(").unwrap_err();
    /// assert_eq!(e.kind, ParseErrorKind::UnexpectedEof);
    /// assert_eq!(e.span, 5..5);
    /// assert_eq!(e.to_string(), "unexpected end of input at 5, expected expression or )");
    /// assert!(ExcelEvaluator::try_eval("=1+2").is_ok());
    /// ```
    pub fn try_eval(formula: &str) -> Result<Value, ParseError> {
        evaluate(formula, None)
    }

    /// Like [`try_eval`](Self::try_eval), resolving references through `provider`.
    pub fn try_eval_with(formula: &str, provider: &dyn RangeProvider) -> Result<Value, ParseError> {
        evaluate(formula, Some(provider))
    }
}

/// Translates a span in the sanitized body into byte offsets of the caller's string.
/// `base` is where the body starts in the sanitized text and `map` gives, for each
/// sanitized byte, its offset in the original. Errors at the end of the input point at the
/// end of the original string.
fn relocate(e: ParseError, base: usize, map: &[usize], input_len: usize) -> ParseError {
    let ParseError {
        kind,
        span,
        expected,
    } = e;
    let at = |i: usize| map.get(i).copied().unwrap_or(input_len);
    let span = if kind == ParseErrorKind::UnexpectedEof {
        input_len..input_len
    } else if span.is_empty() {
        at(base + span.start)..at(base + span.start)
    } else {
        // The end is one past the last byte of the span, mapped from that last byte.
        let last = at(base + span.end.saturating_sub(1));
        at(base + span.start)..last.saturating_add(1).min(input_len)
    };
    ParseError::new(kind, span, expected)
}

fn evaluate(formula: &str, provider: Option<&dyn RangeProvider>) -> Result<Value, ParseError> {
    // Counted on the raw input and capped, so oversized input costs O(limit).
    if formula.chars().take(MAX_INPUT_CHARS + 1).count() > MAX_INPUT_CHARS {
        return Err(ParseError::new(
            ParseErrorKind::LimitExceeded,
            0..formula.len(),
            Vec::new(),
        ));
    }
    let (clean, map) = lexer::sanitize_mapped(formula);
    // `base` is the offset of `body` inside `clean`.
    let rest = clean.trim_start();
    let mut base = clean.len() - rest.len();
    let mut body = rest.trim_end();
    if let Some(after) = body.strip_prefix('=') {
        base += 1;
        body = after;
    }
    let rest = body.trim_start();
    base += body.len() - rest.len();
    let body = rest.trim_end();
    let expr = lexer::tokenize(body)
        .and_then(parser::parse)
        .map_err(|e| relocate(e, base, &map, formula.len()))?;
    let mut ctx = eval::Ctx::new(provider);
    let result = eval::eval_expr(&expr, &mut ctx);
    eval::dismantle(expr);
    Ok(result)
}
