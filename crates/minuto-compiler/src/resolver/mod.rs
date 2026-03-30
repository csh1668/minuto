use std::collections::HashMap;

use crate::ast::*;
use crate::common::{
    FieldInfo, FnSig, ParamInfo, Resolved, Span, StructInfo, Symbol, SymbolId, SymbolKind,
    SymbolTable,
};
use crate::diagnostic::Diagnostic;
use crate::errors::ResolverError;

struct Scope {
    bindings: HashMap<String, SymbolId>,
}

pub struct Resolver {
    symbols: SymbolTable,
    scopes: Vec<Scope>,
    next_id: u32,
    loop_depth: u32,
    diagnostics: Vec<Diagnostic>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            scopes: Vec::new(),
            next_id: 0,
            loop_depth: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn resolve(
        mut self,
        program: &ParsedProgram,
    ) -> Result<(ResolvedProgram, SymbolTable), Vec<Diagnostic>> {
        // Global scope
        self.push_scope();

        // Pass 1: collect all top-level declarations
        self.collect_decls(&program.decls);

        // Pass 2: resolve all bodies
        let decls = self.resolve_decls(&program.decls);

        // Check for main function
        self.check_main();

        self.pop_scope();

        if self.diagnostics.is_empty() {
            Ok((Program { decls }, self.symbols))
        } else {
            Err(self.diagnostics)
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            bindings: HashMap::new(),
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn fresh_id(&mut self) -> SymbolId {
        let id = SymbolId(self.next_id);
        self.next_id += 1;
        id
    }

    fn define(&mut self, name: &str, span: Span, kind: SymbolKind) -> SymbolId {
        self.check_reserved(name, &span);

        let id = self.fresh_id();

        let has_dup = self
            .scopes
            .last()
            .map(|s| s.bindings.contains_key(name))
            .unwrap_or(false);
        if has_dup {
            self.emit(
                ResolverError::DuplicateDefinition {
                    name: name.to_string(),
                },
                span.clone(),
            );
        }

        let scope = self.scopes.last_mut().expect("no scope");
        scope.bindings.insert(name.to_string(), id);
        self.symbols.insert(Symbol {
            id,
            name: name.to_string(),
            kind,
            span,
        });
        id
    }

    fn lookup(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.bindings.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn check_reserved(&mut self, name: &str, span: &Span) {
        if RESERVED_NAMES.contains(&name) {
            self.emit(
                ResolverError::ReservedIdentifier {
                    name: name.to_string(),
                },
                span.clone(),
            );
        }
    }

    fn emit(&mut self, error: ResolverError, span: Span) {
        self.diagnostics.push(Diagnostic::from((error, span)));
    }

    fn collect_decls(&mut self, decls: &[Decl<Parsed>]) {
        for decl in decls {
            match decl {
                Decl::Fn(fn_decl) => {
                    let sig = self.build_fn_sig(fn_decl);
                    self.define(
                        &fn_decl.name.name,
                        fn_decl.ann.clone(),
                        SymbolKind::Fn(sig),
                    );
                }
                Decl::Struct(struct_decl) => {
                    // Check for duplicate field names
                    let mut seen_fields = HashMap::new();
                    for f in &struct_decl.fields {
                        if let Some(_) = seen_fields.insert(f.name.clone(), ()) {
                            self.emit(
                                ResolverError::DuplicateField {
                                    struct_name: struct_decl.name.name.clone(),
                                    field: f.name.clone(),
                                },
                                f.ann.clone(),
                            );
                        }
                    }

                    let fields: Vec<FieldInfo> = struct_decl
                        .fields
                        .iter()
                        .map(|f| FieldInfo {
                            name: f.name.clone(),
                        })
                        .collect();

                    // Insert struct symbol first (with empty fns) to maintain sequential IDs
                    let struct_id = self.define(
                        &struct_decl.name.name,
                        struct_decl.ann.clone(),
                        SymbolKind::Struct(StructInfo {
                            fields,
                            fns: HashMap::new(),
                        }),
                    );

                    // Now insert method symbols sequentially after the struct
                    let mut fns = HashMap::new();
                    for method in &struct_decl.methods {
                        let sig = self.build_fn_sig(method);
                        let method_id = self.fresh_id();

                        self.symbols.insert(Symbol {
                            id: method_id,
                            name: method.name.name.clone(),
                            kind: SymbolKind::Fn(sig),
                            span: method.ann.clone(),
                        });

                        fns.insert(method.name.name.clone(), method_id);
                    }

                    // Update struct's StructInfo with method IDs
                    if !fns.is_empty() {
                        let sym = self.symbols.get_mut(struct_id);
                        if let SymbolKind::Struct(info) = &mut sym.kind {
                            info.fns = fns;
                        }
                    }
                }
            }
        }
    }

    fn build_fn_sig(&self, fn_decl: &FnDecl<Parsed>) -> FnSig {
        let params = fn_decl
            .params
            .iter()
            .map(|p| ParamInfo {
                name: p.name.name.clone(),
            })
            .collect();

        let ret_ty = format!("{:?}", fn_decl.ret_ty);

        FnSig { params, ret_ty }
    }

    fn resolve_decls(&mut self, decls: &[Decl<Parsed>]) -> Vec<Decl<Resolved>> {
        decls.iter().map(|d| self.resolve_decl(d)).collect()
    }

    fn resolve_decl(&mut self, decl: &Decl<Parsed>) -> Decl<Resolved> {
        match decl {
            Decl::Fn(fn_decl) => Decl::Fn(self.resolve_fn_decl(fn_decl)),
            Decl::Struct(struct_decl) => Decl::Struct(self.resolve_struct_decl(struct_decl)),
        }
    }

    fn resolve_fn_decl(&mut self, fn_decl: &FnDecl<Parsed>) -> FnDecl<Resolved> {
        let name_id = self
            .lookup(&fn_decl.name.name)
            .unwrap_or(SymbolId(u32::MAX));

        self.push_scope();

        let params = self.resolve_params(&fn_decl.params);
        let ret_ty = self.resolve_type_expr(&fn_decl.ret_ty);
        let body = self.resolve_block(&fn_decl.body);

        self.pop_scope();

        FnDecl {
            ann: fn_decl.ann.clone(),
            name: Ident {
                name: fn_decl.name.name.clone(),
                id: name_id,
            },
            params,
            ret_ty,
            body,
        }
    }

    fn resolve_struct_decl(&mut self, struct_decl: &StructDecl<Parsed>) -> StructDecl<Resolved> {
        let name_id = self
            .lookup(&struct_decl.name.name)
            .unwrap_or(SymbolId(u32::MAX));

        let fields = struct_decl
            .fields
            .iter()
            .map(|f| FieldDecl {
                ann: f.ann.clone(),
                name: f.name.clone(),
                ty: self.resolve_type_expr(&f.ty),
            })
            .collect();

        let methods = struct_decl
            .methods
            .iter()
            .map(|m| self.resolve_method(m, &struct_decl.name.name))
            .collect();

        StructDecl {
            ann: struct_decl.ann.clone(),
            name: Ident {
                name: struct_decl.name.name.clone(),
                id: name_id,
            },
            fields,
            methods,
        }
    }

    fn resolve_method(
        &mut self,
        method: &FnDecl<Parsed>,
        struct_name: &str,
    ) -> FnDecl<Resolved> {
        // Look up method's SymbolId from the struct's info
        let method_id = self
            .lookup(struct_name)
            .and_then(|sid| {
                let info = self.symbols.get_struct_info(sid)?;
                info.fns.get(&method.name.name).copied()
            })
            .unwrap_or(SymbolId(u32::MAX));

        self.push_scope();

        let params = self.resolve_params(&method.params);
        let ret_ty = self.resolve_type_expr(&method.ret_ty);
        let body = self.resolve_block(&method.body);

        self.pop_scope();

        FnDecl {
            ann: method.ann.clone(),
            name: Ident {
                name: method.name.name.clone(),
                id: method_id,
            },
            params,
            ret_ty,
            body,
        }
    }

    fn resolve_params(&mut self, params: &[Param<Parsed>]) -> Vec<Param<Resolved>> {
        params
            .iter()
            .map(|p| {
                let id = self.define(&p.name.name, p.ann.clone(), SymbolKind::Param);
                Param {
                    ann: p.ann.clone(),
                    name: Ident {
                        name: p.name.name.clone(),
                        id,
                    },
                    ty: self.resolve_type_expr(&p.ty),
                }
            })
            .collect()
    }

    fn resolve_block(&mut self, block: &Block<Parsed>) -> Block<Resolved> {
        self.push_scope();
        let stmts = block.stmts.iter().map(|s| self.resolve_stmt(s)).collect();
        self.pop_scope();
        Block {
            ann: block.ann.clone(),
            stmts,
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt<Parsed>) -> Stmt<Resolved> {
        let kind = match &stmt.kind {
            StmtKind::Expr(expr) => StmtKind::Expr(self.resolve_expr(expr)),

            StmtKind::VarDecl { name, ty, init } => {
                let init = self.resolve_expr(init);
                let ty = ty.as_ref().map(|t| self.resolve_type_expr(t));
                let id = self.define(&name.name, stmt.ann.clone(), SymbolKind::Var);
                StmtKind::VarDecl {
                    name: Ident {
                        name: name.name.clone(),
                        id,
                    },
                    ty,
                    init,
                }
            }

            StmtKind::ConstDecl { name, ty, init } => {
                let init = self.resolve_expr(init);
                let ty = self.resolve_type_expr(ty);
                let id = self.define(&name.name, stmt.ann.clone(), SymbolKind::Const);
                StmtKind::ConstDecl {
                    name: Ident {
                        name: name.name.clone(),
                        id,
                    },
                    ty,
                    init,
                }
            }

            StmtKind::Return(expr) => {
                StmtKind::Return(expr.as_ref().map(|e| self.resolve_expr(e)))
            }

            StmtKind::If {
                cond,
                then_block,
                else_ifs,
                else_block,
            } => StmtKind::If {
                cond: self.resolve_expr(cond),
                then_block: self.resolve_block(then_block),
                else_ifs: else_ifs
                    .iter()
                    .map(|(c, b)| (self.resolve_expr(c), self.resolve_block(b)))
                    .collect(),
                else_block: else_block.as_ref().map(|b| self.resolve_block(b)),
            },

            StmtKind::While { cond, body } => {
                self.loop_depth += 1;
                let result = StmtKind::While {
                    cond: self.resolve_expr(cond),
                    body: self.resolve_block(body),
                };
                self.loop_depth -= 1;
                result
            }

            StmtKind::Break => {
                if self.loop_depth == 0 {
                    self.emit(ResolverError::BreakOutsideLoop, stmt.ann.clone());
                }
                StmtKind::Break
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    self.emit(ResolverError::ContinueOutsideLoop, stmt.ann.clone());
                }
                StmtKind::Continue
            }
        };

        Stmt {
            ann: stmt.ann.clone(),
            kind,
        }
    }

    fn resolve_expr(&mut self, expr: &Expr<Parsed>) -> Expr<Resolved> {
        let kind = match &expr.kind {
            ExprKind::IntLit(v) => ExprKind::IntLit(*v),
            ExprKind::CharLit(v) => ExprKind::CharLit(*v),
            ExprKind::StrLit(v) => ExprKind::StrLit(v.clone()),

            ExprKind::Ident(ident) => match self.lookup(&ident.name) {
                Some(id) => ExprKind::Ident(Ident {
                    name: ident.name.clone(),
                    id,
                }),
                None => {
                    self.emit(
                        ResolverError::UndefinedVariable {
                            name: ident.name.clone(),
                        },
                        expr.ann.clone(),
                    );
                    ExprKind::Ident(Ident {
                        name: ident.name.clone(),
                        id: SymbolId(u32::MAX),
                    })
                }
            },

            ExprKind::Unary(op, operand) => {
                ExprKind::Unary(*op, Box::new(self.resolve_expr(operand)))
            }
            ExprKind::Binary(op, lhs, rhs) => ExprKind::Binary(
                *op,
                Box::new(self.resolve_expr(lhs)),
                Box::new(self.resolve_expr(rhs)),
            ),

            ExprKind::Assign { lhs, rhs } => ExprKind::Assign {
                lhs: Box::new(self.resolve_expr(lhs)),
                rhs: Box::new(self.resolve_expr(rhs)),
            },

            ExprKind::Call { callee, args } => ExprKind::Call {
                callee: Box::new(self.resolve_expr(callee)),
                args: args.iter().map(|a| self.resolve_expr(a)).collect(),
            },

            ExprKind::Field { base, field } => ExprKind::Field {
                base: Box::new(self.resolve_expr(base)),
                field: field.clone(),
            },

            ExprKind::Index { base, index } => ExprKind::Index {
                base: Box::new(self.resolve_expr(base)),
                index: Box::new(self.resolve_expr(index)),
            },

            ExprKind::MethodCall {
                base,
                method,
                args,
            } => ExprKind::MethodCall {
                base: Box::new(self.resolve_expr(base)),
                method: method.clone(),
                args: args.iter().map(|a| self.resolve_expr(a)).collect(),
            },

            ExprKind::StaticCall {
                receiver,
                method,
                args,
            } => {
                // Validate Named receiver refers to a known struct
                if let StaticReceiver::Named(name) = receiver {
                    if self.lookup(name).is_none() {
                        self.emit(
                            ResolverError::UndefinedType {
                                name: name.clone(),
                            },
                            expr.ann.clone(),
                        );
                    }
                }
                ExprKind::StaticCall {
                    receiver: receiver.clone(),
                    method: method.clone(),
                    args: args.iter().map(|a| self.resolve_expr(a)).collect(),
                }
            }

            ExprKind::StdCall { func, args } => ExprKind::StdCall {
                func: func.clone(),
                args: args.iter().map(|a| self.resolve_expr(a)).collect(),
            },

            ExprKind::StructLit { name, fields } => {
                // Validate struct name exists
                match self.lookup(name) {
                    Some(id) => {
                        // Validate field names
                        if let Some(info) = self.symbols.get_struct_info(id) {
                            let known_fields: Vec<String> =
                                info.fields.iter().map(|f| f.name.clone()).collect();
                            for field in fields {
                                if !known_fields.contains(&field.name) {
                                    self.emit(
                                        ResolverError::UndefinedField {
                                            struct_name: name.clone(),
                                            field: field.name.clone(),
                                        },
                                        expr.ann.clone(),
                                    );
                                }
                            }
                        }
                    }
                    None => {
                        self.emit(
                            ResolverError::UndefinedType {
                                name: name.clone(),
                            },
                            expr.ann.clone(),
                        );
                    }
                }

                ExprKind::StructLit {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|f| FieldInit {
                            name: f.name.clone(),
                            value: self.resolve_expr(&f.value),
                        })
                        .collect(),
                }
            }

            ExprKind::Alloc { ty, count } => ExprKind::Alloc {
                ty: self.resolve_type_expr(ty),
                count: Box::new(self.resolve_expr(count)),
            },

            ExprKind::Free { expr: inner } => ExprKind::Free {
                expr: Box::new(self.resolve_expr(inner)),
            },
        };

        Expr {
            ann: expr.ann.clone(),
            ty: (),
            kind,
        }
    }

    fn resolve_type_expr(&mut self, ty: &TypeExpr<Parsed>) -> TypeExpr<Resolved> {
        match ty {
            TypeExpr::Int(ann) => TypeExpr::Int(ann.clone()),
            TypeExpr::Char(ann) => TypeExpr::Char(ann.clone()),
            TypeExpr::Void(ann) => TypeExpr::Void(ann.clone()),
            TypeExpr::Ptr(ann, inner) => {
                TypeExpr::Ptr(ann.clone(), Box::new(self.resolve_type_expr(inner)))
            }
            TypeExpr::Span(ann, inner) => {
                TypeExpr::Span(ann.clone(), Box::new(self.resolve_type_expr(inner)))
            }
            TypeExpr::Readonly(ann, inner) => {
                TypeExpr::Readonly(ann.clone(), Box::new(self.resolve_type_expr(inner)))
            }
            TypeExpr::Fn { ann, params, ret } => TypeExpr::Fn {
                ann: ann.clone(),
                params: params.iter().map(|p| self.resolve_type_expr(p)).collect(),
                ret: Box::new(self.resolve_type_expr(ret)),
            },
            TypeExpr::Named(ann, name) => {
                if self.lookup(name).is_none() {
                    self.emit(
                        ResolverError::UndefinedType {
                            name: name.clone(),
                        },
                        ann.clone(),
                    );
                }
                TypeExpr::Named(ann.clone(), name.clone())
            }
        }
    }

    fn check_main(&mut self) {
        match self.lookup("main") {
            None => {
                self.diagnostics.push(Diagnostic::from((
                    ResolverError::MainNotFound,
                    Span { start: 0, end: 0 },
                )));
            }
            Some(id) => {
                let sym = self.symbols.get(id);
                if let SymbolKind::Fn(sig) = &sym.kind {
                    let has_params = !sig.params.is_empty();
                    let has_non_void_ret = !sig.ret_ty.starts_with("Void");

                    if has_params || has_non_void_ret {
                        let param_str = if has_params {
                            sig.params
                                .iter()
                                .map(|p| p.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        } else {
                            String::new()
                        };
                        let ret_str = if has_non_void_ret {
                            format!(" -> ...")
                        } else {
                            String::new()
                        };
                        self.diagnostics.push(Diagnostic::from((
                            ResolverError::InvalidMainSignature {
                                signature: format!("fn main({}){}", param_str, ret_str),
                            },
                            sym.span.clone(),
                        )));
                    }
                }
            }
        }
    }
}
