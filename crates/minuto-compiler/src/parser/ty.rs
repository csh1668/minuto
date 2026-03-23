use crate::ast::{Parsed, Span, TypeExpr};
use crate::diagnostic::Diagnostic;
use crate::errors::ParserError;
use crate::lexer::token::Token;
use crate::parser::Parser;

type ParsedType = TypeExpr<Parsed>;

impl Parser {
    pub(crate) fn parse_type(&mut self) -> Result<ParsedType, Diagnostic> {
        let start = self.mark();

        // readonly <type>
        if self.eat(&Token::Readonly).is_some() {
            let inner = self.parse_type()?;
            return Ok(TypeExpr::Readonly(self.span_from(start), Box::new(inner)));
        }

        // fn(params) -> ret
        if self.check(&Token::Fn) {
            return self.parse_fn_type(start);
        }

        let (token, span) = match self.peek() {
            Some(t) => t.clone(),
            None => {
                return Err(Diagnostic::from((
                    ParserError::UnexpectedEof {
                        expected: "type".to_string(),
                    },
                    self.span_from(start),
                )));
            }
        };

        match token {
            Token::Int => {
                self.advance();
                Ok(TypeExpr::Int(span))
            }
            Token::Char => {
                self.advance();
                Ok(TypeExpr::Char(span))
            }
            Token::Void => {
                self.advance();
                Ok(TypeExpr::Void(span))
            }
            Token::Ptr => {
                self.advance();
                self.expect(&Token::Lt)?;
                let inner = self.parse_type()?;
                // self.expect(&Token::Gt)?;
                self.expect_gt()?;
                Ok(TypeExpr::Ptr(self.span_from(start), Box::new(inner)))
            }
            Token::Span => {
                self.advance();
                self.expect(&Token::Lt)?;
                let inner = self.parse_type()?;
                // self.expect(&Token::Gt)?;
                self.expect_gt()?;
                Ok(TypeExpr::Span(self.span_from(start), Box::new(inner)))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(TypeExpr::Named(self.span_from(start), name))
            }
            _ => Err(Diagnostic::from((
                ParserError::ExpectedType {
                    found: format!("{token:?}"),
                },
                self.span_from(start),
            ))),
        }
    }

    fn parse_fn_type(&mut self, start: usize) -> Result<ParsedType, Diagnostic> {
        self.advance(); // consume "fn"
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            params.push(self.parse_type()?);
            while self.eat(&Token::Comma).is_some() {
                params.push(self.parse_type()?);
            }
        }
        self.expect(&Token::RParen)?;

        let ret = if self.eat(&Token::Arrow).is_some() {
            self.parse_type()?
        } else {
            TypeExpr::Void(self.span_from(start))
        };

        Ok(TypeExpr::Fn {
            ann: self.span_from(start),
            params,
            ret: Box::new(ret),
        })
    }

    /// `ptr<ptr<int>>` 같은 경우, `>` 토큰이 `>>`로 묶일 수 있음
    /// `>`를 기대할 때 `>>`나 `>=`가 나오면 분해해서 처리한다.
    fn expect_gt(&mut self) -> Result<Span, Diagnostic> {
        match self.peek().cloned() {
            Some((Token::Gt, span)) => {
                self.advance();
                Ok(span)
            }
            Some((Token::Shr, span)) => {
                // >>를 > / >로 분해한다
                let first = Span {
                    start: span.start,
                    end: span.start + 1,
                };
                let second = Span {
                    start: span.start + 1,
                    end: span.end,
                };
                self.tokens[self.pos] = (Token::Gt, second);
                // 두 번째 >가 남아있으므로 pos는 유지
                Ok(first)
            }
            Some((Token::GtEq, span)) => {
                // >=를 > / =로 분해한다
                let first = Span {
                    start: span.start,
                    end: span.start + 1,
                };
                let second = Span {
                    start: span.start + 1,
                    end: span.end,
                };
                self.tokens[self.pos] = (Token::Eq, second);
                // =가 남아있으므로 pos는 유지
                Ok(first)
            }
            Some((token, _)) => {
                let start = self.mark();
                Err(Diagnostic::from((
                    ParserError::UnexpectedToken {
                        expected: ">".into(),
                        found: format!("{token:?}"),
                    },
                    self.span_from(start),
                )))
            }
            None => {
                let start = self.mark();
                Err(Diagnostic::from((
                    ParserError::UnexpectedEof {
                        expected: ">".into(),
                    },
                    self.span_from(start),
                )))
            }
        }
    }
}
