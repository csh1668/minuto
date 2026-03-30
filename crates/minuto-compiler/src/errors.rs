use std::fmt;

pub trait CompilerError: fmt::Display {
    fn code(&self) -> &'static str;
}

// ── Lexer Errors (E0001–E0006) ──

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerError {
    UnexpectedCharacter { character: String },
    InvalidIntLiteral { literal: String, reason: String },
    InvalidCharLiteral { literal: String },
    InvalidEscapeSequence { sequence: char },
    UnterminatedString,
    UnterminatedBlockComment,
}

impl CompilerError for LexerError {
    fn code(&self) -> &'static str {
        match self {
            LexerError::UnexpectedCharacter { .. } => "E0001",
            LexerError::InvalidIntLiteral { .. } => "E0002",
            LexerError::InvalidCharLiteral { .. } => "E0003",
            LexerError::InvalidEscapeSequence { .. } => "E0004",
            LexerError::UnterminatedString => "E0005",
            LexerError::UnterminatedBlockComment => "E0006",
        }
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerError::UnexpectedCharacter { character } => {
                write!(f, "unexpected character: '{}'", character)
            }
            LexerError::InvalidIntLiteral { literal, reason } => {
                write!(f, "invalid integer literal '{}': {}", literal, reason)
            }
            LexerError::InvalidCharLiteral { literal } => {
                write!(f, "invalid character literal: {}", literal)
            }
            LexerError::InvalidEscapeSequence { sequence } => {
                write!(f, "invalid escape sequence: '\\{}'", sequence)
            }
            LexerError::UnterminatedString => {
                write!(f, "unterminated string literal")
            }
            LexerError::UnterminatedBlockComment => {
                write!(f, "unterminated block comment")
            }
        }
    }
}

// ── Parser Errors (E0101–E0108) ──

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEof { expected: String },
    ExpectedExpression { found: String },
    ExpectedType { found: String },
    ExpectedIdentifier { found: String },
}

impl CompilerError for ParserError {
    fn code(&self) -> &'static str {
        match self {
            ParserError::UnexpectedToken { .. } => "E0101",
            ParserError::UnexpectedEof { .. } => "E0102",
            ParserError::ExpectedExpression { .. } => "E0103",
            ParserError::ExpectedType { .. } => "E0104",
            ParserError::ExpectedIdentifier { .. } => "E0105",
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, found } => {
                write!(f, "expected {}, found {}", expected, found)
            }
            ParserError::UnexpectedEof { expected } => {
                write!(f, "unexpected end of file, expected {}", expected)
            }
            ParserError::ExpectedExpression { found } => {
                write!(f, "expected expression, found {}", found)
            }
            ParserError::ExpectedType { found } => {
                write!(f, "expected type, found {}", found)
            }
            ParserError::ExpectedIdentifier { found } => {
                write!(f, "expected identifier, found {}", found)
            }
        }
    }
}

// ── Resolver Errors (E0201–E0209) ──

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    UndefinedVariable { name: String },
    UndefinedFunction { name: String },
    UndefinedType { name: String },
    UndefinedField { struct_name: String, field: String },
    UndefinedMethod { struct_name: String, method: String },
    DuplicateDefinition { name: String },
    DuplicateField { struct_name: String, field: String },
    MainNotFound,
    InvalidMainSignature { signature: String },
    ReservedIdentifier { name: String },
    BreakOutsideLoop,
    ContinueOutsideLoop,
}

impl CompilerError for ResolverError {
    fn code(&self) -> &'static str {
        match self {
            ResolverError::UndefinedVariable { .. } => "E0201",
            ResolverError::UndefinedFunction { .. } => "E0202",
            ResolverError::UndefinedType { .. } => "E0203",
            ResolverError::UndefinedField { .. } => "E0204",
            ResolverError::UndefinedMethod { .. } => "E0205",
            ResolverError::DuplicateDefinition { .. } => "E0206",
            ResolverError::DuplicateField { .. } => "E0210",
            ResolverError::MainNotFound => "E0207",
            ResolverError::InvalidMainSignature { .. } => "E0208",
            ResolverError::ReservedIdentifier { .. } => "E0209",
            ResolverError::BreakOutsideLoop => "E0211",
            ResolverError::ContinueOutsideLoop => "E0212",
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverError::UndefinedVariable { name } => {
                write!(f, "undefined variable '{}'", name)
            }
            ResolverError::UndefinedFunction { name } => {
                write!(f, "undefined function '{}'", name)
            }
            ResolverError::UndefinedType { name } => {
                write!(f, "undefined type '{}'", name)
            }
            ResolverError::UndefinedField { struct_name, field } => {
                write!(f, "struct '{}' has no field '{}'", struct_name, field)
            }
            ResolverError::UndefinedMethod {
                struct_name,
                method,
            } => {
                write!(f, "struct '{}' has no method '{}'", struct_name, method)
            }
            ResolverError::DuplicateDefinition { name } => {
                write!(f, "duplicate definition of '{}'", name)
            }
            ResolverError::MainNotFound => {
                write!(f, "entry point 'main' function not found")
            }
            ResolverError::InvalidMainSignature { signature } => {
                write!(
                    f,
                    "invalid main signature: expected 'fn main()', found '{}'",
                    signature,
                )
            }
            ResolverError::DuplicateField { struct_name, field } => {
                write!(f, "duplicate field '{}' in struct '{}'", field, struct_name)
            }
            ResolverError::ReservedIdentifier { name } => {
                write!(
                    f,
                    "'{}' is a reserved builtin identifier and cannot be used as a variable or function name",
                    name,
                )
            }
            ResolverError::BreakOutsideLoop => {
                write!(f, "'break' used outside of a loop")
            }
            ResolverError::ContinueOutsideLoop => {
                write!(f, "'continue' used outside of a loop")
            }
        }
    }
}

// ── TypeChecker Errors (E0301–E0317) ──

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeCheckerError {
    TypeMismatch {
        expected: String,
        found: String,
    },
    InvalidBinaryOp {
        op: String,
        lhs: String,
        rhs: String,
    },
    InvalidUnaryOp {
        op: String,
        operand: String,
    },
    InvalidDereference {
        found: String,
    },
    InvalidFieldAccess {
        found: String,
    },
    InvalidIndexing {
        found: String,
    },
    InvalidFunctionCall {
        found: String,
    },
    WrongArgCount {
        expected: usize,
        found: usize,
    },
    InvalidPointerArithmetic {
        found: String,
    },
    AssignToConst {
        name: String,
    },
    AssignToReadonly {
        name: String,
    },
    MissingReturn {
        function: String,
    },
    BreakOutsideLoop,
    ContinueOutsideLoop,
    PrintFormatMustBeStringLiteral,
    PrintArgCountMismatch {
        expected: usize,
        found: usize,
    },
    PrintArgNotPrintable {
        index: usize,
        found: String,
    },
}

impl CompilerError for TypeCheckerError {
    fn code(&self) -> &'static str {
        match self {
            TypeCheckerError::TypeMismatch { .. } => "E0301",
            TypeCheckerError::InvalidBinaryOp { .. } => "E0302",
            TypeCheckerError::InvalidUnaryOp { .. } => "E0303",
            TypeCheckerError::InvalidDereference { .. } => "E0304",
            TypeCheckerError::InvalidFieldAccess { .. } => "E0305",
            TypeCheckerError::InvalidIndexing { .. } => "E0306",
            TypeCheckerError::InvalidFunctionCall { .. } => "E0307",
            TypeCheckerError::WrongArgCount { .. } => "E0308",
            TypeCheckerError::InvalidPointerArithmetic { .. } => "E0309",
            TypeCheckerError::AssignToConst { .. } => "E0310",
            TypeCheckerError::AssignToReadonly { .. } => "E0311",
            TypeCheckerError::MissingReturn { .. } => "E0312",
            TypeCheckerError::BreakOutsideLoop => "E0313",
            TypeCheckerError::ContinueOutsideLoop => "E0314",
            TypeCheckerError::PrintFormatMustBeStringLiteral => "E0315",
            TypeCheckerError::PrintArgCountMismatch { .. } => "E0316",
            TypeCheckerError::PrintArgNotPrintable { .. } => "E0317",
        }
    }
}

impl fmt::Display for TypeCheckerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeCheckerError::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "type mismatch: expected '{}', found '{}'",
                    expected, found
                )
            }
            TypeCheckerError::InvalidBinaryOp { op, lhs, rhs } => {
                write!(f, "invalid binary operation: '{}' {} '{}'", lhs, op, rhs)
            }
            TypeCheckerError::InvalidUnaryOp { op, operand } => {
                write!(f, "invalid unary operation: {}{}", op, operand)
            }
            TypeCheckerError::InvalidDereference { found } => {
                write!(f, "cannot dereference type '{}'", found)
            }
            TypeCheckerError::InvalidFieldAccess { found } => {
                write!(f, "cannot access field on type '{}'", found)
            }
            TypeCheckerError::InvalidIndexing { found } => {
                write!(f, "cannot index type '{}'", found)
            }
            TypeCheckerError::InvalidFunctionCall { found } => {
                write!(f, "cannot call non-function type '{}'", found)
            }
            TypeCheckerError::WrongArgCount { expected, found } => {
                write!(
                    f,
                    "wrong number of arguments: expected {}, found {}",
                    expected, found,
                )
            }
            TypeCheckerError::InvalidPointerArithmetic { found } => {
                write!(f, "pointer arithmetic requires 'ptr<T>', found '{}'", found)
            }
            TypeCheckerError::AssignToConst { name } => {
                write!(f, "cannot assign to const variable '{}'", name)
            }
            TypeCheckerError::AssignToReadonly { name } => {
                write!(f, "cannot write through readonly reference '{}'", name)
            }
            TypeCheckerError::MissingReturn { function } => {
                write!(f, "function '{}' is missing a return statement", function)
            }
            TypeCheckerError::BreakOutsideLoop => {
                write!(f, "'break' used outside of a loop")
            }
            TypeCheckerError::ContinueOutsideLoop => {
                write!(f, "'continue' used outside of a loop")
            }
            TypeCheckerError::PrintFormatMustBeStringLiteral => {
                write!(f, "first argument to 'std::print' must be a string literal",)
            }
            TypeCheckerError::PrintArgCountMismatch { expected, found } => {
                write!(
                    f,
                    "format string has {} placeholder(s) but {} argument(s) were supplied",
                    expected, found,
                )
            }
            TypeCheckerError::PrintArgNotPrintable { index, found } => {
                write!(
                    f,
                    "argument {} has type '{}' which cannot be formatted with '{{}}'",
                    index, found,
                )
            }
        }
    }
}
