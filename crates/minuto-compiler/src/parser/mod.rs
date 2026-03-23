mod decl;
mod expr;
mod ty;

#[cfg(test)]
mod tests;
mod stmt;

use crate::ast::{ParsedProgram, Span};
use crate::diagnostic::Diagnostic;
use crate::errors::ParserError;
use crate::lexer::token::Token;

pub struct Parser {
    tokens: Vec<(Token, Span)>,
    pos: usize,
    allow_struct_lit: bool,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, Span)>) -> Self {
        Self {
            tokens,
            pos: 0,
            allow_struct_lit: true,
        }
    }

    pub fn parse(&mut self) -> Result<ParsedProgram, Vec<Diagnostic>> {
        let mut decls = Vec::new();
        let mut errors = Vec::new();

        while !self.at_end() {
            match self.parse_decl() {
                Ok(decl) => decls.push(decl),
                Err(mut errs) => {
                    errors.append(&mut errs);
                    self.synchronize_top_level();
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(ParsedProgram { decls })
    }

    fn peek(&self) -> Option<&(Token, Span)> {
        self.tokens.get(self.pos)
    }
    fn peek_at(&self, offset: usize) -> Option<&(Token, Span)> {
        self.tokens.get(self.pos + offset)
    }
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
    fn advance(&mut self) -> (Token, Span) {
        let token = self.peek().expect("Unexpected end of input").clone();
        self.pos += 1;
        token
    }
    fn check(&self, expected: &Token) -> bool {
        matches!(self.peek(), Some((token, _)) if token == expected)
    }
    fn eat(&mut self, expected: &Token) -> Option<Span> {
        if self.check(expected) {
            Some(self.advance().1)
        } else {
            None
        }
    }
    fn expect(&mut self, expected: &Token) -> Result<Span, Diagnostic> {
        if self.check(expected) {
            Ok(self.advance().1)
        } else {
            let start = self.mark();
            let found = match self.peek() {
                Some((t, _)) => format!("{t:?}"),
                None => "EOF".to_string(),
            };
            Err(Diagnostic::from((
                ParserError::UnexpectedToken {
                    expected: format!("{expected:?}"),
                    found,
                },
                self.span_from(start),
            )))
        }
    }

    fn span_from(&self, start: usize) -> Span {
        let end = match self.peek() {
            Some((_, span)) => span.start,
            None => self
                .tokens
                .last()
                .map(|(_, span)| span.end)
                .unwrap_or(start),
        };
        Span { start, end }
    }

    fn mark(&self) -> usize {
        match self.peek() {
            Some((_, span)) => span.start,
            None => self.tokens.last().map(|(_, span)| span.end).unwrap_or(0),
        }
    }

    fn synchronize(&mut self) {
        loop {
            match self.peek().map(|(t, _)| t) {
                None => break,
                Some(Token::Semicolon) => {
                    self.advance(); // consume the `;`
                    break;
                }
                Some(
                    Token::RBrace
                    | Token::Var
                    | Token::Const
                    | Token::Return
                    | Token::If
                    | Token::While
                    | Token::Break
                    | Token::Continue,
                ) => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn synchronize_top_level(&mut self) {
        loop {
            match self.peek().map(|(t, _)| t) {
                None => break,
                Some(Token::Fn | Token::Struct) => break,
                Some(Token::RBrace) => {
                    self.advance(); // consume `}`
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}
