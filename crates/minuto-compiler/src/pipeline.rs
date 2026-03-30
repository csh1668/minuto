use crate::ast::{ParsedProgram, ResolvedProgram};
use crate::common::{Span, SymbolTable};
use crate::diagnostic::Diagnostic;
use crate::lexer::token::Token;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolver::Resolver;

// ── Pass trait ──

pub trait Pass {
    type Input;
    type Output;
    fn run(self, input: Self::Input) -> Result<Self::Output, Vec<Diagnostic>>;
}

// ── Pipeline (typed chain) ──

pub struct Pipeline<P> {
    pass: P,
}

impl<P: Pass> Pipeline<P> {
    pub fn start(pass: P) -> Self {
        Pipeline { pass }
    }

    pub fn then<Q: Pass<Input = P::Output>>(self, next: Q) -> Pipeline<Chain<P, Q>> {
        Pipeline {
            pass: Chain {
                first: self.pass,
                second: next,
            },
        }
    }

    pub fn run(self, input: P::Input) -> Result<P::Output, Vec<Diagnostic>> {
        self.pass.run(input)
    }
}

pub struct Chain<A, B> {
    first: A,
    second: B,
}

impl<A: Pass, B: Pass<Input = A::Output>> Pass for Chain<A, B> {
    type Input = A::Input;
    type Output = B::Output;

    fn run(self, input: Self::Input) -> Result<Self::Output, Vec<Diagnostic>> {
        let mid = self.first.run(input)?;
        self.second.run(mid)
    }
}

// ── Concrete passes ──

pub struct Lex;

impl Pass for Lex {
    type Input = String;
    type Output = Vec<(Token, Span)>;

    fn run(self, input: String) -> Result<Self::Output, Vec<Diagnostic>> {
        let mut lexer = Lexer::new(&input);
        let (tokens, errors) = lexer.tokenize();
        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}

pub struct Parse;

impl Pass for Parse {
    type Input = Vec<(Token, Span)>;
    type Output = ParsedProgram;

    fn run(self, input: Self::Input) -> Result<Self::Output, Vec<Diagnostic>> {
        Parser::new(input).parse()
    }
}

pub struct Resolve;

impl Pass for Resolve {
    type Input = ParsedProgram;
    type Output = (ResolvedProgram, SymbolTable);

    fn run(self, input: Self::Input) -> Result<Self::Output, Vec<Diagnostic>> {
        Resolver::new().resolve(&input)
    }
}
