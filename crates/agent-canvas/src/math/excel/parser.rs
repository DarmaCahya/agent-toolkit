//! Pratt parser producing the expression tree.
#![forbid(unsafe_code)]

use std::ops::Range;

use super::error::{ParseError, ParseErrorKind};
use super::eval::dismantle;
use super::functions::arity;
use super::lexer::{Spanned, Token};
use super::MAX_DEPTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnOp {
    Neg,
    Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Expr {
    Number(f64),
    Text(String),
    Bool(bool),
    Ref(String),
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    /// `operand` divided by 100, `times` times. Consecutive `%` collapse into one
    /// node so `5%%%...%` cannot build a tree as deep as the input.
    Percent {
        operand: Box<Expr>,
        times: usize,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

const BP_PERCENT: u8 = 6;

fn infix_bp(op: BinOp) -> u8 {
    match op {
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => 1,
        BinOp::Concat => 2,
        BinOp::Add | BinOp::Sub => 3,
        BinOp::Mul | BinOp::Div => 4,
        BinOp::Pow => 5,
    }
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    /// Where the input ends (empty span for end-of-input errors).
    end: usize,
}

/// The error for finding `found` where `expected` was required.
fn unexpected(found: &Spanned, expected: Vec<&'static str>) -> ParseError {
    let kind = if found.tok == Token::Eof {
        ParseErrorKind::UnexpectedEof
    } else {
        ParseErrorKind::UnexpectedToken
    };
    ParseError::new(kind, found.span.clone(), expected)
}

/// Rejects a call to a known function with too few or too many arguments. Unknown names
/// pass (they evaluate to `#NAME?`).
fn check_arity(name: &str, got: usize, span: Range<usize>) -> Result<(), ParseError> {
    match arity(name) {
        Some((min, max)) if got < min || got > max => Err(ParseError::new(
            ParseErrorKind::WrongArgCount {
                name: name.to_string(),
                min,
                max: (max != usize::MAX).then_some(max),
                got,
            },
            span,
            Vec::new(),
        )),
        _ => Ok(()),
    }
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map_or(&Token::Eof, |t| &t.tok)
    }

    fn span_here(&self) -> Range<usize> {
        self.tokens
            .get(self.pos)
            .map_or(self.end..self.end, |t| t.span.clone())
    }

    fn bump(&mut self) -> Spanned {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Spanned {
            tok: Token::Eof,
            span: self.end..self.end,
        });
        self.pos += 1;
        t
    }

    fn enter(depth: usize, at: &Range<usize>) -> Result<usize, ParseError> {
        if depth >= MAX_DEPTH {
            Err(ParseError::new(
                ParseErrorKind::LimitExceeded,
                at.clone(),
                Vec::new(),
            ))
        } else {
            Ok(depth + 1)
        }
    }

    fn parse_expr(&mut self, min_bp: u8, depth: usize) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix(depth)?;
        loop {
            match self.peek() {
                Token::Percent if BP_PERCENT >= min_bp => {
                    self.bump();
                    lhs = match lhs {
                        Expr::Percent { operand, times } => Expr::Percent {
                            operand,
                            times: times.saturating_add(1),
                        },
                        other => Expr::Percent {
                            operand: Box::new(other),
                            times: 1,
                        },
                    };
                }
                Token::Op(op) => {
                    let op = *op;
                    let bp = infix_bp(op);
                    if bp < min_bp {
                        break;
                    }
                    self.bump();
                    let rhs = match self.parse_expr(bp + 1, depth) {
                        Ok(rhs) => rhs,
                        Err(e) => {
                            // Avoid recursing down a long left spine on drop.
                            dismantle(lhs);
                            return Err(e);
                        }
                    };
                    lhs = Expr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    /// Parses call arguments after the opening parenthesis `open`; returns the argument
    /// list and the end offset of the closing parenthesis. Directly after `(` the closing
    /// parenthesis is also acceptable.
    fn parse_args(
        &mut self,
        depth: usize,
        open: &Range<usize>,
    ) -> Result<(Vec<Expr>, usize), ParseError> {
        let d = Self::enter(depth, open)?;
        let mut args = Vec::new();
        if *self.peek() == Token::RParen {
            let close = self.bump();
            return Ok((args, close.span.end));
        }
        loop {
            let first = args.is_empty();
            let here = self.span_here();
            let arg = self.parse_expr(0, d).map_err(|mut e| {
                let at_start = matches!(
                    e.kind,
                    ParseErrorKind::UnexpectedEof | ParseErrorKind::UnexpectedToken
                ) && e.span == here;
                if first && at_start && e.expected == ["expression"] {
                    e.expected.push(")");
                }
                e
            })?;
            args.push(arg);
            let t = self.bump();
            match t.tok {
                Token::Comma => {}
                Token::RParen => return Ok((args, t.span.end)),
                _ => return Err(unexpected(&t, vec![",", ")"])),
            }
        }
    }

    fn parse_prefix(&mut self, depth: usize) -> Result<Expr, ParseError> {
        let t = self.bump();
        match t.tok {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Str(s) => Ok(Expr::Text(s)),
            Token::Bool(b) => Ok(Expr::Bool(b)),
            Token::Ref(r) => Ok(Expr::Ref(r)),
            Token::Ident(name) => {
                let open = self.bump();
                if open.tok != Token::LParen {
                    return Err(unexpected(&open, vec!["("]));
                }
                let (args, end) = self.parse_args(depth, &open.span)?;
                let name = name.to_ascii_uppercase();
                check_arity(&name, args.len(), t.span.start..end)?;
                Ok(Expr::Call { name, args })
            }
            Token::LParen => {
                let d = Self::enter(depth, &t.span)?;
                let inner = self.parse_expr(0, d)?;
                let close = self.bump();
                if close.tok != Token::RParen {
                    return Err(unexpected(&close, vec![")"]));
                }
                Ok(inner)
            }
            Token::Op(op @ (BinOp::Add | BinOp::Sub)) => {
                let d = Self::enter(depth, &t.span)?;
                let operand = self.parse_prefix(d)?;
                let op = if op == BinOp::Sub {
                    UnOp::Neg
                } else {
                    UnOp::Pos
                };
                Ok(Expr::Unary {
                    op,
                    operand: Box::new(operand),
                })
            }
            _ => Err(unexpected(&t, vec!["expression"])),
        }
    }
}

/// Parses a full token stream; trailing tokens are an error.
pub(crate) fn parse(tokens: Vec<Spanned>) -> Result<Expr, ParseError> {
    let end = tokens.last().map_or(0, |t| t.span.end);
    let mut p = Parser {
        tokens,
        pos: 0,
        end,
    };
    let expr = p.parse_expr(0, 0)?;
    if *p.peek() != Token::Eof {
        let t = p.bump();
        return Err(unexpected(&t, Vec::new()));
    }
    Ok(expr)
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
    use crate::math::excel::lexer::tokenize;

    fn p(s: &str) -> Result<Expr, ParseError> {
        parse(tokenize(s).unwrap())
    }

    fn num(x: f64) -> Box<Expr> {
        Box::new(Expr::Number(x))
    }

    fn bin(op: BinOp, l: Box<Expr>, r: Box<Expr>) -> Box<Expr> {
        Box::new(Expr::Binary { op, lhs: l, rhs: r })
    }

    #[test]
    fn precedence_mul_over_add() {
        assert_eq!(
            p("1+2*3").unwrap(),
            *bin(BinOp::Add, num(1.0), bin(BinOp::Mul, num(2.0), num(3.0)))
        );
        assert_eq!(
            p("(1+2)*3").unwrap(),
            *bin(BinOp::Mul, bin(BinOp::Add, num(1.0), num(2.0)), num(3.0))
        );
    }

    #[test]
    fn pow_is_left_associative_and_unary_binds_tighter() {
        assert_eq!(
            p("2^3^2").unwrap(),
            *bin(BinOp::Pow, bin(BinOp::Pow, num(2.0), num(3.0)), num(2.0))
        );
        assert_eq!(
            p("-3^2").unwrap(),
            *bin(
                BinOp::Pow,
                Box::new(Expr::Unary {
                    op: UnOp::Neg,
                    operand: num(3.0)
                }),
                num(2.0)
            )
        );
    }

    #[test]
    fn calls_unary_percent_ref() {
        assert_eq!(
            p("f(1,2)").unwrap(),
            Expr::Call {
                name: "F".into(),
                args: vec![Expr::Number(1.0), Expr::Number(2.0)]
            }
        );
        assert_eq!(
            p("f()").unwrap(),
            Expr::Call {
                name: "F".into(),
                args: vec![]
            }
        );
        assert_eq!(
            p("-5").unwrap(),
            Expr::Unary {
                op: UnOp::Neg,
                operand: num(5.0)
            }
        );
        assert_eq!(
            p("50%").unwrap(),
            Expr::Percent {
                operand: num(50.0),
                times: 1
            }
        );
        assert_eq!(
            p("50%%").unwrap(),
            Expr::Percent {
                operand: num(50.0),
                times: 2
            }
        );
        assert_eq!(p("A1:B3").unwrap(), Expr::Ref("A1:B3".into()));
    }

    #[test]
    fn syntax_failures() {
        for s in [
            "1+", "(1", "SUM(", ")", "1 2", "5% 3", "", "1+*2", "f(1,)", "f(,1)", "f(1 2)",
        ] {
            let toks = tokenize(s).unwrap();
            assert!(parse(toks).is_err(), "{s:?}");
        }
    }

    #[test]
    fn eof_errors_are_located_with_expected_items() {
        let e = p("SUM(").unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::UnexpectedEof);
        assert_eq!(e.span, 4..4);
        assert_eq!(e.expected, vec!["expression", ")"]);
        let e = p("SUM(1,").unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::UnexpectedEof);
        assert_eq!(e.expected, vec!["expression"]);
        let e = p("SUM(1").unwrap_err();
        assert_eq!(e.expected, vec![",", ")"]);
        let e = p("(1").unwrap_err();
        assert_eq!(e.expected, vec![")"]);
        let e = p("1+").unwrap_err();
        assert_eq!(e.expected, vec!["expression"]);
    }

    #[test]
    fn token_errors_carry_the_token_span() {
        let e = p("1 2").unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::UnexpectedToken);
        assert_eq!(e.span, 2..3);
        let e = p("SUM(*)").unwrap_err();
        assert_eq!(e.span, 4..5);
        assert_eq!(e.expected, vec!["expression", ")"]);
    }

    #[test]
    fn depth_boundary_parens() {
        let ok = format!("{}1{}", "(".repeat(64), ")".repeat(64));
        let bad = format!("{}1{}", "(".repeat(65), ")".repeat(65));
        assert!(p(&ok).is_ok());
        assert_eq!(p(&bad).unwrap_err().kind, ParseErrorKind::LimitExceeded);
    }

    #[test]
    fn depth_boundary_prefix_and_calls() {
        assert!(p(&format!("{}1", "-".repeat(64))).is_ok());
        assert_eq!(
            p(&format!("{}1", "-".repeat(65))).unwrap_err().kind,
            ParseErrorKind::LimitExceeded
        );
        let call = |n: usize| format!("{}1{}", "ABS(".repeat(n), ")".repeat(n));
        assert!(p(&call(64)).is_ok());
        assert_eq!(
            p(&call(65)).unwrap_err().kind,
            ParseErrorKind::LimitExceeded
        );
    }
}
