use crate::ast::{Block, Ident, Parsed, ParsedStmt, Stmt, StmtKind};
use crate::diagnostic::Diagnostic;
use crate::errors::ParserError;
use crate::lexer::token::Token;
use crate::Parser;

impl Parser {
    pub(crate) fn parse_stmt(&mut self) -> Result<ParsedStmt, Vec<Diagnostic>> {
        let start = self.mark();

        match self.peek().map(|(t, _)| t.clone()) {
            None => Err(vec![Diagnostic::from((
                ParserError::UnexpectedEof {
                    expected: "statement".to_string(),
                },
                self.span_from(start),
            ))]),

            Some(Token::Var) => {
                let kind = self.parse_var_decl().map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind,
                })
            }

            Some(Token::Const) => {
                let kind = self.parse_const_decl().map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind,
                })
            }

            Some(Token::Return) => {
                let kind = self.parse_return_stmt().map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind,
                })
            }

            Some(Token::If) => {
                let kind = self.parse_if_stmt()?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind,
                })
            }

            Some(Token::While) => {
                let kind = self.parse_while_stmt()?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind,
                })
            }

            Some(Token::Break) => {
                self.advance();
                self.expect(&Token::Semicolon).map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind: StmtKind::Break,
                })
            }

            Some(Token::Continue) => {
                self.advance();
                self.expect(&Token::Semicolon).map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind: StmtKind::Continue,
                })
            }

            Some(_) => {
                let expr = self.parse_expr().map_err(|e| vec![e])?;
                self.expect(&Token::Semicolon).map_err(|e| vec![e])?;
                Ok(Stmt {
                    ann: self.span_from(start),
                    kind: StmtKind::Expr(expr),
                })
            }
        }
    }

    pub(crate) fn parse_block(&mut self) -> Result<Block<Parsed>, Vec<Diagnostic>> {
        self.expect(&Token::LBrace).map_err(|e| vec![e])?;

        let start = self.mark();
        let mut stmts = Vec::new();
        let mut all_errors = Vec::new();

        while !self.check(&Token::RBrace) && !self.at_end() {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(mut errs) => {
                    all_errors.append(&mut errs);
                    self.synchronize();
                }
            }
        }

        self.expect(&Token::RBrace).map_err(|e| vec![e])?;

        if !all_errors.is_empty() {
            return Err(all_errors);
        }

        Ok(Block {
            ann: self.span_from(start),
            stmts,
        })
    }

    fn parse_var_decl(&mut self) -> Result<StmtKind<Parsed>, Diagnostic> {
        self.advance(); // consume `var`
        let name_str = self.expect_ident()?;
        let name = Ident {
            name: name_str,
            id: (),
        };

        let ty = if self.eat(&Token::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Eq)?;
        let init = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;

        Ok(StmtKind::VarDecl { name, ty, init })
    }

    fn parse_const_decl(&mut self) -> Result<StmtKind<Parsed>, Diagnostic> {
        self.advance(); // consume `const`
        let name_str = self.expect_ident()?;
        let name = Ident {
            name: name_str,
            id: (),
        };

        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Eq)?;
        let init = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;

        Ok(StmtKind::ConstDecl { name, ty, init })
    }

    fn parse_return_stmt(&mut self) -> Result<StmtKind<Parsed>, Diagnostic> {
        self.advance(); // consume `return`

        if self.eat(&Token::Semicolon).is_some() {
            return Ok(StmtKind::Return(None));
        }

        let expr = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        Ok(StmtKind::Return(Some(expr)))
    }

    fn parse_if_stmt(&mut self) -> Result<StmtKind<Parsed>, Vec<Diagnostic>> {
        self.advance(); // consume `if`

        let cond = self.parse_expr_no_struct_lit().map_err(|e| vec![e])?;
        let then_block = self.parse_block()?;

        let mut else_ifs = Vec::new();
        let mut else_block = None;

        while self.eat(&Token::Else).is_some() {
            if self.check(&Token::If) {
                self.advance(); // consume `if`
                let ei_cond = self.parse_expr_no_struct_lit().map_err(|e| vec![e])?;
                let ei_block = self.parse_block()?;
                else_ifs.push((ei_cond, ei_block));
            } else {
                else_block = Some(self.parse_block()?);
                break;
            }
        }

        Ok(StmtKind::If {
            cond,
            then_block,
            else_ifs,
            else_block,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<StmtKind<Parsed>, Vec<Diagnostic>> {
        self.advance(); // consume `while`

        let cond = self.parse_expr_no_struct_lit().map_err(|e| vec![e])?;
        let body = self.parse_block()?;

        Ok(StmtKind::While { cond, body })
    }
}
