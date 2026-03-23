mod decl;
mod expr;
mod stmt;
mod ty;

pub use crate::common::*;
pub use decl::*;
pub use expr::*;
pub use stmt::*;
pub use ty::*;

pub type ParsedProgram = Program<Parsed>;
pub type ParsedExpr = Expr<Parsed>;
pub type ParsedStmt = Stmt<Parsed>;

pub type ResolvedProgram = Program<Resolved>;
pub type ResolvedExpr = Expr<Resolved>;

pub type TypedProgram = Program<Typed>;
pub type TypedExpr = Expr<Typed>;
