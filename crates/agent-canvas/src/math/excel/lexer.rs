//! Input sanitizing and tokenizing.
#![forbid(unsafe_code)]

use super::parser::BinOp;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Number(f64),
    Str(String),
    Bool(bool),
    /// A function name; always directly followed by `(`.
    Ident(String),
    /// Reference-like word, uppercased. May be invalid (`$A$1`, `SHEET1!A1`, `FOO`);
    /// the evaluator maps invalid references to `#NAME?`.
    Ref(String),
    Op(BinOp),
    LParen,
    RParen,
    Comma,
    Percent,
    Eof,
}

/// Removes non-rendering and tag code points (toolkit-wide rule).
pub(crate) fn sanitize(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\u{200B}'..='\u{200D}'
                    | '\u{2060}'
                    | '\u{FEFF}'
                    | '\u{034F}'
                    | '\u{E0000}'..='\u{E007F}'
            )
        })
        .collect()
}

/// True for `[A-Z]{1,3}[0-9]{1,7}` optionally followed by `:` and another such cell.
pub(crate) fn is_valid_reference(s: &str) -> bool {
    fn cell(part: &str) -> bool {
        let letters = part.chars().take_while(char::is_ascii_uppercase).count();
        let digits = part
            .chars()
            .skip(letters)
            .take_while(char::is_ascii_digit)
            .count();
        (1..=3).contains(&letters)
            && (1..=7).contains(&digits)
            && part.chars().count() == letters + digits
    }
    match s.split_once(':') {
        Some((a, b)) => cell(a) && cell(b),
        None => cell(s),
    }
}

fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$' | '!' | ':')
}

fn is_plain_ident(w: &str) -> bool {
    let mut it = w.chars();
    matches!(it.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && it.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Splits sanitized formula text into tokens, ending with `Token::Eof`.
pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, ()> {
    let chars: Vec<char> = input.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    while let Some(c) = at(i) {
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && at(i + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let start = i;
            while at(i).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
            }
            if at(i) == Some('.') {
                i += 1;
                while at(i).is_some_and(|d| d.is_ascii_digit()) {
                    i += 1;
                }
            }
            if matches!(at(i), Some('e' | 'E')) {
                let mut j = i + 1;
                if matches!(at(j), Some('+' | '-')) {
                    j += 1;
                }
                if at(j).is_some_and(|d| d.is_ascii_digit()) {
                    while at(j).is_some_and(|d| d.is_ascii_digit()) {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = chars.get(start..i).ok_or(())?.iter().collect();
            let n = text.parse::<f64>().map_err(|_| ())?;
            if !n.is_finite() {
                return Err(());
            }
            tokens.push(Token::Number(n));
        } else if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                match at(i) {
                    None => return Err(()),
                    Some('"') if at(i + 1) == Some('"') => {
                        s.push('"');
                        i += 2;
                    }
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some(ch) => {
                        s.push(ch);
                        i += 1;
                    }
                }
            }
            tokens.push(Token::Str(s));
        } else if is_word_start(c) {
            let start = i;
            while at(i).is_some_and(is_word_char) {
                i += 1;
            }
            let word: String = chars.get(start..i).ok_or(())?.iter().collect();
            let call = at(i) == Some('(');
            if call {
                if !is_plain_ident(&word) {
                    return Err(());
                }
                tokens.push(Token::Ident(word));
            } else if word.eq_ignore_ascii_case("TRUE") {
                tokens.push(Token::Bool(true));
            } else if word.eq_ignore_ascii_case("FALSE") {
                tokens.push(Token::Bool(false));
            } else {
                tokens.push(Token::Ref(word.to_ascii_uppercase()));
            }
        } else {
            i += 1;
            let tok = match c {
                '+' => Token::Op(BinOp::Add),
                '-' => Token::Op(BinOp::Sub),
                '*' => Token::Op(BinOp::Mul),
                '/' => Token::Op(BinOp::Div),
                '^' => Token::Op(BinOp::Pow),
                '&' => Token::Op(BinOp::Concat),
                '=' => Token::Op(BinOp::Eq),
                '<' => match at(i) {
                    Some('>') => {
                        i += 1;
                        Token::Op(BinOp::Ne)
                    }
                    Some('=') => {
                        i += 1;
                        Token::Op(BinOp::Le)
                    }
                    _ => Token::Op(BinOp::Lt),
                },
                '>' => {
                    if at(i) == Some('=') {
                        i += 1;
                        Token::Op(BinOp::Ge)
                    } else {
                        Token::Op(BinOp::Gt)
                    }
                }
                '(' => Token::LParen,
                ')' => Token::RParen,
                ',' => Token::Comma,
                '%' => Token::Percent,
                _ => return Err(()),
            };
            tokens.push(tok);
        }
    }
    tokens.push(Token::Eof);
    Ok(tokens)
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
    use crate::math::excel::parser::BinOp;

    fn lex(s: &str) -> Vec<Token> {
        tokenize(s).unwrap()
    }

    #[test]
    fn sanitize_strips_invisible_code_points() {
        let dirty = "1\u{200B}+\u{200C}2\u{200D}\u{2060}\u{FEFF}\u{034F}\u{E0001}\u{E007F}";
        assert_eq!(sanitize(dirty), "1+2");
        assert_eq!(sanitize("a\u{E0000}b"), "ab");
        assert_eq!(sanitize("plain é"), "plain é");
    }

    #[test]
    fn numbers() {
        assert_eq!(lex("12"), vec![Token::Number(12.0), Token::Eof]);
        assert_eq!(lex("1.5"), vec![Token::Number(1.5), Token::Eof]);
        assert_eq!(lex(".5"), vec![Token::Number(0.5), Token::Eof]);
        assert_eq!(lex("1E3"), vec![Token::Number(1000.0), Token::Eof]);
        assert_eq!(lex("1e-2"), vec![Token::Number(0.01), Token::Eof]);
    }

    #[test]
    fn strings_with_escape_and_whitespace() {
        assert_eq!(
            lex("\"a\"\"b\""),
            vec![Token::Str("a\"b".into()), Token::Eof]
        );
        assert_eq!(
            lex("\" a  b \""),
            vec![Token::Str(" a  b ".into()), Token::Eof]
        );
        assert_eq!(lex("\"\""), vec![Token::Str(String::new()), Token::Eof]);
    }

    #[test]
    fn whitespace_skipped_between_tokens() {
        assert_eq!(
            lex(" 1 \n+\t2 "),
            vec![
                Token::Number(1.0),
                Token::Op(BinOp::Add),
                Token::Number(2.0),
                Token::Eof
            ]
        );
    }

    #[test]
    fn bools_idents_refs() {
        assert_eq!(lex("TRUE"), vec![Token::Bool(true), Token::Eof]);
        assert_eq!(lex("false"), vec![Token::Bool(false), Token::Eof]);
        assert_eq!(
            lex("sum(1)"),
            vec![
                Token::Ident("sum".into()),
                Token::LParen,
                Token::Number(1.0),
                Token::RParen,
                Token::Eof
            ]
        );
        assert_eq!(lex("A1"), vec![Token::Ref("A1".into()), Token::Eof]);
        assert_eq!(lex("a1:B3"), vec![Token::Ref("A1:B3".into()), Token::Eof]);
        assert_eq!(lex("$A$1"), vec![Token::Ref("$A$1".into()), Token::Eof]);
        assert_eq!(
            lex("Sheet1!a1"),
            vec![Token::Ref("SHEET1!A1".into()), Token::Eof]
        );
    }

    #[test]
    fn reference_validity() {
        assert!(is_valid_reference("A1"));
        assert!(is_valid_reference("ABC1234567"));
        assert!(is_valid_reference("A1:B3"));
        assert!(!is_valid_reference("ABCD1"));
        assert!(!is_valid_reference("A12345678"));
        assert!(!is_valid_reference("A"));
        assert!(!is_valid_reference("$A$1"));
        assert!(!is_valid_reference("SHEET1!A1"));
        assert!(!is_valid_reference("A1:"));
        assert!(!is_valid_reference(""));
    }

    #[test]
    fn operators() {
        let ops = lex("+ - * / ^ & = <> < > <= >=");
        let expect = [
            BinOp::Add,
            BinOp::Sub,
            BinOp::Mul,
            BinOp::Div,
            BinOp::Pow,
            BinOp::Concat,
            BinOp::Eq,
            BinOp::Ne,
            BinOp::Lt,
            BinOp::Gt,
            BinOp::Le,
            BinOp::Ge,
        ];
        assert_eq!(ops.len(), expect.len() + 1);
        for (t, e) in ops.iter().zip(expect) {
            assert_eq!(t, &Token::Op(e));
        }
        assert_eq!(
            lex("(1,2)%"),
            vec![
                Token::LParen,
                Token::Number(1.0),
                Token::Comma,
                Token::Number(2.0),
                Token::RParen,
                Token::Percent,
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexer_errors() {
        assert!(tokenize("\"abc").is_err());
        assert!(tokenize("1 # 2").is_err());
        assert!(tokenize("'a'").is_err());
        assert!(tokenize("1e999").is_err());
        assert!(tokenize("é").is_err());
        assert!(tokenize("{1}").is_err());
    }
}
