#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // [A] LANGUAGE STABILITY: Keywords frozen at v0.2.0 (see LANGUAGE_SPEC.md)
    // No new keywords without major version bump
    
    // Literals
    IntLit, FloatLit, StringLit, Ident,
    // Keywords – original
    Imprt, Writ, Def, Retrun, Iff, Elseif, Els,
    Fr, Whl, Brk, Cont, Tru, Fals, Nul, Let,
    And, Or, Not, Cls, New, Ths, In,
    // Keywords – new
    Try, Catch, Finally, Throw,
    Match, Extends, Lam,
    // Symbols – original
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Dot, Colon, Newline, EOF, Semicolon,
    // Operators – original
    Plus, Minus, Star, Slash, Percent, StarStar,
    Eq, EqEq, BangEq, Lt, Gt, LtEq, GtEq,
    PlusEq, MinusEq, StarEq, SlashEq,
    // Operators – new
    Arrow,      // =>
    DotDot,     // ..
    Question,   // ?
    Ampersand,  // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    ShiftLeft,  // <<
    ShiftRight, // >>
    Bang,       // !
    PercentEq,  // %=
    FStringStart, // f"
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, line: usize) -> Self {
        Self { kind, lexeme: lexeme.into(), line }
    }
}
