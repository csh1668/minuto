use crate::ast::ty::TypeExpr;
use crate::common::{Ident, Phase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr<P: Phase> {
    pub ann: P::Ann,
    pub ty: P::TypeAnn,
    pub kind: ExprKind<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind<P: Phase> {
    // Literals
    IntLit(i64),
    CharLit(char),
    StrLit(String),

    // Name reference
    Ident(Ident<P>),

    // Unary/Binary operators
    Unary(UnaryOp, Box<Expr<P>>),
    Binary(BinOp, Box<Expr<P>>, Box<Expr<P>>),

    Assign {
        lhs: Box<Expr<P>>,
        rhs: Box<Expr<P>>,
    },
    Call {
        callee: Box<Expr<P>>,
        args: Vec<Expr<P>>,
    },
    Field {
        base: Box<Expr<P>>,
        field: String,
    },
    Index {
        base: Box<Expr<P>>,
        index: Box<Expr<P>>,
    },
    MethodCall {
        base: Box<Expr<P>>,
        method: String,
        args: Vec<Expr<P>>,
    },
    StaticCall {
        receiver: StaticReceiver,
        method: String,
        args: Vec<Expr<P>>,
    },

    // 나중에 ModCall로 통합될 수도
    StdCall {
        func: String,
        args: Vec<Expr<P>>,
    },

    StructLit {
        name: String,
        fields: Vec<FieldInit<P>>,
    },

    Alloc {
        ty: TypeExpr<P>,
        count: Box<Expr<P>>,
    },
    Free {
        expr: Box<Expr<P>>,
    },
}

pub const RESERVED_NAMES: &[&str] = &["alloc", "free"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInit<P: Phase> {
    pub name: String,
    pub value: Expr<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticReceiver {
    Span,
    Ptr,
    Int,
    Char,
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum UnaryOp {
    Neg,    // -x
    Not,    // !x
    BitNot, // ~x
    Deref,  // *x
    AddrOf, // &x
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}
