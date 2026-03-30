use crate::ast::{BinOp, ExprKind, FieldInit, Ident, Parsed, ParsedExpr, Span, StaticReceiver, UnaryOp, RESERVED_NAMES};
use crate::errors::ParserError;
use crate::lexer::token::Token;
use crate::parser::Parser;

#[derive(Clone, Copy)]
struct InfixRule {
    op: BinOp,
    prec: u8,
    right_assoc: bool,
}

/// Infix operator precedence and associativity rules
fn infix_rule(token: &Token) -> Option<InfixRule> {
    let (op, prec, right_assoc) = match token {
        Token::OrOr => (BinOp::Or, 1, false),

        Token::AndAnd => (BinOp::And, 2, false),

        Token::Pipe => (BinOp::BitOr, 3, false),

        Token::Caret => (BinOp::BitXor, 4, false),

        Token::Ampersand => (BinOp::BitAnd, 5, false),

        Token::EqEq => (BinOp::Eq, 6, false),
        Token::NotEq => (BinOp::Ne, 6, false),

        Token::Lt => (BinOp::Lt, 7, false),
        Token::LtEq => (BinOp::Le, 7, false),
        Token::Gt => (BinOp::Gt, 7, false),
        Token::GtEq => (BinOp::Ge, 7, false),

        Token::Shl => (BinOp::Shl, 8, false),
        Token::Shr => (BinOp::Shr, 8, false),

        Token::Plus => (BinOp::Add, 9, false),
        Token::Minus => (BinOp::Sub, 9, false),

        Token::Star => (BinOp::Mul, 10, false),
        Token::Slash => (BinOp::Div, 10, false),
        Token::Percent => (BinOp::Mod, 10, false),

        _ => return None,
    };
    Some(InfixRule {
        op,
        prec,
        right_assoc,
    })
}

fn expr(span: Span, expr_kind: ExprKind<Parsed>) -> ParsedExpr {
    ParsedExpr {
        ann: span,
        ty: (),
        kind: expr_kind,
    }
}

impl Parser {
    pub fn parse_expr(&mut self) -> Result<ParsedExpr, ()> {
        self.parse_assignment()
    }

    pub(crate) fn parse_expr_no_struct_lit(&mut self) -> Result<ParsedExpr, ()> {
        let saved = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let result = self.parse_expr();
        self.allow_struct_lit = saved;
        result
    }

    fn parse_assignment(&mut self) -> Result<ParsedExpr, ()> {
        let lhs = self.parse_binary(0)?;

        if self.eat(&Token::Eq).is_some() {
            let rhs = self.parse_assignment()?;
            let start = lhs.ann.start;
            return Ok(expr(
                self.span_from(start),
                ExprKind::Assign {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            ));
        }

        Ok(lhs)
    }

    fn parse_binary(&mut self, min_prec: u8) -> Result<ParsedExpr, ()> {
        let mut lhs = self.parse_unary()?;

        while let Some(rule) = self.peek().and_then(|(tok, _)| infix_rule(tok)) {
            if rule.prec < min_prec {
                break;
            }
            self.advance(); // consume operator

            let next_min_prec = if rule.right_assoc {
                rule.prec
            } else {
                rule.prec + 1
            };
            let rhs = self.parse_binary(next_min_prec)?;

            let span = self.span_from(lhs.ann.start);

            lhs = expr(
                span,
                ExprKind::Binary(rule.op, Box::new(lhs), Box::new(rhs)),
            );
        }

        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<ParsedExpr, ()> {
        let start = self.mark();

        let op = match self.peek().map(|(t, _)| t) {
            Some(Token::Minus) => UnaryOp::Neg,
            Some(Token::Bang) => UnaryOp::Not,
            Some(Token::Tilde) => UnaryOp::BitNot,
            Some(Token::Star) => UnaryOp::Deref,
            Some(Token::Ampersand) => UnaryOp::AddrOf,
            _ => return self.parse_postfix(),
        };

        self.advance(); // consume operator
        let operand = self.parse_unary()?;

        Ok(expr(
            self.span_from(start),
            ExprKind::Unary(op, Box::new(operand)),
        ))
    }

    fn parse_postfix(&mut self) -> Result<ParsedExpr, ()> {
        let mut lhs = self.parse_primary()?;

        loop {
            if self.eat(&Token::LBracket).is_some() {
                // Index: expr[expr]
                let index = self.parse_expr()?;
                self.expect(&Token::RBracket)?;
                let span = self.span_from(lhs.ann.start);
                lhs = expr(
                    span,
                    ExprKind::Index {
                        base: Box::new(lhs),
                        index: Box::new(index),
                    },
                );
            } else if self.eat(&Token::Arrow).is_some() {
                // a->x  →  (*a).x
                // a->method(args)  →  (*a).method(args)
                let start = lhs.ann.start;
                let derefed = expr(
                    self.span_from(start),
                    ExprKind::Unary(UnaryOp::Deref, Box::new(lhs)),
                );

                let name = self.expect_field_name()?;
                if self.check(&Token::LParen) {
                    let args = self.parse_args()?;
                    let span = self.span_from(start);
                    lhs = expr(
                        span,
                        ExprKind::MethodCall {
                            base: Box::new(derefed),
                            method: name,
                            args,
                        },
                    );
                } else {
                    let span = self.span_from(start);
                    lhs = expr(
                        span,
                        ExprKind::Field {
                            base: Box::new(derefed),
                            field: name,
                        },
                    );
                }
            } else if self.eat(&Token::Dot).is_some() {
                // Field or MethodCall: expr.ident or expr.ident(args)
                let name = self.expect_field_name()?;
                if self.check(&Token::LParen) {
                    let args = self.parse_args()?;
                    let span = self.span_from(lhs.ann.start);
                    lhs = expr(
                        span,
                        ExprKind::MethodCall {
                            base: Box::new(lhs),
                            method: name,
                            args,
                        },
                    );
                } else {
                    let span = self.span_from(lhs.ann.start);
                    lhs = expr(
                        span,
                        ExprKind::Field {
                            base: Box::new(lhs),
                            field: name,
                        },
                    );
                }
            } else if self.check(&Token::LParen) {
                // Call: expr(args)
                let args = self.parse_args()?;
                let span = self.span_from(lhs.ann.start);
                lhs = expr(
                    span,
                    ExprKind::Call {
                        callee: Box::new(lhs),
                        args,
                    },
                );
            } else {
                break;
            }
        }

        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<ParsedExpr, ()> {
        let start = self.mark();
        let (token, span) = match self.peek() {
            Some(t) => t.clone(),
            None => {
                self.emit(
                    ParserError::UnexpectedEof {
                        expected: "expression".to_string(),
                    },
                    self.span_from(start),
                );
                return Err(());
            }
        };

        match token {
            Token::IntLit(n) => {
                self.advance();
                Ok(expr(span, ExprKind::IntLit(n)))
            }
            Token::CharLit(c) => {
                self.advance();
                Ok(expr(span, ExprKind::CharLit(c)))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(expr(span, ExprKind::StrLit(s)))
            }

            // (expr) — parenthesized expression
            Token::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }

            Token::Ident(ref name) if RESERVED_NAMES.contains(&name.as_str()) => {
                match name.as_str() {
                    // alloc<T>(n)
                    "alloc" => self.parse_alloc(start),
                    // free(expr)
                    "free" => self.parse_free(start),
                    _ => unreachable!(),
                }
            }

            // span::method(args)
            Token::Span if self.peek_at(1).map(|(t, _)| t) == Some(&Token::ColonColon) => {
                self.parse_type_static_call(start, StaticReceiver::Span)
            }
            // ptr::method(args)
            Token::Ptr if self.peek_at(1).map(|(t, _)| t) == Some(&Token::ColonColon) => {
                self.parse_type_static_call(start, StaticReceiver::Ptr)
            }
            // int::method(args)
            Token::Int if self.peek_at(1).map(|(t, _)| t) == Some(&Token::ColonColon) => {
                self.parse_type_static_call(start, StaticReceiver::Int)
            }
            // char::method(args)
            Token::Char if self.peek_at(1).map(|(t, _)| t) == Some(&Token::ColonColon) => {
                self.parse_type_static_call(start, StaticReceiver::Char)
            }

            // std::func(args)
            Token::Ident(ref name) if name == "std" => self.parse_std_call(start),

            Token::SelfKw => {
                self.advance();
                Ok(expr(
                    span,
                    ExprKind::Ident(Ident {
                        name: "self".to_string(),
                        id: (),
                    }),
                ))
            }

            Token::Ident(name) => {
                self.advance();

                // StructName::method(args)
                if self.check(&Token::ColonColon) {
                    return self.parse_static_call(start, &name);
                }

                // StructName { field: value, ... }
                if self.allow_struct_lit && self.check(&Token::LBrace) {
                    return self.parse_struct_lit(start, &name);
                }

                Ok(expr(
                    self.span_from(start),
                    ExprKind::Ident(Ident { name, id: () }),
                ))
            }

            _ => {
                self.emit(
                    ParserError::ExpectedExpression {
                        found: format!("{token:?}"),
                    },
                    self.span_from(start),
                );
                Err(())
            }
        }
    }

    fn parse_alloc(&mut self, start: usize) -> Result<ParsedExpr, ()> {
        self.advance(); // consume "alloc"
        self.expect(&Token::Lt)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Gt)?;
        self.expect(&Token::LParen)?;
        let count = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(expr(
            self.span_from(start),
            ExprKind::Alloc {
                ty,
                count: Box::new(count),
            },
        ))
    }

    fn parse_free(&mut self, start: usize) -> Result<ParsedExpr, ()> {
        self.advance(); // consume "free"
        self.expect(&Token::LParen)?;
        let arg = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(expr(
            self.span_from(start),
            ExprKind::Free {
                expr: Box::new(arg),
            },
        ))
    }

    fn parse_type_static_call(
        &mut self,
        start: usize,
        receiver: StaticReceiver,
    ) -> Result<ParsedExpr, ()> {
        self.advance(); // consume type keyword (span, ptr, int, char)
        self.expect(&Token::ColonColon)?;
        let method = self.expect_ident()?;
        let args = self.parse_args()?;
        Ok(expr(
            self.span_from(start),
            ExprKind::StaticCall {
                receiver,
                method,
                args,
            },
        ))
    }

    fn parse_std_call(&mut self, start: usize) -> Result<ParsedExpr, ()> {
        self.advance(); // consume "std"
        self.expect(&Token::ColonColon)?;
        let func = self.expect_ident()?;
        let args = self.parse_args()?;
        Ok(expr(
            self.span_from(start),
            ExprKind::StdCall { func, args },
        ))
    }

    fn parse_static_call(
        &mut self,
        start: usize,
        receiver: &str,
    ) -> Result<ParsedExpr, ()> {
        self.expect(&Token::ColonColon)?;
        let method = self.expect_ident()?;
        let args = self.parse_args()?;
        Ok(expr(
            self.span_from(start),
            ExprKind::StaticCall {
                receiver: StaticReceiver::Named(receiver.to_string()),
                method,
                args,
            },
        ))
    }

    fn parse_struct_lit(&mut self, start: usize, name: &str) -> Result<ParsedExpr, ()> {
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.at_end() {
            let field_name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push(FieldInit {
                name: field_name,
                value,
            });
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(expr(
            self.span_from(start),
            ExprKind::StructLit {
                name: name.to_string(),
                fields,
            },
        ))
    }

    // ── Helpers ──

    pub(crate) fn expect_ident(&mut self) -> Result<String, ()> {
        let start = self.mark();
        match self.peek().cloned() {
            Some((Token::Ident(name), _)) => {
                self.advance();
                Ok(name)
            }
            Some((token, _)) => {
                self.emit(
                    ParserError::ExpectedIdentifier {
                        found: format!("{token:?}"),
                    },
                    self.span_from(start),
                );
                Err(())
            }
            None => {
                self.emit(
                    ParserError::UnexpectedEof {
                        expected: "identifier".to_string(),
                    },
                    self.span_from(start),
                );
                Err(())
            }
        }
    }

    fn expect_field_name(&mut self) -> Result<String, ()> {
        let start = self.mark();

        let type_keyword_as_name = |kw: &Token| match kw {
            Token::Int => Some("int".to_string()),
            Token::Char => Some("char".to_string()),
            Token::Void => Some("void".to_string()),
            Token::Fn => Some("fn".to_string()),
            Token::Ptr => Some("ptr".to_string()),
            Token::Span => Some("span".to_string()),
            _ => None,
        };

        match self.peek().cloned() {
            Some((Token::Ident(name), _)) => {
                self.advance();
                Ok(name)
            }
            Some((ref tok, _)) if type_keyword_as_name(tok).is_some() => {
                let name = type_keyword_as_name(tok).unwrap();
                self.advance();
                Ok(name)
            }
            Some((tok, _)) => {
                self.emit(
                    ParserError::ExpectedIdentifier {
                        found: format!("{tok:?}"),
                    },
                    self.span_from(start),
                );
                Err(())
            }
            None => {
                self.emit(
                    ParserError::UnexpectedEof {
                        expected: "field name".to_string(),
                    },
                    self.span_from(start),
                );
                Err(())
            }
        }
    }

    fn parse_args(&mut self) -> Result<Vec<ParsedExpr>, ()> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            args.push(self.parse_expr()?);
            while self.eat(&Token::Comma).is_some() {
                args.push(self.parse_expr()?);
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }
}
