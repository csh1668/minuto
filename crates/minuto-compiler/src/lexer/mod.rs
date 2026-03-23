pub mod token;

use logos::Logos;
use token::Token;

use crate::common::Span;
use crate::diagnostic::Diagnostic;
use crate::errors::LexerError;

pub struct Lexer<'source> {
    source: &'source str,
    inner: logos::Lexer<'source, Token>,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            source,
            inner: Token::lexer(source),
        }
    }

    pub fn next_token(&mut self) -> Option<Result<(Token, Span), Diagnostic>> {
        let result = self.inner.next()?;
        let span = Span::from(self.inner.span());

        match result {
            Ok(token) => Some(Ok((token, span))),
            Err(()) => {
                let range = span.clone().into_range();
                let error = self.inner.extras.error.take().unwrap_or_else(|| {
                    let fragment = &self.source[range];
                    if fragment.starts_with('"') {
                        LexerError::UnterminatedString
                    } else if fragment.starts_with("/*") {
                        LexerError::UnterminatedBlockComment
                    } else {
                        LexerError::UnexpectedCharacter {
                            character: fragment.to_string(),
                        }
                    }
                });

                Some(Err(Diagnostic::from((error, span))))
            }
        }
    }

    pub fn tokenize(&mut self) -> (Vec<(Token, Span)>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while let Some(result) = self.next_token() {
            match result {
                Ok(token) => tokens.push(token),
                Err(diag) => errors.push(diag),
            }
        }

        (tokens, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use token::Token;

    /// Helper: Lex the source and collect tokens, panicking on any lexer error
    fn lex_ok(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        while let Some(result) = lexer.next_token() {
            tokens.push(result.expect("unexpected lexer error").0);
        }
        tokens
    }

    /// Helper: Lex the source and collect tokens with spans, panicking on any lexer error
    fn lex_ok_with_spans(source: &str) -> Vec<(Token, Span)> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        while let Some(result) = lexer.next_token() {
            tokens.push(result.expect("unexpected lexer error"));
        }
        tokens
    }

    /// Helper: Lex the source and return the first lexer error encountered, panicking if no error occurs
    fn lex_first_error(source: &str) -> Diagnostic {
        let mut lexer = Lexer::new(source);
        while let Some(result) = lexer.next_token() {
            if let Err(diag) = result {
                return diag;
            }
        }
        panic!("expected a lexer error");
    }

    // ── Keywords ──

    #[test]
    fn keywords() {
        let tokens =
            lex_ok("var const return if else while break continue fn struct self readonly");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Const,
                Token::Return,
                Token::If,
                Token::Else,
                Token::While,
                Token::Break,
                Token::Continue,
                Token::Fn,
                Token::Struct,
                Token::SelfKw,
                Token::Readonly,
            ]
        );
    }

    // ── Type keywords ──

    #[test]
    fn type_keywords() {
        let tokens = lex_ok("int char void ptr span");
        assert_eq!(
            tokens,
            vec![
                Token::Int,
                Token::Char,
                Token::Void,
                Token::Ptr,
                Token::Span,
            ]
        );
    }

    // ── Integer literals ──

    #[test]
    fn integer_literals() {
        let tokens = lex_ok("0 42 1024");
        assert_eq!(
            tokens,
            vec![Token::IntLit(0), Token::IntLit(42), Token::IntLit(1024),]
        );
    }

    #[test]
    fn integer_overflow() {
        let diag = lex_first_error("var x = 999999999999999999999;");
        assert!(diag.message.contains("invalid integer literal"));
        assert_eq!(diag.code, Some("E0002"));
    }

    // ── Char literals ──

    #[test]
    fn char_literal() {
        let tokens = lex_ok("'a' 'Z' '0'");
        assert_eq!(
            tokens,
            vec![
                Token::CharLit('a'),
                Token::CharLit('Z'),
                Token::CharLit('0'),
            ]
        );
    }

    #[test]
    fn char_literal_escape() {
        let tokens = lex_ok(r"'\n' '\t' '\\'");
        assert_eq!(
            tokens,
            vec![
                Token::CharLit('\n'),
                Token::CharLit('\t'),
                Token::CharLit('\\'),
            ]
        );
    }

    // ── String literals ──

    #[test]
    fn string_literal() {
        let tokens = lex_ok(r#""hello""#);
        assert_eq!(tokens, vec![Token::StringLit("hello".to_string())]);
    }

    #[test]
    fn string_literal_escapes() {
        let tokens = lex_ok(r#""hello\nworld""#);
        assert_eq!(tokens, vec![Token::StringLit("hello\nworld".to_string())]);
    }

    #[test]
    fn string_literal_empty() {
        let tokens = lex_ok(r#""""#);
        assert_eq!(tokens, vec![Token::StringLit(String::new())]);
    }

    // ── Identifiers ──

    #[test]
    fn identifiers() {
        let tokens = lex_ok("foo _bar baz123");
        assert_eq!(
            tokens,
            vec![
                Token::Ident("foo".into()),
                Token::Ident("_bar".into()),
                Token::Ident("baz123".into()),
            ]
        );
    }

    #[test]
    fn keyword_prefix_is_ident() {
        // "variable"은 "var" + ident가 아니라 하나의 ident여야 함
        let tokens = lex_ok("variable");
        assert_eq!(tokens, vec![Token::Ident("variable".into())]);
    }

    // ── Two-character operators ──

    #[test]
    fn two_char_operators() {
        let tokens = lex_ok("<< >> == != <= >= && || -> ::");
        assert_eq!(
            tokens,
            vec![
                Token::Shl,
                Token::Shr,
                Token::EqEq,
                Token::NotEq,
                Token::LtEq,
                Token::GtEq,
                Token::AndAnd,
                Token::OrOr,
                Token::Arrow,
                Token::ColonColon,
            ]
        );
    }

    // ── Single-character operators ──

    #[test]
    fn single_char_operators() {
        let tokens = lex_ok("+ - * / % & | ^ ~ ! < > = .");
        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Percent,
                Token::Ampersand,
                Token::Pipe,
                Token::Caret,
                Token::Tilde,
                Token::Bang,
                Token::Lt,
                Token::Gt,
                Token::Eq,
                Token::Dot,
            ]
        );
    }

    // ── Delimiters ──

    #[test]
    fn delimiters() {
        let tokens = lex_ok("( ) { } [ ] , : ;");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::RBrace,
                Token::LBracket,
                Token::RBracket,
                Token::Comma,
                Token::Colon,
                Token::Semicolon,
            ]
        );
    }

    // ── Spans ──

    #[test]
    fn span_tracking() {
        let result = lex_ok_with_spans("var x = 42;");
        assert_eq!(result[0], (Token::Var, Span::from(0..3)));
        assert_eq!(result[1], (Token::Ident("x".into()), Span::from(4..5)));
        assert_eq!(result[2], (Token::Eq, Span::from(6..7)));
        assert_eq!(result[3], (Token::IntLit(42), Span::from(8..10)));
        assert_eq!(result[4], (Token::Semicolon, Span::from(10..11)));
    }

    // ── Comments ──

    #[test]
    fn single_line_comment() {
        let tokens = lex_ok("var x // this is a comment\n= 5;");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Ident("x".into()),
                Token::Eq,
                Token::IntLit(5),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn multi_line_comment() {
        let tokens = lex_ok("var /* this is\na comment */ x;");
        assert_eq!(
            tokens,
            vec![Token::Var, Token::Ident("x".into()), Token::Semicolon,]
        );
    }

    // ── Whitespace ──

    #[test]
    fn whitespace_is_skipped() {
        let tokens = lex_ok("  var\t\tx\n\n=\r\n5 ;");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Ident("x".into()),
                Token::Eq,
                Token::IntLit(5),
                Token::Semicolon,
            ]
        );
    }

    // ── Compound expressions (spec examples) ──

    #[test]
    fn var_declaration() {
        let tokens = lex_ok("var x: int = 5;");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Ident("x".into()),
                Token::Colon,
                Token::Int,
                Token::Eq,
                Token::IntLit(5),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn fn_declaration() {
        let tokens = lex_ok("fn add(a: int, b: int) -> int { return a + b; }");
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Ident("add".into()),
                Token::LParen,
                Token::Ident("a".into()),
                Token::Colon,
                Token::Int,
                Token::Comma,
                Token::Ident("b".into()),
                Token::Colon,
                Token::Int,
                Token::RParen,
                Token::Arrow,
                Token::Int,
                Token::LBrace,
                Token::Return,
                Token::Ident("a".into()),
                Token::Plus,
                Token::Ident("b".into()),
                Token::Semicolon,
                Token::RBrace,
            ]
        );
    }

    #[test]
    fn struct_method_call() {
        let tokens = lex_ok("c.increment();");
        assert_eq!(
            tokens,
            vec![
                Token::Ident("c".into()),
                Token::Dot,
                Token::Ident("increment".into()),
                Token::LParen,
                Token::RParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn static_method_call() {
        let tokens = lex_ok("Counter::new()");
        assert_eq!(
            tokens,
            vec![
                Token::Ident("Counter".into()),
                Token::ColonColon,
                Token::Ident("new".into()),
                Token::LParen,
                Token::RParen,
            ]
        );
    }

    #[test]
    fn pointer_operations() {
        let tokens = lex_ok("*p = &x; p->y");
        assert_eq!(
            tokens,
            vec![
                Token::Star,
                Token::Ident("p".into()),
                Token::Eq,
                Token::Ampersand,
                Token::Ident("x".into()),
                Token::Semicolon,
                Token::Ident("p".into()),
                Token::Arrow,
                Token::Ident("y".into()),
            ]
        );
    }

    #[test]
    fn span_type_and_alloc() {
        let tokens = lex_ok("var arr: span<int> = alloc<int>(10);");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Ident("arr".into()),
                Token::Colon,
                Token::Span,
                Token::Lt,
                Token::Int,
                Token::Gt,
                Token::Eq,
                Token::Ident("alloc".into()),
                Token::Lt,
                Token::Int,
                Token::Gt,
                Token::LParen,
                Token::IntLit(10),
                Token::RParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn span_static_method() {
        // span::new(p, 10) — type keyword used as static method receiver
        let tokens = lex_ok("var arr: span<int> = span::new(p, 10);");
        assert_eq!(
            tokens,
            vec![
                Token::Var,
                Token::Ident("arr".into()),
                Token::Colon,
                Token::Span,
                Token::Lt,
                Token::Int,
                Token::Gt,
                Token::Eq,
                Token::Span,
                Token::ColonColon,
                Token::Ident("new".into()),
                Token::LParen,
                Token::Ident("p".into()),
                Token::Comma,
                Token::IntLit(10),
                Token::RParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn std_print() {
        let tokens = lex_ok(r#"std::print("{}\n", x);"#);
        assert_eq!(
            tokens,
            vec![
                Token::Ident("std".into()),
                Token::ColonColon,
                Token::Ident("print".into()),
                Token::LParen,
                Token::StringLit("{}\n".into()),
                Token::Comma,
                Token::Ident("x".into()),
                Token::RParen,
                Token::Semicolon,
            ]
        );
    }

    // ── Builtin identifiers (alloc, free are not keywords) ──

    #[test]
    fn alloc_free_are_identifiers() {
        let tokens = lex_ok("alloc free");
        assert_eq!(
            tokens,
            vec![Token::Ident("alloc".into()), Token::Ident("free".into()),]
        );
    }

    // ── Error cases ──

    #[test]
    fn error_unexpected_character() {
        let diag = lex_first_error("var x = @;");
        assert!(diag.message.contains("unexpected character"));
        assert_eq!(diag.code, Some("E0001"));
        assert_eq!(diag.span, Span::from(8..9));
    }

    #[test]
    fn error_invalid_escape() {
        let diag = lex_first_error(r#""hello\qworld""#);
        assert!(diag.message.contains("invalid escape sequence"));
        assert!(diag.message.contains("\\q"));
        assert_eq!(diag.code, Some("E0004"));
    }

    #[test]
    fn error_unterminated_string() {
        let diag = lex_first_error(r#"var x = "hello"#);
        assert!(diag.message.contains("unterminated string"));
        assert_eq!(diag.code, Some("E0005"));
    }

    #[test]
    fn error_unterminated_block_comment() {
        let diag = lex_first_error("var x = /* comment without end");
        assert!(diag.message.contains("unterminated block comment"));
        assert_eq!(diag.code, Some("E0006"));
    }

    #[test]
    fn error_diagnostic_eprint() {
        // eprint가 패닉 없이 동작하는지 확인
        let diag = lex_first_error("var x = @;");
        diag.eprint("test.min", "var x = @;");
    }
}
