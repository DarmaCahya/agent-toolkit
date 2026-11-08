//! Pratt parser producing the expression tree.
#![forbid(unsafe_code)]

use super::eval::dismantle;
use super::lexer::Token;
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
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        t
    }

    fn enter(depth: usize) -> Result<usize, ()> {
        if depth >= MAX_DEPTH {
            Err(())
        } else {
            Ok(depth + 1)
        }
    }

    fn parse_expr(&mut self, min_bp: u8, depth: usize) -> Result<Expr, ()> {
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
                        Err(()) => {
                            // Avoid recursing down a long left spine on drop.
                            dismantle(lhs);
                            return Err(());
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

    fn parse_prefix(&mut self, depth: usize) -> Result<Expr, ()> {
        match self.bump() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Str(s) => Ok(Expr::Text(s)),
            Token::Bool(b) => Ok(Expr::Bool(b)),
            Token::Ref(r) => Ok(Expr::Ref(r)),
            Token::Ident(name) => {
                if self.bump() != Token::LParen {
                    return Err(());
                }
                let d = Self::enter(depth)?;
                let mut args = Vec::new();
                if *self.peek() == Token::RParen {
                    self.bump();
                } else {
                    loop {
                        args.push(self.parse_expr(0, d)?);
                        match self.bump() {
                            Token::Comma => {}
                            Token::RParen => break,
                            _ => return Err(()),
                        }
                    }
                }
                Ok(Expr::Call {
                    name: name.to_ascii_uppercase(),
                    args,
                })
            }
            Token::LParen => {
                let d = Self::enter(depth)?;
                let inner = self.parse_expr(0, d)?;
                if self.bump() != Token::RParen {
                    return Err(());
                }
                Ok(inner)
            }
            Token::Op(op @ (BinOp::Add | BinOp::Sub)) => {
                let d = Self::enter(depth)?;
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
            _ => Err(()),
        }
    }
}

/// Parses a full token stream; trailing tokens are an error.
pub(crate) fn parse(tokens: Vec<Token>) -> Result<Expr, ()> {
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.parse_expr(0, 0)?;
    if *p.peek() != Token::Eof {
        return Err(());
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

    fn p(s: &str) -> Result<Expr, ()> {
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
    fn depth_boundary_parens() {
        let ok = format!("{}1{}", "(".repeat(64), ")".repeat(64));
        let bad = format!("{}1{}", "(".repeat(65), ")".repeat(65));
        assert!(p(&ok).is_ok());
        assert!(p(&bad).is_err());
    }

    #[test]
    fn depth_boundary_prefix_and_calls() {
        assert!(p(&format!("{}1", "-".repeat(64))).is_ok());
        assert!(p(&format!("{}1", "-".repeat(65))).is_err());
        let call = |n: usize| format!("{}1{}", "ABS(".repeat(n), ")".repeat(n));
        assert!(p(&call(64)).is_ok());
        assert!(p(&call(65)).is_err());
    }
}
