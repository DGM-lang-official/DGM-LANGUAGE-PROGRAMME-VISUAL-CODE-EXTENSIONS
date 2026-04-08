use crate::ast::Span;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackFrame {
    pub function: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    LexError,
    UnexpectedToken,
    ParseError,
    ExpectedToken,
    UndefinedVariable,
    InvalidCall,
    DivideByZero,
    InvalidIndex,
    RuntimeError,
    ImportFail,
    CircularImport,
    ProgramNotAllowed,
    ShellExecutionDisabled,
    ThrownValue,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::LexError => "E001",
            ErrorCode::UnexpectedToken => "E301",
            ErrorCode::ParseError => "E300",
            ErrorCode::ExpectedToken => "E302",
            ErrorCode::UndefinedVariable => "E100",
            ErrorCode::InvalidCall => "E101",
            ErrorCode::DivideByZero => "E102",
            ErrorCode::InvalidIndex => "E103",
            ErrorCode::RuntimeError => "E199",
            ErrorCode::ImportFail => "E200",
            ErrorCode::CircularImport => "E201",
            ErrorCode::ProgramNotAllowed => "E410",
            ErrorCode::ShellExecutionDisabled => "E411",
            ErrorCode::ThrownValue => "E400",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorSnapshot {
    pub code: String,
    pub message: String,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
}

#[derive(Debug, Clone)]
pub struct DgmError {
    pub code: ErrorCode,
    pub message: String,
    pub span: Option<Span>,
    pub stack: Vec<StackFrame>,
}

impl DgmError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            span: None,
            stack: vec![],
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_stack(mut self, stack: Vec<StackFrame>) -> Self {
        self.stack = stack;
        self
    }

    pub fn with_fallback_span(mut self, span: Span) -> Self {
        if self.span.is_none() {
            self.span = Some(span);
        }
        self
    }

    pub fn with_fallback_stack(mut self, stack: &[StackFrame]) -> Self {
        if self.stack.is_empty() {
            self.stack = stack.to_vec();
        }
        self
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::RuntimeError, message)
    }

    pub fn runtime_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(code, message)
    }

    pub fn undefined_variable(name: &str) -> Self {
        Self::new(
            ErrorCode::UndefinedVariable,
            format!("undefined variable '{}'", name),
        )
    }

    pub fn invalid_call(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidCall, message)
    }

    pub fn divide_by_zero(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DivideByZero, message)
    }

    pub fn invalid_index(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidIndex, message)
    }

    pub fn import_fail(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ImportFail, message)
    }

    pub fn circular_import(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::CircularImport, message)
    }

    pub fn thrown(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ThrownValue, message)
    }

    pub fn snapshot(&self) -> ErrorSnapshot {
        ErrorSnapshot {
            code: self.code.as_str().to_string(),
            message: self.message.clone(),
            span: self.span.clone(),
            stack: self.stack.clone(),
        }
    }

    pub fn summary(&self) -> String {
        format!("[{}] {}", self.code.as_str(), self.message)
    }

    pub fn render(&self, fallback_name: &str, fallback_source: &str) -> String {
        let mut rendered = format!("{}\n", self.summary());

        if let Some(span) = &self.span {
            rendered.push_str(&self.render_span(span, fallback_name, fallback_source));
        }

        if !self.stack.is_empty() {
            rendered.push_str("Stack trace:\n");
            for frame in self.stack.iter().rev() {
                rendered.push_str(&format!(
                    "  at {} ({}:{}:{})\n",
                    frame.function,
                    frame.span.file,
                    frame.span.line,
                    frame.span.col
                ));
            }
        }

        rendered
    }

    fn render_span(&self, span: &Span, fallback_name: &str, fallback_source: &str) -> String {
        let source = if span.file.as_ref() == fallback_name {
            Some(fallback_source.to_string())
        } else if !span.file.starts_with('<') {
            std::fs::read_to_string(span.file.as_ref()).ok()
        } else {
            None
        };

        let line_text = source
            .as_deref()
            .and_then(|src| src.lines().nth(span.line.saturating_sub(1)))
            .unwrap_or("");
        let gutter = span.line.to_string();
        let gutter_pad = " ".repeat(gutter.len());
        let caret_pad = " ".repeat(span.col.saturating_sub(1));

        format!(
            " --> {}:{}:{}\n{} |\n{} | {}\n{} | {}^\n",
            span.file,
            span.line,
            span.col,
            gutter_pad,
            gutter,
            line_text,
            gutter_pad,
            caret_pad
        )
    }
}

impl fmt::Display for DgmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl std::error::Error for DgmError {}
