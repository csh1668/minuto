use crate::common::Phase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr<P: Phase> {
    Int(P::Ann),
    Char(P::Ann),
    Void(P::Ann),
    Ptr(P::Ann, Box<TypeExpr<P>>),
    Span(P::Ann, Box<TypeExpr<P>>),

    Readonly(P::Ann, Box<TypeExpr<P>>),

    Fn {
        ann: P::Ann,
        params: Vec<TypeExpr<P>>,
        ret: Box<TypeExpr<P>>,
    },

    Named(P::Ann, String),
}
