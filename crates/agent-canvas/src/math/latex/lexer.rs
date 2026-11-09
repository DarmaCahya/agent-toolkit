//! Whitelist lexer: characters to tokens with char-index positions.

use super::error::MathError;

/// Token kinds (data-model "Lexer").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Letter(char),
    Digit(char),
    /// One of `+ - = < > ( ) ,`.
    Symbol(char),
    /// Command name after the backslash.
    Command(String),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Caret,
    Underscore,
}

/// A token with its char-index position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

/// Tokenises `input`. Any character outside the whitelist is rejected.
pub(crate) fn lex(input: &str) -> Result<Vec<Token>, MathError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    while let Some(&c) = chars.get(i) {
        let pos = i;
        let kind = match c {
            c if c.is_ascii_whitespace() => {
                i = i.saturating_add(1);
                continue;
            }
            c if c.is_ascii_alphabetic() => TokenKind::Letter(c),
            c if c.is_ascii_digit() => TokenKind::Digit(c),
            '+' | '-' | '=' | '<' | '>' | '(' | ')' | ',' => TokenKind::Symbol(c),
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '^' => TokenKind::Caret,
            '_' => TokenKind::Underscore,
            '\\' => {
                let name: String = chars
                    .iter()
                    .skip(i.saturating_add(1))
                    .take_while(|ch| ch.is_ascii_alphabetic())
                    .collect();
                if name.is_empty() {
                    match chars.get(i.saturating_add(1)) {
                        // Explicit spacing: exactly one char is consumed.
                        Some(&ch @ (',' | ':' | ';' | '!' | ' ')) => {
                            tokens.push(Token {
                                kind: TokenKind::Command(ch.to_string()),
                                pos,
                            });
                            i = i.saturating_add(2);
                            continue;
                        }
                        None => {
                            return Err(MathError::UnexpectedEof {
                                pos: chars.len(),
                                expected: "a command name after `\\`".into(),
                            })
                        }
                        // Other single-symbol commands are not supported.
                        Some(&ch) if ch.is_ascii_punctuation() => {
                            return Err(MathError::Unsupported {
                                pos,
                                name: ch.to_string(),
                                hint: "only the spacing commands \\, \\: \\; \\! \\quad \\qquad \
                                       and backslash-space are supported; other symbol commands \
                                       are out of scope for v1"
                                    .into(),
                            })
                        }
                        Some(&ch) => {
                            return Err(MathError::UnexpectedChar {
                                pos: i.saturating_add(1),
                                ch,
                            })
                        }
                    }
                }
                i = i.saturating_add(name.chars().count());
                TokenKind::Command(name)
            }
            ch => return Err(MathError::UnexpectedChar { pos, ch }),
        };
        tokens.push(Token { kind, pos });
        i = i.saturating_add(1);
    }
    Ok(tokens)
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

    fn kinds(s: &str) -> Vec<TokenKind> {
        lex(s).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_quadratic_formula() {
        let toks = lex(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}").unwrap();
        let k: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(k[0], TokenKind::Command("frac".into()));
        assert_eq!(k[1], TokenKind::LBrace);
        assert_eq!(k[2], TokenKind::Symbol('-'));
        assert_eq!(k[3], TokenKind::Letter('b'));
        assert_eq!(k[4], TokenKind::Command("pm".into()));
        assert_eq!(k[5], TokenKind::Command("sqrt".into()));
        assert_eq!(k[8], TokenKind::Caret);
        assert_eq!(k[9], TokenKind::Digit('2'));
        assert_eq!(toks[0].pos, 0);
        assert_eq!(toks[2].pos, 6);
        assert_eq!(toks[4].pos, 9); // `\pm` after "\frac{-b "
    }

    #[test]
    fn whitespace_skipped_and_punctuation() {
        assert_eq!(
            kinds(" f ( x , y ) "),
            vec![
                TokenKind::Letter('f'),
                TokenKind::Symbol('('),
                TokenKind::Letter('x'),
                TokenKind::Symbol(','),
                TokenKind::Letter('y'),
                TokenKind::Symbol(')'),
            ]
        );
        assert_eq!(
            kinds("[_]"),
            vec![
                TokenKind::LBracket,
                TokenKind::Underscore,
                TokenKind::RBracket
            ]
        );
    }

    #[test]
    fn rejects_hostile_chars_with_position() {
        for (s, pos, ch) in [
            ("$", 0, '$'),
            ("a%", 1, '%'),
            ("#", 0, '#'),
            ("&", 0, '&'),
            ("~", 0, '~'),
            ("é", 0, 'é'),
            ("a\u{200B}", 1, '\u{200B}'),
            ("a\u{E0041}", 1, '\u{E0041}'),
            ("a\u{0001}", 1, '\u{0001}'),
        ] {
            assert_eq!(lex(s), Err(MathError::UnexpectedChar { pos, ch }), "{s:?}");
        }
    }

    #[test]
    fn positions_are_char_indices() {
        // `é` is two bytes; `$` is the 3rd char (index 2).
        assert_eq!(lex("éa$").unwrap_err().position(), Some(0));
        assert_eq!(lex("a\u{E0041}$").unwrap_err().position(), Some(1));
        assert_eq!(
            lex("ab$"),
            Err(MathError::UnexpectedChar { pos: 2, ch: '$' })
        );
    }

    #[test]
    fn trailing_backslash_is_eof() {
        assert!(matches!(
            lex(r"a\"),
            Err(MathError::UnexpectedEof { pos: 2, .. })
        ));
    }
}
