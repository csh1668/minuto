use crate::ast::stmt::Block;
use crate::ast::ty::TypeExpr;
use crate::common::{Ident, Phase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program<P: Phase> {
    pub decls: Vec<Decl<P>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decl<P: Phase> {
    Fn(FnDecl<P>),
    Struct(StructDecl<P>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl<P: Phase> {
    pub ann: P::Ann,
    pub name: Ident<P>,
    pub params: Vec<Param<P>>,
    pub ret_ty: TypeExpr<P>,
    pub body: Block<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param<P: Phase> {
    pub ann: P::Ann,
    pub kind: ParamKind<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind<P: Phase> {
    SelfParam { ty: Option<TypeExpr<P>> },
    Named { name: Ident<P>, ty: TypeExpr<P> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl<P: Phase> {
    pub ann: P::Ann,
    pub name: Ident<P>,
    pub fields: Vec<FieldDecl<P>>,
    pub methods: Vec<FnDecl<P>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl<P: Phase> {
    pub ann: P::Ann,
    pub name: String, // struct 필드명은 Ident가 아니라 그냥 String으로 표현.
    pub ty: TypeExpr<P>,
}
