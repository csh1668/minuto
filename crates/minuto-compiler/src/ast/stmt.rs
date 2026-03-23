use crate::ast::expr::Expr;
use crate::ast::ty::TypeExpr;
use crate::common::{Ident, Phase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block<P: Phase> {
    pub ann: P::Ann,
    pub stmts: Vec<Stmt<P>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt<P: Phase> {
    pub ann: P::Ann,
    pub kind: StmtKind<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind<P: Phase> {
    Expr(Expr<P>),
    VarDecl {
        // var name: [: type] = init;
        name: Ident<P>,
        ty: Option<TypeExpr<P>>,
        init: Expr<P>,
    },
    ConstDecl {
        // const name: type = init;
        name: Ident<P>,
        ty: TypeExpr<P>,
        init: Expr<P>,
    },
    Return(Option<Expr<P>>), // return [expr];
    If {
        // if cond { then_block } [else if cond { else_if_block }] [else { else_block }]
        cond: Expr<P>,
        then_block: Block<P>,
        else_ifs: Vec<(Expr<P>, Block<P>)>,
        else_block: Option<Block<P>>,
    },
    While {
        // while cond { ... }
        cond: Expr<P>,
        body: Block<P>,
    },
    Break,
    Continue,
}
