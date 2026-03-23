use logos::{FilterResult, Logos};

use crate::errors::LexerError;

#[derive(Default, Debug, Clone)]
pub struct LexerExtras {
    pub error: Option<LexerError>,
}

#[derive(Logos, Debug, PartialEq, Clone, Eq)]
#[logos(skip r"[ \t\r\n]+")] // Skip whitespace
#[logos(skip(r"//[^\n]*", allow_greedy = true))] // Skip single-line comments
#[logos(extras = LexerExtras)]
pub enum Token {
    // Keywords
    #[token("var")]
    Var,
    #[token("const")]
    Const,
    #[token("return")]
    Return,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("fn")]
    Fn,
    #[token("struct")]
    Struct,
    #[token("self")]
    SelfKw,
    #[token("readonly")]
    Readonly,

    // Type keywords
    #[token("int")]
    Int,
    #[token("char")]
    Char,
    #[token("void")]
    Void,
    #[token("ptr")]
    Ptr,
    #[token("span")]
    Span,

    // Literals
    #[regex(r"[0-9]+", parse_i64)]
    IntLit(i64),
    #[regex(r"'([^'\\]|\\.)'", parse_char)]
    CharLit(char),
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    StringLit(String),

    // Identifier
    #[regex(r"[_\p{XID_Start}]\p{XID_Continue}*", parse_ident)]
    Ident(String),

    // Two-character operators (must be before single-char variants)
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("->")]
    Arrow,
    #[token("::")]
    ColonColon,

    // Single-character operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("!")]
    Bang,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("=")]
    Eq,
    #[token(".")]
    Dot,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,

    // Block comment: skipped if closed, error if unterminated
    #[regex(r"/\*([^*]|\*[^/])*(\*/)?", handle_block_comment)]
    BlockComment,
}

fn parse_string(lex: &mut logos::Lexer<'_, Token>) -> Option<String> {
    let slice = lex.slice();
    let unquoted = &slice[1..slice.len() - 1];
    match process_string_escapes(unquoted) {
        Ok(s) => Some(s),
        Err(ch) => {
            lex.extras.error = Some(LexerError::InvalidEscapeSequence { sequence: ch });
            None
        }
    }
}

fn parse_ident(lex: &mut logos::Lexer<'_, Token>) -> Option<String> {
    Some(lex.slice().to_string())
}

fn parse_char(lex: &mut logos::Lexer<'_, Token>) -> Option<char> {
    let slice = lex.slice();
    let unquoted = &slice[1..slice.len() - 1];
    match process_string_escapes(unquoted) {
        Ok(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(c),
                _ => {
                    lex.extras.error = Some(LexerError::InvalidCharLiteral {
                        literal: slice.to_string(),
                    });
                    None
                }
            }
        }
        Err(ch) => {
            lex.extras.error = Some(LexerError::InvalidEscapeSequence { sequence: ch });
            None
        }
    }
}

fn parse_i64(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    let slice = lex.slice();
    match slice.parse::<i64>() {
        Ok(value) => Some(value),
        Err(err) => {
            lex.extras.error = Some(LexerError::InvalidIntLiteral {
                literal: slice.to_string(),
                reason: err.to_string(),
            });
            None
        }
    }
}

fn handle_block_comment(lex: &mut logos::Lexer<'_, Token>) -> FilterResult<(), ()> {
    if lex.slice().ends_with("*/") {
        FilterResult::Skip
    } else {
        lex.extras.error = Some(LexerError::UnterminatedBlockComment);
        FilterResult::Error(())
    }
}

fn process_string_escapes(s: &str) -> Result<String, char> {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        '0' => result.push('\0'),
                        _ => return Err(escaped),
                    }
                } else {
                    // Trailing backslash
                    result.push('\\');
                }
            }
            _ => result.push(ch),
        }
    }

    Ok(result)
}
