use crate::ast::Span;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TokenKind {
    // Internal token kinds stay stable even when the lexer accepts compatible
    // public spellings such as fn/def or import/imprt.

    IntLit,
    FloatLit,
    StringLit,
    Ident,
    Imprt,
    Writ,
    Def,
    Retrun,
    Iff,
    Elseif,
    Els,
    Fr,
    Whl,
    Brk,
    Cont,
    Tru,
    Fals,
    Nul,
    Let,
    And,
    Or,
    Not,
    Cls,
    New,
    Ths,
    In,
    Try,
    Catch,
    Finally,
    Throw,
    Match,
    Extends,
    Lam,
    Const,
    Super,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Newline,
    EOF,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Arrow,
    DotDot,
    DotDotDot,
    Question,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    ShiftLeft,
    ShiftRight,
    Bang,
    PercentEq,
    FStringStart,
}

#[derive(Debug, Clone, Serialize)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    #[serde(skip_serializing)]
    pub file: Arc<String>,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(
        kind: TokenKind,
        lexeme: impl Into<String>,
        file: Arc<String>,
        line: usize,
        col: usize,
    ) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            file,
            line,
            col,
        }
    }

    pub fn span(&self) -> Span {
        Span::new(Arc::clone(&self.file), self.line, self.col)
    }
}
