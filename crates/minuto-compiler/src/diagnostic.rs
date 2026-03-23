use std::ops::Range;

use ariadne::{Color, Label, Report, ReportKind, Source};

use crate::common::Span;
use crate::errors::{CompilerError, LexerError, ParserError, ResolverError, TypeCheckerError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Lexer,
    Parser,
    Resolver,
    TypeChecker,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Lexer => write!(f, "lexer"),
            Phase::Parser => write!(f, "parser"),
            Phase::Resolver => write!(f, "resolver"),
            Phase::TypeChecker => write!(f, "type checker"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub phase: Phase,
    pub span: Span,
    pub message: String,
    pub code: Option<&'static str>,
    pub label: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn error(phase: Phase, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            phase,
            span,
            message: message.into(),
            code: None,
            label: None,
            note: None,
        }
    }

    pub fn warning(phase: Phase, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            phase,
            span,
            message: message.into(),
            code: None,
            label: None,
            note: None,
        }
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn eprint(&self, filename: &str, source: &str) {
        self.build_report(filename)
            .eprint((filename, Source::from(source)))
            .unwrap();
    }

    pub fn write<W: std::io::Write>(&self, w: &mut W, filename: &str, source: &str) {
        self.build_report(filename)
            .write((filename, Source::from(source)), w)
            .unwrap();
    }

    fn build_report<'a>(&self, filename: &'a str) -> Report<'a, (&'a str, Range<usize>)> {
        let kind = match self.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
        };

        let color = match self.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
        };

        let header = match self.code {
            Some(code) => format!("[{}][{}] {}", self.phase, code, self.message),
            None => format!("[{}] {}", self.phase, self.message),
        };

        let span_range = self.span.clone().into_range();
        let mut builder = Report::build(kind, (filename, span_range.clone())).with_message(&header);

        let mut label = Label::new((filename, span_range)).with_color(color);
        if let Some(ref msg) = self.label {
            label = label.with_message(msg);
        }
        builder = builder.with_label(label);

        if let Some(ref note) = self.note {
            builder = builder.with_note(note);
        }

        builder.finish()
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match self.code {
            Some(code) => write!(f, "{}[{}][{}]: {}", kind, self.phase, code, self.message),
            None => write!(f, "{}[{}]: {}", kind, self.phase, self.message),
        }
    }
}

// ── From impls for typed errors ──

impl From<(LexerError, Span)> for Diagnostic {
    fn from((error, span): (LexerError, Span)) -> Self {
        let code = error.code();
        Diagnostic::error(Phase::Lexer, span, error.to_string()).with_code(code)
    }
}

impl From<(ParserError, Span)> for Diagnostic {
    fn from((error, span): (ParserError, Span)) -> Self {
        let code = error.code();
        Diagnostic::error(Phase::Parser, span, error.to_string()).with_code(code)
    }
}

impl From<(ResolverError, Span)> for Diagnostic {
    fn from((error, span): (ResolverError, Span)) -> Self {
        let code = error.code();
        Diagnostic::error(Phase::Resolver, span, error.to_string()).with_code(code)
    }
}

impl From<(TypeCheckerError, Span)> for Diagnostic {
    fn from((error, span): (TypeCheckerError, Span)) -> Self {
        let code = error.code();
        Diagnostic::error(Phase::TypeChecker, span, error.to_string()).with_code(code)
    }
}
