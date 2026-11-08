//! Deterministic, panic-free Excel formula evaluator.
//!
//! ```
//! use agent_canvas::math::ExcelEvaluator;
//!
//! let v = ExcelEvaluator::eval(r#"=IF(SUM(10, 20) > 25, "Profit", "Loss")"#);
//! assert_eq!(v.as_str(), "Profit");
//! assert_eq!(ExcelEvaluator::eval("=-3^2").as_str(), "9");
//! assert_eq!(ExcelEvaluator::eval("=1/0").as_str(), "#DIV/0!");
//! ```
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
//! Failures are [`Value::Error`] values, never `Result::Err` or panics; see [`ExcelEvaluator`]
//! for the mapping.
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
mod eval;
mod functions;
mod lexer;
mod parser;
mod value;

pub use value::{Cell, ExcelError, RangeProvider, Value};

pub(crate) const MAX_INPUT_CHARS: usize = 8192;
pub(crate) const MAX_DEPTH: usize = 64;

/// Stateless entry point for evaluating Excel formula strings.
///
/// Evaluation is total: nothing here returns `Result` or panics. Every failure is a
/// [`Value::Error`]:
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
        evaluate(formula, None)
    }

    /// Like [`eval`](Self::eval), resolving references such as `A1:B3` through `provider`.
    pub fn eval_with(formula: &str, provider: &dyn RangeProvider) -> Value {
        evaluate(formula, Some(provider))
    }
}

fn evaluate(formula: &str, provider: Option<&dyn RangeProvider>) -> Value {
    let invalid = Value::Error(ExcelError::Value);
    // Counted on the raw input and capped, so oversized input costs O(limit).
    if formula.chars().take(MAX_INPUT_CHARS + 1).count() > MAX_INPUT_CHARS {
        return invalid;
    }
    let clean = lexer::sanitize(formula);
    let body = clean.trim();
    let body = body.strip_prefix('=').unwrap_or(body).trim();
    if body.is_empty() {
        return invalid;
    }
    let Ok(tokens) = lexer::tokenize(body) else {
        return invalid;
    };
    let Ok(expr) = parser::parse(tokens) else {
        return invalid;
    };
    let mut ctx = eval::Ctx::new(provider);
    let result = eval::eval_expr(&expr, &mut ctx);
    eval::dismantle(expr);
    result
}
