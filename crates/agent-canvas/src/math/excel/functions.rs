//! Phase-1 function implementations.
#![forbid(unsafe_code)]

use super::eval::{eval_arg, eval_expr, ipow, Arg, Ctx};
use super::parser::Expr;
use super::value::{
    coerce_logical, coerce_text, finite_or_num, to_number, Cell, ExcelError, Value,
};

/// `(min, max)` argument counts (case-insensitive name), or `None` for names outside the
/// Phase-1 set. Variadic functions need at least one argument; their max is `usize::MAX`.
pub(crate) fn arity(name: &str) -> Option<(usize, usize)> {
    match name.to_ascii_uppercase().as_str() {
        "NOT" | "ABS" => Some((1, 1)),
        "ROUND" | "IFERROR" => Some((2, 2)),
        "IF" => Some((2, 3)),
        "SUM" | "AVERAGE" | "MIN" | "MAX" | "COUNT" | "AND" | "OR" | "CONCAT" => {
            Some((1, usize::MAX))
        }
        _ => None,
    }
}

fn value_error() -> Value {
    Value::Error(ExcelError::Value)
}

/// Dispatches a call by uppercased name. Unknown names never evaluate their arguments.
pub(crate) fn call(name: &str, args: &[Expr], ctx: &mut Ctx<'_>) -> Value {
    let Some((min, max)) = arity(name) else {
        return Value::Error(ExcelError::Name);
    };
    if args.len() < min || args.len() > max {
        return value_error();
    }
    match name {
        "IF" => if_fn(args, ctx),
        "IFERROR" => iferror_fn(args, ctx),
        "NOT" | "ABS" => unary_fn(name, args, ctx),
        "ROUND" => round_fn(args, ctx),
        _ => {
            let evaluated: Vec<Arg> = args.iter().map(|a| eval_arg(a, ctx)).collect();
            match name {
                "AND" => logical(true, &evaluated),
                "OR" => logical(false, &evaluated),
                "CONCAT" => concat(&evaluated),
                _ => numeric(name, &evaluated),
            }
        }
    }
}

/// `IF` takes a boolean or numeric condition; text (even numeric text) is `#VALUE!`.
fn if_fn(args: &[Expr], ctx: &mut Ctx<'_>) -> Value {
    let (Some(cond), Some(then)) = (args.first(), args.get(1)) else {
        return value_error();
    };
    let truth = match eval_expr(cond, ctx) {
        Value::Bool(b) => b,
        Value::Number(x) => x != 0.0,
        Value::Text(_) => return value_error(),
        Value::Error(e) => return Value::Error(e),
        Value::Blank => false,
    };
    if truth {
        eval_expr(then, ctx)
    } else {
        args.get(2)
            .map_or(Value::Bool(false), |e| eval_expr(e, ctx))
    }
}

fn iferror_fn(args: &[Expr], ctx: &mut Ctx<'_>) -> Value {
    let (Some(first), Some(fallback)) = (args.first(), args.get(1)) else {
        return value_error();
    };
    match eval_expr(first, ctx) {
        Value::Error(_) => eval_expr(fallback, ctx),
        ok => ok,
    }
}

fn unary_fn(name: &str, args: &[Expr], ctx: &mut Ctx<'_>) -> Value {
    let Some(arg) = args.first() else {
        return value_error();
    };
    let v = eval_expr(arg, ctx);
    if name == "NOT" {
        return match coerce_logical(&v) {
            Ok(b) => Value::Bool(!b),
            Err(e) => Value::Error(e),
        };
    }
    match to_number(&v) {
        Ok(x) => finite_or_num(x.abs()),
        Err(e) => Value::Error(e),
    }
}

fn round_fn(args: &[Expr], ctx: &mut Ctx<'_>) -> Value {
    let (Some(x), Some(digits)) = (args.first(), args.get(1)) else {
        return value_error();
    };
    let x = match to_number(&eval_expr(x, ctx)) {
        Ok(x) => x,
        Err(e) => return Value::Error(e),
    };
    let digits = match to_number(&eval_expr(digits, ctx)) {
        Ok(d) => d,
        Err(e) => return Value::Error(e),
    };
    round(x, digits)
}

/// Rounds halves away from zero; treats values within ~15 significant digits of a half as a half
/// (so `2.675` rounds to `2.68` at two digits, as Excel does).
fn round_half_away(y: f64) -> f64 {
    let a = y.abs();
    let tolerance = (4.0 * f64::EPSILON * a).min(0.25);
    let r = if (a.fract() - 0.5).abs() <= tolerance {
        a.trunc() + 1.0
    } else {
        a.round()
    };
    r.copysign(y)
}

pub(crate) fn round(x: f64, digits: f64) -> Value {
    // Truncate toward zero, then clamp: the saturating cast cannot wrap.
    let k = digits.trunc().clamp(-400.0, 400.0) as i32;
    let scale = ipow(10.0, k.unsigned_abs());
    if k >= 0 {
        if !scale.is_finite() {
            return finite_or_num(x);
        }
        let y = x * scale;
        if !y.is_finite() {
            // Too large to have a fractional part to round away.
            return finite_or_num(x);
        }
        finite_or_num(round_half_away(y) / scale)
    } else {
        if !scale.is_finite() {
            return Value::Number(0.0);
        }
        finite_or_num(round_half_away(x / scale) * scale)
    }
}

/// Feeds every numeric value to `f`. Direct arguments are coerced (`COUNT` merely skips what
/// does not coerce); range elements count only when they are real numbers. The first error
/// in argument order propagates.
fn for_each_number(
    args: &[Arg],
    count_mode: bool,
    f: &mut dyn FnMut(f64),
) -> Result<(), ExcelError> {
    for arg in args {
        match arg {
            Arg::Scalar(Value::Error(e)) => return Err(*e),
            Arg::Scalar(v) => {
                if count_mode {
                    if let Ok(x) = to_number(v) {
                        f(x);
                    }
                } else {
                    f(to_number(v)?);
                }
            }
            Arg::Range(cells) => {
                for cell in cells {
                    match cell {
                        Cell::Value(Value::Number(x)) => f(*x),
                        Cell::Value(Value::Error(e)) => return Err(*e),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn numeric(name: &str, args: &[Arg]) -> Value {
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let walked = for_each_number(args, name == "COUNT", &mut |x| {
        sum += x;
        count += 1;
        min = min.min(x);
        max = max.max(x);
    });
    if let Err(e) = walked {
        return Value::Error(e);
    }
    match name {
        "SUM" => finite_or_num(sum),
        "COUNT" => Value::Number(count as f64),
        "AVERAGE" if count == 0 => Value::Error(ExcelError::DivZero),
        "AVERAGE" => finite_or_num(sum / count as f64),
        "MIN" if count == 0 => Value::Number(0.0),
        "MIN" => Value::Number(min),
        "MAX" if count == 0 => Value::Number(0.0),
        "MAX" => Value::Number(max),
        _ => Value::Error(ExcelError::Name),
    }
}

fn logical(is_and: bool, args: &[Arg]) -> Value {
    let mut seen = false;
    let mut acc = is_and;
    let mut fold = |b: bool| {
        seen = true;
        acc = if is_and { acc && b } else { acc || b };
    };
    for arg in args {
        match arg {
            Arg::Scalar(v) => match coerce_logical(v) {
                Ok(b) => fold(b),
                Err(e) => return Value::Error(e),
            },
            // A single reference to an empty cell is a blank scalar, which reads as FALSE.
            Arg::Range(cells)
                if matches!(cells.as_slice(), [Cell::Empty | Cell::Value(Value::Blank)]) =>
            {
                fold(false);
            }
            Arg::Range(cells) => {
                for cell in cells {
                    match cell {
                        Cell::Value(Value::Bool(b)) => fold(*b),
                        Cell::Value(Value::Number(x)) => fold(*x != 0.0),
                        Cell::Value(Value::Error(e)) => return Value::Error(*e),
                        _ => {}
                    }
                }
            }
        }
    }
    if seen {
        Value::Bool(acc)
    } else {
        value_error()
    }
}

fn concat(args: &[Arg]) -> Value {
    let mut out = String::new();
    for arg in args {
        match arg {
            Arg::Scalar(v) => match coerce_text(v) {
                Ok(s) => out.push_str(&s),
                Err(e) => return Value::Error(e),
            },
            Arg::Range(cells) => {
                for cell in cells {
                    if let Cell::Value(v) = cell {
                        match coerce_text(v) {
                            Ok(s) => out.push_str(&s),
                            Err(e) => return Value::Error(e),
                        }
                    }
                }
            }
        }
    }
    Value::Text(out)
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

    #[test]
    fn arity_table() {
        for fixed in [
            ("NOT", 1, 1),
            ("ABS", 1, 1),
            ("ROUND", 2, 2),
            ("IFERROR", 2, 2),
            ("IF", 2, 3),
        ] {
            assert_eq!(arity(fixed.0), Some((fixed.1, fixed.2)), "{}", fixed.0);
        }
        for variadic in [
            "SUM", "AVERAGE", "MIN", "MAX", "COUNT", "AND", "OR", "CONCAT",
        ] {
            assert_eq!(arity(variadic), Some((1, usize::MAX)), "{variadic}");
        }
        // Lookup is case-insensitive.
        assert_eq!(arity("sum"), Some((1, usize::MAX)));
        assert_eq!(arity("If"), Some((2, 3)));
        for unknown in ["NOW", "RAND", "TODAY", "VLOOKUP", ""] {
            assert_eq!(arity(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn empty_variadics() {
        assert_eq!(numeric("SUM", &[]), Value::Number(0.0));
        assert_eq!(numeric("COUNT", &[]), Value::Number(0.0));
        assert_eq!(numeric("MIN", &[]), Value::Number(0.0));
        assert_eq!(numeric("MAX", &[]), Value::Number(0.0));
        assert_eq!(numeric("AVERAGE", &[]), Value::Error(ExcelError::DivZero));
        assert_eq!(logical(true, &[]), Value::Error(ExcelError::Value));
        assert_eq!(logical(false, &[]), Value::Error(ExcelError::Value));
        assert_eq!(concat(&[]), Value::Text(String::new()));
    }

    #[test]
    fn runtime_guard_rejects_zero_arg_variadics() {
        // Parse-time validation is the primary check; the runtime guard is defence in depth.
        let mut ctx = Ctx::new(None);
        for name in [
            "SUM", "AVERAGE", "MIN", "MAX", "COUNT", "AND", "OR", "CONCAT",
        ] {
            assert_eq!(
                call(name, &[], &mut ctx),
                Value::Error(ExcelError::Value),
                "{name}"
            );
        }
    }

    #[test]
    fn round_basics() {
        assert_eq!(round(2.5, 0.0), Value::Number(3.0));
        assert_eq!(round(-2.5, 0.0), Value::Number(-3.0));
        assert_eq!(round(1234.0, -2.0), Value::Number(1200.0));
        assert_eq!(round(2.675, 2.0), Value::Number(2.68));
        assert_eq!(round(1.005, 2.0), Value::Number(1.01));
        assert_eq!(round(2.4, 0.0), Value::Number(2.0));
        assert_eq!(round(-0.4, 0.0), Value::Number(-0.0));
    }

    #[test]
    fn round_digit_edge_cases() {
        // Fractional digit counts truncate toward zero.
        assert_eq!(round(7.12159, 2.9), Value::Number(7.12));
        assert_eq!(round(1234.0, -2.9), Value::Number(1200.0));
        // Digits clamp to +-400; huge positive digits leave x unchanged.
        assert_eq!(round(1.5, 1e9), Value::Number(1.5));
        assert_eq!(round(1.5, 400.0), Value::Number(1.5));
        // Huge negative digits round everything to zero.
        assert_eq!(round(1.5e300, -1e9), Value::Number(0.0));
        // Values too large to have a fraction are unchanged, never a panic or NaN.
        assert_eq!(round(1e300, 10.0), Value::Number(1e300));
        assert_eq!(round(1e15, 0.0), Value::Number(1e15));
        // Rounding up past the top of the range overflows.
        assert_eq!(round(f64::MAX, -308.0), Value::Error(ExcelError::Num));
    }
}
