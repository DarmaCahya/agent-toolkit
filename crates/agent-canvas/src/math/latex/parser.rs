//! Recursive-descent parser: tokens to an AST, with a bounded nesting depth.

use super::atoms::{math_char, AtomClass};
use super::error::MathError;
use super::lexer::{Token, TokenKind};
use super::MAX_DEPTH;

/// A resolved symbol: final glyph code point plus its TeX class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sym {
    pub ch: char,
    pub class: AtomClass,
}

/// Large operators supported in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BigOpKind {
    Sum,
    Int,
    Prod,
}

impl BigOpKind {
    pub(crate) fn glyph_char(self) -> char {
        match self {
            Self::Sum => '\u{2211}',
            Self::Int => '\u{222B}',
            Self::Prod => '\u{220F}',
        }
    }
}

/// AST node with the char-index position of its first token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    pub kind: NodeKind,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NodeKind {
    Symbol(Sym),
    Group(Vec<Node>),
    Scripts {
        base: Box<Node>,
        sup: Option<Box<Node>>,
        sub: Option<Box<Node>>,
    },
    Frac {
        num: Box<Node>,
        den: Box<Node>,
    },
    Sqrt {
        index: Option<Box<Node>>,
        body: Box<Node>,
    },
    BigOp {
        op: BigOpKind,
        sup: Option<Box<Node>>,
        sub: Option<Box<Node>>,
    },
}

const GREEK_LOWER: [(&str, u32); 24] = [
    ("alpha", 0),
    ("beta", 1),
    ("gamma", 2),
    ("delta", 3),
    ("epsilon", 4),
    ("zeta", 5),
    ("eta", 6),
    ("theta", 7),
    ("iota", 8),
    ("kappa", 9),
    ("lambda", 10),
    ("mu", 11),
    ("nu", 12),
    ("xi", 13),
    ("pi", 15),
    ("rho", 16),
    ("sigma", 18),
    ("tau", 19),
    ("upsilon", 20),
    ("phi", 21),
    ("chi", 22),
    ("psi", 23),
    ("omega", 24),
    ("varsigma", 17),
];

const GREEK_UPPER: [(&str, char); 11] = [
    ("Gamma", '\u{0393}'),
    ("Delta", '\u{0394}'),
    ("Theta", '\u{0398}'),
    ("Lambda", '\u{039B}'),
    ("Xi", '\u{039E}'),
    ("Pi", '\u{03A0}'),
    ("Sigma", '\u{03A3}'),
    ("Upsilon", '\u{03A5}'),
    ("Phi", '\u{03A6}'),
    ("Psi", '\u{03A8}'),
    ("Omega", '\u{03A9}'),
];

const OPERATOR_SYMBOLS: [(&str, char, AtomClass); 10] = [
    ("ne", '\u{2260}', AtomClass::Rel),
    ("pm", '\u{00B1}', AtomClass::Bin),
    ("cdot", '\u{00B7}', AtomClass::Bin),
    ("times", '\u{00D7}', AtomClass::Bin),
    ("leq", '\u{2264}', AtomClass::Rel),
    ("le", '\u{2264}', AtomClass::Rel),
    ("geq", '\u{2265}', AtomClass::Rel),
    ("ge", '\u{2265}', AtomClass::Rel),
    ("neq", '\u{2260}', AtomClass::Rel),
    ("infty", '\u{221E}', AtomClass::Ord),
];

const UNSUPPORTED: [&str; 27] = [
    "left",
    "right",
    "text",
    "begin",
    "end",
    "hat",
    "bar",
    "vec",
    "tilde",
    "dot",
    "newcommand",
    "renewcommand",
    "def",
    "mathbf",
    "mathrm",
    "mathit",
    "mathcal",
    "mathbb",
    "color",
    "displaystyle",
    "textstyle",
    "over",
    "choose",
    "lim",
    "log",
    "sin",
    "cos",
];

fn symbol_command(name: &str) -> Option<Sym> {
    if let Some((_, off)) = GREEK_LOWER.iter().find(|(n, _)| *n == name) {
        let ch = char::from_u32(0x1D6FC_u32.saturating_add(*off))?;
        return Some(Sym {
            ch,
            class: AtomClass::Ord,
        });
    }
    if let Some((_, ch)) = GREEK_UPPER.iter().find(|(n, _)| *n == name) {
        return Some(Sym {
            ch: *ch,
            class: AtomClass::Ord,
        });
    }
    OPERATOR_SYMBOLS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, ch, class)| Sym {
            ch: *ch,
            class: *class,
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Eof,
    Brace,
    Bracket,
}

struct Parser<'a> {
    tokens: &'a [Token],
    idx: usize,
    end: usize,
}

/// Parses `tokens` (from an input of `end` chars) into a root `Group`.
pub(crate) fn parse(tokens: &[Token], end: usize) -> Result<Node, MathError> {
    let mut p = Parser {
        tokens,
        idx: 0,
        end,
    };
    let items = p.parse_list(0, Stop::Eof)?;
    Ok(Node {
        kind: NodeKind::Group(items),
        pos: 0,
    })
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.idx)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.idx).cloned();
        if t.is_some() {
            self.idx = self.idx.saturating_add(1);
        }
        t
    }

    fn parse_list(&mut self, depth: usize, stop: Stop) -> Result<Vec<Node>, MathError> {
        let mut items = Vec::new();
        loop {
            let Some(tok) = self.peek() else {
                return match stop {
                    Stop::Eof => Ok(items),
                    Stop::Brace => Err(self.eof("`}` to close the group")),
                    Stop::Bracket => Err(self.eof("`]` to close the root index")),
                };
            };
            match (&tok.kind, stop) {
                (TokenKind::RBrace, Stop::Brace) | (TokenKind::RBracket, Stop::Bracket) => {
                    return Ok(items);
                }
                (TokenKind::RBrace, _) => {
                    return Err(MathError::UnexpectedChar {
                        pos: tok.pos,
                        ch: '}',
                    });
                }
                (TokenKind::RBracket, _) => {
                    return Err(MathError::UnexpectedChar {
                        pos: tok.pos,
                        ch: ']',
                    });
                }
                _ => items.push(self.parse_item(depth)?),
            }
        }
    }

    fn eof(&self, expected: &str) -> MathError {
        MathError::UnexpectedEof {
            pos: self.end,
            expected: expected.into(),
        }
    }

    fn parse_item(&mut self, depth: usize) -> Result<Node, MathError> {
        let start = self.peek().map_or(self.end, |t| t.pos);
        let mut base = match self.peek().map(|t| &t.kind) {
            Some(TokenKind::Caret | TokenKind::Underscore) => Node {
                kind: NodeKind::Group(Vec::new()),
                pos: start,
            },
            _ => self.parse_nucleus(depth)?,
        };
        let mut sup: Option<Box<Node>> = None;
        let mut sub: Option<Box<Node>> = None;
        loop {
            let (is_sup, pos) = match self.peek() {
                Some(Token {
                    kind: TokenKind::Caret,
                    pos,
                }) => (true, *pos),
                Some(Token {
                    kind: TokenKind::Underscore,
                    pos,
                }) => (false, *pos),
                _ => break,
            };
            let slot = if is_sup { &mut sup } else { &mut sub };
            if slot.is_some() {
                return Err(MathError::DoubleScript {
                    pos,
                    kind: if is_sup { "superscript" } else { "subscript" },
                });
            }
            self.bump();
            let arg = self.parse_script_arg(depth, pos, is_sup)?;
            if is_sup {
                sup = Some(Box::new(arg));
            } else {
                sub = Some(Box::new(arg));
            }
        }
        if sup.is_none() && sub.is_none() {
            return Ok(base);
        }
        if let NodeKind::BigOp {
            sup: op_sup,
            sub: op_sub,
            ..
        } = &mut base.kind
        {
            *op_sup = sup;
            *op_sub = sub;
            return Ok(base);
        }
        let pos = base.pos;
        Ok(Node {
            kind: NodeKind::Scripts {
                base: Box::new(base),
                sup,
                sub,
            },
            pos,
        })
    }

    fn parse_script_arg(
        &mut self,
        depth: usize,
        marker_pos: usize,
        is_sup: bool,
    ) -> Result<Node, MathError> {
        match self.peek().map(|t| &t.kind) {
            None => Err(self.eof("a script argument")),
            Some(
                TokenKind::Caret
                | TokenKind::Underscore
                | TokenKind::RBrace
                | TokenKind::RBracket
                | TokenKind::LBracket,
            ) => Err(MathError::MissingArgument {
                pos: marker_pos,
                command: if is_sup { "^" } else { "_" }.into(),
            }),
            Some(_) => self.parse_nucleus(depth),
        }
    }

    fn parse_nucleus(&mut self, depth: usize) -> Result<Node, MathError> {
        let Some(tok) = self.bump() else {
            return Err(self.eof("a math element"));
        };
        let pos = tok.pos;
        let sym = |ch: char, class: AtomClass| Node {
            kind: NodeKind::Symbol(Sym { ch, class }),
            pos,
        };
        match tok.kind {
            TokenKind::Letter(c) => Ok(sym(math_char(c), AtomClass::Ord)),
            TokenKind::Digit(c) => Ok(sym(c, AtomClass::Ord)),
            TokenKind::Symbol(c) => {
                let class = match c {
                    '+' | '-' => AtomClass::Bin,
                    '=' | '<' | '>' => AtomClass::Rel,
                    '(' => AtomClass::Open,
                    ')' => AtomClass::Close,
                    _ => AtomClass::Punct,
                };
                Ok(sym(math_char(c), class))
            }
            TokenKind::LBrace => self.parse_group(depth, pos),
            TokenKind::Command(name) => self.parse_command(&name, pos, depth),
            TokenKind::LBracket => Err(MathError::UnexpectedChar { pos, ch: '[' }),
            // Callers filter these out before reaching here.
            TokenKind::RBrace => Err(MathError::UnexpectedChar { pos, ch: '}' }),
            TokenKind::RBracket => Err(MathError::UnexpectedChar { pos, ch: ']' }),
            TokenKind::Caret => Err(MathError::UnexpectedChar { pos, ch: '^' }),
            TokenKind::Underscore => Err(MathError::UnexpectedChar { pos, ch: '_' }),
        }
    }

    /// Parses a group body after its `{` has been consumed.
    fn parse_group(&mut self, depth: usize, open_pos: usize) -> Result<Node, MathError> {
        let inner = self.enter(depth, open_pos)?;
        let items = self.parse_list(inner, Stop::Brace)?;
        self.bump(); // the closing `}` checked by parse_list
        Ok(Node {
            kind: NodeKind::Group(items),
            pos: open_pos,
        })
    }

    fn enter(&self, depth: usize, pos: usize) -> Result<usize, MathError> {
        let inner = depth.saturating_add(1);
        if inner > MAX_DEPTH {
            Err(MathError::NestingTooDeep {
                pos,
                max: MAX_DEPTH,
            })
        } else {
            Ok(inner)
        }
    }

    fn braced_arg(&mut self, depth: usize, name: &str, cmd_pos: usize) -> Result<Node, MathError> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::LBrace,
                pos,
            }) => {
                let pos = *pos;
                self.bump();
                self.parse_group(depth, pos)
            }
            _ => Err(MathError::MissingArgument {
                pos: cmd_pos,
                command: name.into(),
            }),
        }
    }

    fn parse_command(&mut self, name: &str, pos: usize, depth: usize) -> Result<Node, MathError> {
        match name {
            "frac" => {
                let num = self.braced_arg(depth, name, pos)?;
                let den = self.braced_arg(depth, name, pos)?;
                Ok(Node {
                    kind: NodeKind::Frac {
                        num: Box::new(num),
                        den: Box::new(den),
                    },
                    pos,
                })
            }
            "sqrt" => {
                let index = if let Some(Token {
                    kind: TokenKind::LBracket,
                    pos: bpos,
                }) = self.peek()
                {
                    let bpos = *bpos;
                    self.bump();
                    let inner = self.enter(depth, bpos)?;
                    let items = self.parse_list(inner, Stop::Bracket)?;
                    self.bump(); // the closing `]`
                    Some(Box::new(Node {
                        kind: NodeKind::Group(items),
                        pos: bpos,
                    }))
                } else {
                    None
                };
                let body = self.braced_arg(depth, name, pos)?;
                Ok(Node {
                    kind: NodeKind::Sqrt {
                        index,
                        body: Box::new(body),
                    },
                    pos,
                })
            }
            "sum" | "int" | "prod" => {
                let op = match name {
                    "sum" => BigOpKind::Sum,
                    "int" => BigOpKind::Int,
                    _ => BigOpKind::Prod,
                };
                Ok(Node {
                    kind: NodeKind::BigOp {
                        op,
                        sup: None,
                        sub: None,
                    },
                    pos,
                })
            }
            _ => {
                if let Some(sym) = symbol_command(name) {
                    Ok(Node {
                        kind: NodeKind::Symbol(sym),
                        pos,
                    })
                } else if UNSUPPORTED.contains(&name) {
                    Err(MathError::Unsupported {
                        pos,
                        name: name.into(),
                        hint: "out of scope for v1; see issue #9 later parts".into(),
                    })
                } else {
                    Err(MathError::UnknownCommand {
                        pos,
                        name: name.into(),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use super::*;
    use crate::math::latex::lexer::lex;

    fn p(s: &str) -> Result<Vec<Node>, MathError> {
        let toks = lex(s)?;
        match parse(&toks, s.chars().count())?.kind {
            NodeKind::Group(items) => Ok(items),
            _ => unreachable!(),
        }
    }

    #[test]
    fn quadratic_formula_shape() {
        let items = p(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}").unwrap();
        assert_eq!(items.len(), 1);
        let NodeKind::Frac { num, den } = &items[0].kind else {
            panic!("expected frac")
        };
        let NodeKind::Group(n) = &num.kind else {
            panic!()
        };
        assert_eq!(n.len(), 4); // - b \pm \sqrt
        assert!(matches!(n[3].kind, NodeKind::Sqrt { index: None, .. }));
        let NodeKind::Group(d) = &den.kind else {
            panic!()
        };
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn superscript_shapes() {
        let items = p("b^2").unwrap();
        let NodeKind::Scripts { base, sup, sub } = &items[0].kind else {
            panic!()
        };
        assert!(matches!(base.kind, NodeKind::Symbol(_)));
        assert!(sup.is_some() && sub.is_none());
        // single-token script argument
        let items = p("x^23").unwrap();
        assert_eq!(items.len(), 2);
        let NodeKind::Scripts { sup, .. } = &items[0].kind else {
            panic!()
        };
        let NodeKind::Symbol(s) = &sup.as_ref().unwrap().kind else {
            panic!()
        };
        assert_eq!(s.ch, '2');
        assert!(matches!(&items[1].kind, NodeKind::Symbol(s) if s.ch == '3'));
        // both scripts
        let items = p("x_i^2").unwrap();
        let NodeKind::Scripts { sup, sub, .. } = &items[0].kind else {
            panic!()
        };
        assert!(sup.is_some() && sub.is_some());
    }

    #[test]
    fn sqrt_index_and_symbols() {
        let items = p(r"\sqrt[3]{x}").unwrap();
        let NodeKind::Sqrt { index, .. } = &items[0].kind else {
            panic!()
        };
        assert!(index.is_some());
        let items = p(r"\pm\alpha\Gamma\infty\leq").unwrap();
        let chars: Vec<char> = items
            .iter()
            .map(|n| match &n.kind {
                NodeKind::Symbol(s) => s.ch,
                _ => '?',
            })
            .collect();
        assert_eq!(
            chars,
            vec!['\u{B1}', '\u{1D6FC}', '\u{393}', '\u{221E}', '\u{2264}']
        );
        let NodeKind::Symbol(s) = &items[0].kind else {
            panic!()
        };
        assert_eq!(s.class, AtomClass::Bin);
    }

    #[test]
    fn big_operators_take_scripts() {
        let items = p(r"\sum_{i=0}^{n} i").unwrap();
        assert_eq!(items.len(), 2);
        let NodeKind::BigOp { op, sup, sub } = &items[0].kind else {
            panic!()
        };
        assert_eq!(*op, BigOpKind::Sum);
        assert!(sup.is_some() && sub.is_some());
        assert!(matches!(
            p(r"\int").unwrap()[0].kind,
            NodeKind::BigOp {
                op: BigOpKind::Int,
                ..
            }
        ));
        assert!(matches!(
            p(r"\prod").unwrap()[0].kind,
            NodeKind::BigOp {
                op: BigOpKind::Prod,
                ..
            }
        ));
    }

    #[test]
    fn leading_script_gets_empty_base() {
        let items = p("^2").unwrap();
        let NodeKind::Scripts { base, .. } = &items[0].kind else {
            panic!()
        };
        assert_eq!(base.kind, NodeKind::Group(Vec::new()));
    }

    #[test]
    fn errors_with_positions() {
        assert!(matches!(
            p(r"\foo"),
            Err(MathError::UnknownCommand { pos: 0, ref name }) if name == "foo"
        ));
        assert!(matches!(
            p(r"\left"),
            Err(MathError::Unsupported { pos: 0, .. })
        ));
        assert_eq!(
            p("x^2^3"),
            Err(MathError::DoubleScript {
                pos: 3,
                kind: "superscript"
            })
        );
        assert_eq!(
            p("x_1_2"),
            Err(MathError::DoubleScript {
                pos: 3,
                kind: "subscript"
            })
        );
        assert!(matches!(
            p(r"\frac{a}{"),
            Err(MathError::UnexpectedEof { pos: 9, .. })
        ));
        assert!(matches!(
            p(r"\frac{a}"),
            Err(MathError::MissingArgument { pos: 0, .. })
        ));
        assert!(matches!(
            p(r"\sqrt"),
            Err(MathError::MissingArgument { pos: 0, .. })
        ));
        assert!(matches!(
            p("x^"),
            Err(MathError::UnexpectedEof { pos: 2, .. })
        ));
        assert!(matches!(
            p("}"),
            Err(MathError::UnexpectedChar { ch: '}', .. })
        ));
    }

    #[test]
    fn depth_limit() {
        let ok = format!("{}a{}", "{".repeat(64), "}".repeat(64));
        assert!(p(&ok).is_ok());
        let bad = format!("{}a{}", "{".repeat(65), "}".repeat(65));
        assert_eq!(p(&bad), Err(MathError::NestingTooDeep { pos: 64, max: 64 }));
    }
}
