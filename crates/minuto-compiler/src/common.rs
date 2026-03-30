use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize, // inclusive
    pub end: usize,   // exclusive
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl Span {
    pub fn into_range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

pub trait Phase: Clone + Debug + PartialEq + Eq {
    /// Annotation
    type Ann: Clone + Debug + PartialEq + Eq;
    /// (Parsed: (), Resolved+: SymbolId)
    type NameRef: Clone + Debug + PartialEq + Eq;
    /// (Parsed/Resolved: (), Typed: Ty)
    type TypeAnn: Clone + Debug + PartialEq + Eq;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident<P: Phase> {
    pub name: String,
    pub id: P::NameRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Char,
    Void,
    Ptr(Box<Ty>),
    Span(Box<Ty>),

    ReadonlyPtr(Box<Ty>),
    ReadonlySpan(Box<Ty>),

    Fn { params: Vec<Ty>, ret: Box<Ty> },

    Struct(SymbolId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Typed;

impl Phase for Parsed {
    type Ann = Span;
    type NameRef = ();
    type TypeAnn = ();
}

impl Phase for Resolved {
    type Ann = Span;
    type NameRef = SymbolId;
    type TypeAnn = ();
}

impl Phase for Typed {
    type Ann = Span;
    type NameRef = SymbolId;
    type TypeAnn = Ty;
}


#[derive(Debug, Clone)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        let idx = symbol.id.0 as usize;
        assert_eq!(idx, self.symbols.len(), "SymbolId must be sequential");
        self.symbols.push(symbol);
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        &mut self.symbols[id.0 as usize]
    }

    pub fn get_struct_info(&self, id: SymbolId) -> Option<&StructInfo> {
        match &self.get(id).kind {
            SymbolKind::Struct(info) => Some(info),
            _ => None,
        }
    }

    pub fn get_fn_sig(&self, id: SymbolId) -> Option<&FnSig> {
        match &self.get(id).kind {
            SymbolKind::Fn(sig) => Some(sig),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Var,
    Const,
    Param,
    Fn(FnSig),
    Struct(StructInfo),
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub params: Vec<ParamInfo>,
    pub ret_ty: String, // TypeExpr의 문자열 표현 (main 시그니처 검증용)
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub fields: Vec<FieldInfo>,
    pub fns: HashMap<String, SymbolId>,
}

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
}
