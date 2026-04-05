use std::fmt;

#[derive(Debug, Clone)]
pub enum DgmError {
    LexError { msg: String, line: usize },
    ParseError { msg: String, line: usize },
    RuntimeError { msg: String },
    /// Thrown by `throw` in DGM source; can be caught with try/catch
    ThrownError { value: String },
    /// Import errors
    ImportError { msg: String },
}

impl fmt::Display for DgmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DgmError::LexError { msg, line } => write!(f, "[LexError line {}] {}", line, msg),
            DgmError::ParseError { msg, line } => write!(f, "[ParseError line {}] {}", line, msg),
            DgmError::RuntimeError { msg } => write!(f, "[RuntimeError] {}", msg),
            DgmError::ThrownError { value } => write!(f, "[ThrownError] {}", value),
            DgmError::ImportError { msg } => write!(f, "[ImportError] {}", msg),
        }
    }
}

impl std::error::Error for DgmError {}
