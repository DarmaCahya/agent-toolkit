//! Tree-walking evaluator.
#![forbid(unsafe_code)]

use super::compare::compare;
use super::functions;
use super::lexer::is_valid_reference;
use super::parser::{BinOp, Expr, UnOp};
use super::value::{coerce_text, finite_or_num, to_number, Cell, ExcelError, RangeProvider, Value};
use super::MAX_DEPTH;

/// Total cells a single formula may pull from a provider.
pub(crate) const MAX_RANGE_CELLS: usize = 1_000_000;

pub(crate) struct Ctx<'a> {
    provider: Option<&'a dyn RangeProvider>,
    depth: usize,
    cells_left: usize,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(provider: Option<&'a dyn RangeProvider>) -> Self {
        Ctx {
            provider,
            depth: 0,
            cells_left: MAX_RANGE_CELLS,
        }
    }
}

/// A function argument: a plain value, or the cells behind a reference.
pub(crate) enum Arg {
    Scalar(Value),
    Range(Vec<Cell>),
}

fn resolve_ref(reference: &str, ctx: &mut Ctx<'_>) -> Result<Vec<Cell>, ExcelError> {
    if !is_valid_reference(reference) {
        return Err(ExcelError::Name);
    }
    let provider = ctx.provider.ok_or(ExcelError::Name)?;
    let cells = provider.resolve(reference).ok_or(ExcelError::Name)?;
    if cells.len() > ctx.cells_left {
        return Err(ExcelError::Value);
    }
    ctx.cells_left -= cells.len();
    // Keep the "a Number is always finite" invariant even for hostile providers.
    Ok(cells
        .into_iter()
        .map(|c| match c {
            Cell::Value(Value::Number(x)) if !x.is_finite() => {
                Cell::Value(Value::Error(ExcelError::Num))
            }
            other => other,
        })
        .collect())
}

/// A one-cell range used as a scalar; an empty cell is [`Value::Blank`].
fn cells_to_scalar(cells: Vec<Cell>) -> Value {
    let mut it = cells.into_iter();
    match (it.next(), it.next()) {
        (Some(Cell::Empty), None) => Value::Blank,
        (Some(Cell::Value(v)), None) => v,
        _ => Value::Error(ExcelError::Value),
    }
}

/// Evaluates a function argument, keeping references as ranges.
pub(crate) fn eval_arg(expr: &Expr, ctx: &mut Ctx<'_>) -> Arg {
    match expr {
        Expr::Ref(r) => match resolve_ref(r, ctx) {
            Ok(cells) => Arg::Range(cells),
            Err(e) => Arg::Scalar(Value::Error(e)),
        },
        other => Arg::Scalar(eval_expr(other, ctx)),
    }
}

pub(crate) fn eval_expr(expr: &Expr, ctx: &mut Ctx<'_>) -> Value {
    match expr {
        Expr::Number(n) => Value::Number(*n),
        Expr::Text(s) => Value::Text(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Ref(r) => match resolve_ref(r, ctx) {
            Ok(cells) => cells_to_scalar(cells),
            Err(e) => Value::Error(e),
        },
        Expr::Unary { op, operand } => {
            if ctx.depth >= MAX_DEPTH {
                return Value::Error(ExcelError::Value);
            }
            ctx.depth += 1;
            let v = eval_expr(operand, ctx);
            ctx.depth -= 1;
            match op {
                UnOp::Pos => v,
                UnOp::Neg => match to_number(&v) {
                    Ok(x) => finite_or_num(-x),
                    Err(e) => Value::Error(e),
                },
            }
        }
        Expr::Percent { operand, times } => {
            let v = eval_expr(operand, ctx);
            match to_number(&v) {
                Ok(mut x) => {
                    for _ in 0..*times {
                        x /= 100.0;
                    }
                    finite_or_num(x)
                }
                Err(e) => Value::Error(e),
            }
        }
        Expr::Binary { .. } => eval_binary_chain(expr, ctx),
        Expr::Call { name, args } => {
            if ctx.depth >= MAX_DEPTH {
                return Value::Error(ExcelError::Value);
            }
            ctx.depth += 1;
            let v = functions::call(name, args, ctx);
            ctx.depth -= 1;
            v
        }
    }
}

/// Folds a left-leaning chain of binary operators iteratively so that
/// `1+1+...+1` never recurses as deep as the chain is long.
fn eval_binary_chain(expr: &Expr, ctx: &mut Ctx<'_>) -> Value {
    let mut rest: Vec<(BinOp, &Expr)> = Vec::new();
    let mut cur = expr;
    while let Expr::Binary { op, lhs, rhs } = cur {
        rest.push((*op, rhs));
        cur = lhs;
    }
    let mut acc = eval_expr(cur, ctx);
    for (op, rhs) in rest.into_iter().rev() {
        if matches!(acc, Value::Error(_)) {
            return acc;
        }
        let r = eval_expr(rhs, ctx);
        acc = apply_binary(op, &acc, &r);
    }
    acc
}

fn apply_binary(op: BinOp, a: &Value, b: &Value) -> Value {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
            let x = match to_number(a) {
                Ok(x) => x,
                Err(e) => return Value::Error(e),
            };
            let y = match to_number(b) {
                Ok(y) => y,
                Err(e) => return Value::Error(e),
            };
            match op {
                BinOp::Add => finite_or_num(x + y),
                BinOp::Sub => finite_or_num(x - y),
                BinOp::Mul => finite_or_num(x * y),
                BinOp::Div if y == 0.0 => Value::Error(ExcelError::DivZero),
                BinOp::Div => finite_or_num(x / y),
                _ => power(x, y),
            }
        }
        BinOp::Concat => match (coerce_text(a), coerce_text(b)) {
            (Ok(l), Ok(r)) => Value::Text(format!("{l}{r}")),
            (Err(e), _) | (_, Err(e)) => Value::Error(e),
        },
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
            match compare(a, b) {
                Ok(ord) => Value::Bool(match op {
                    BinOp::Eq => ord.is_eq(),
                    BinOp::Ne => ord.is_ne(),
                    BinOp::Lt => ord.is_lt(),
                    BinOp::Gt => ord.is_gt(),
                    BinOp::Le => ord.is_le(),
                    _ => ord.is_ge(),
                }),
                Err(e) => Value::Error(e),
            }
        }
    }
}

/// Exact square-and-multiply: only IEEE multiplications, so identical on every platform.
pub(crate) fn ipow(base: f64, exp: u32) -> f64 {
    let mut result = 1.0;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result *= b;
        }
        e >>= 1;
        if e > 0 {
            b *= b;
        }
    }
    result
}

fn power(x: f64, y: f64) -> Value {
    if x == 0.0 {
        return if y == 0.0 {
            Value::Error(ExcelError::Num)
        } else if y < 0.0 {
            Value::Error(ExcelError::DivZero)
        } else {
            Value::Number(0.0)
        };
    }
    let r = if y == 0.5 {
        x.sqrt()
    } else if y.fract() == 0.0 && y.abs() <= 1e9 {
        let p = ipow(x, y.abs() as u32);
        if y < 0.0 {
            1.0 / p
        } else {
            p
        }
    } else {
        x.powf(y)
    };
    finite_or_num(r)
}

/// Drops a parsed tree without recursing down a long left spine.
pub(crate) fn dismantle(mut e: Expr) {
    while let Expr::Binary { lhs, .. } = e {
        e = *lhs;
    }
}
