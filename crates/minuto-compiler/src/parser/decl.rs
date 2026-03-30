use crate::ast::{Decl, FieldDecl, FnDecl, Ident, Param, Parsed, StructDecl, TypeExpr};
use crate::errors::ParserError;
use crate::lexer::token::Token;
use crate::Parser;

impl Parser {
    pub(crate) fn parse_decl(&mut self) -> Result<Decl<Parsed>, ()> {
        let start = self.mark();

        match self.peek().map(|(t, _)| t) {
            Some(Token::Fn) => {
                let fn_decl = self.parse_fn_decl(start)?;
                Ok(Decl::Fn(fn_decl))
            }
            Some(Token::Struct) => {
                let struct_decl = self.parse_struct_decl(start)?;
                Ok(Decl::Struct(struct_decl))
            }
            Some(_) => {
                let found = format!("{:?}", self.peek().unwrap().0);
                self.emit(
                    ParserError::UnexpectedToken {
                        expected: "fn or struct".to_string(),
                        found,
                    },
                    self.span_from(start),
                );
                Err(())
            }
            None => {
                self.emit(
                    ParserError::UnexpectedEof {
                        expected: "declaration".to_string(),
                    },
                    self.span_from(start),
                );
                Err(())
            }
        }
    }

    fn parse_fn_decl(&mut self, start: usize) -> Result<FnDecl<Parsed>, ()> {
        self.parse_fn_decl_in(start, None)
    }

    fn parse_fn_decl_in(
        &mut self,
        start: usize,
        struct_name: Option<&str>,
    ) -> Result<FnDecl<Parsed>, ()> {
        self.advance(); // consume `fn`

        let name_str = self.expect_ident()?;
        let name = Ident {
            name: name_str,
            id: (),
        };

        let params = self.parse_params(struct_name)?;

        let ret_ty = if self.eat(&Token::Arrow).is_some() {
            self.parse_type()?
        } else {
            TypeExpr::Void(self.span_from(start))
        };

        let body = self.parse_block()?;

        Ok(FnDecl {
            ann: self.span_from(start),
            name,
            params,
            ret_ty,
            body,
        })
    }

    fn parse_params(&mut self, struct_name: Option<&str>) -> Result<Vec<Param<Parsed>>, ()> {
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            params.push(self.parse_param(struct_name)?);
            while self.eat(&Token::Comma).is_some() {
                params.push(self.parse_param(struct_name)?);
            }
        }

        self.expect(&Token::RParen)?;
        Ok(params)
    }

    fn parse_param(&mut self, struct_name: Option<&str>) -> Result<Param<Parsed>, ()> {
        let start = self.mark();

        // self parameter → desugar to Named("self", ptr<StructName>)
        if self.check(&Token::SelfKw) {
            self.advance();
            let ty = if self.eat(&Token::Colon).is_some() {
                // Explicit type annotation: self: ptr<Foo>
                self.parse_type()?
            } else {
                // Implicit: self → ptr<StructName>
                let sname = struct_name.expect("self param outside struct method");
                let ann = self.span_from(start);
                TypeExpr::Ptr(ann.clone(), Box::new(TypeExpr::Named(ann, sname.to_string())))
            };
            return Ok(Param {
                ann: self.span_from(start),
                name: Ident {
                    name: "self".to_string(),
                    id: (),
                },
                ty,
            });
        }

        // named parameter: name: type
        let name_str = self.expect_ident()?;
        let name = Ident {
            name: name_str,
            id: (),
        };
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;

        Ok(Param {
            ann: self.span_from(start),
            name,
            ty,
        })
    }

    fn parse_struct_decl(&mut self, start: usize) -> Result<StructDecl<Parsed>, ()> {
        self.advance(); // consume `struct`

        let name_str = self.expect_ident()?;
        let name = Ident {
            name: name_str,
            id: (),
        };

        self.expect(&Token::LBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(&Token::RBrace) && !self.at_end() {
            let pos_before = self.pos;
            if self.check(&Token::Fn) {
                let method_start = self.mark();
                match self.parse_fn_decl_in(method_start, Some(&name.name)) {
                    Ok(m) => methods.push(m),
                    Err(()) => self.synchronize_top_level(),
                }
            } else {
                match self.parse_field_decl() {
                    Ok(f) => fields.push(f),
                    Err(()) => {
                        self.synchronize();
                        // Ensure progress to avoid infinite loop
                        if self.pos == pos_before {
                            self.advance();
                        }
                    }
                }
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(StructDecl {
            ann: self.span_from(start),
            name,
            fields,
            methods,
        })
    }

    fn parse_field_decl(&mut self) -> Result<FieldDecl<Parsed>, ()> {
        let start = self.mark();
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&Token::Comma)?;

        Ok(FieldDecl {
            ann: self.span_from(start),
            name,
            ty,
        })
    }
}
