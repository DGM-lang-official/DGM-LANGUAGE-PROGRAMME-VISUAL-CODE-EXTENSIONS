pub mod analyzer;
pub mod ast;
pub mod environment;
pub mod error;
pub mod formatter;
pub mod interpreter;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod stdlib;
pub mod token;

use error::DgmError;
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;
use std::sync::Arc;
use token::Token;

pub use ast::{Expr, ExprKind, Span, Stmt, StmtKind};
pub use error::{ErrorCode, ErrorSnapshot, StackFrame};
pub use token::{Token as DgmToken, TokenKind};
pub use analyzer::{
    analyze_named_source,
    analyze_source,
    AnalysisResult as DgmAnalysisResult,
    ImportInfo,
    ModuleInfo,
    SymbolInfo,
    SymbolKind,
};
pub use formatter::{format_named_source, format_source};

pub fn tokenize_source(source: &str) -> Result<Vec<Token>, DgmError> {
    tokenize_named_source(source, "<source>")
}

pub fn tokenize_named_source(source: &str, source_name: impl Into<String>) -> Result<Vec<Token>, DgmError> {
    let mut lexer = Lexer::with_file(source, Arc::new(source_name.into()));
    lexer.tokenize()
}

pub fn parse_tokens(tokens: Vec<Token>) -> Result<Vec<ast::Stmt>, DgmError> {
    let mut parser = Parser::new(tokens);
    parser.parse()
}

pub fn parse_source(source: &str) -> Result<Vec<ast::Stmt>, DgmError> {
    parse_named_source(source, "<source>")
}

pub fn parse_named_source(source: &str, source_name: impl Into<String>) -> Result<Vec<ast::Stmt>, DgmError> {
    let tokens = tokenize_named_source(source, source_name)?;
    parse_tokens(tokens)
}

pub fn validate_source(source: &str) -> Result<(), DgmError> {
    validate_named_source(source, "<source>")
}

pub fn validate_named_source(source: &str, source_name: impl Into<String>) -> Result<(), DgmError> {
    let source_name = source_name.into();
    let result = analyzer::analyze_named_source(source, source_name)?;
    if let Some(err) = result.diagnostics.first() {
        Err(err.clone())
    } else {
        Ok(())
    }
}

pub fn run_source(source: &str) -> Result<(), DgmError> {
    run_named_source(source, "<source>")
}

pub fn run_named_source(source: &str, source_name: impl Into<String>) -> Result<(), DgmError> {
    let source_name = source_name.into();
    let stmts = parse_named_source(source, source_name.clone())?;
    let mut interp = Interpreter::new(Arc::new(source_name));
    interp.run(stmts)
}
