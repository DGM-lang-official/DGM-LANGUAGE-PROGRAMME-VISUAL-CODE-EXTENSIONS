use crate::ast::Span;
use crate::error::{DgmError, ErrorCode};
use crate::token::{Token, TokenKind};
use std::sync::Arc;

pub struct Lexer {
    source: Vec<char>,
    file: Arc<String>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self::with_file(source, Arc::new("<source>".to_string()))
    }

    pub fn with_file(source: &str, file: Arc<String>) -> Self {
        Self::with_context(source, file, 1, 1)
    }

    pub fn with_context(source: &str, file: Arc<String>, line: usize, col: usize) -> Self {
        Self {
            source: source.chars().collect(),
            file,
            pos: 0,
            line,
            col,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, DgmError> {
        let mut tokens = vec![];
        while !self.is_at_end() {
            let ch = self.current();
            match ch {
                ' ' | '\t' | '\r' => self.advance(),
                '\n' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Newline, "\\n", line, col));
                    self.advance();
                }
                '#' => {
                    while !self.is_at_end() && self.current() != '\n' {
                        self.advance();
                    }
                }
                '(' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::LParen, "(", line, col));
                    self.advance();
                }
                ')' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::RParen, ")", line, col));
                    self.advance();
                }
                '{' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::LBrace, "{", line, col));
                    self.advance();
                }
                '}' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::RBrace, "}", line, col));
                    self.advance();
                }
                '[' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::LBracket, "[", line, col));
                    self.advance();
                }
                ']' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::RBracket, "]", line, col));
                    self.advance();
                }
                ',' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Comma, ",", line, col));
                    self.advance();
                }
                ':' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Colon, ":", line, col));
                    self.advance();
                }
                ';' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Semicolon, ";", line, col));
                    self.advance();
                }
                '?' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Question, "?", line, col));
                    self.advance();
                }
                '~' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Tilde, "~", line, col));
                    self.advance();
                }
                '^' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Caret, "^", line, col));
                    self.advance();
                }
                '.' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '.' {
                        self.advance();
                        if !self.is_at_end() && self.current() == '.' {
                            tokens.push(self.token(TokenKind::DotDotDot, "...", line, col));
                            self.advance();
                        } else {
                            tokens.push(self.token(TokenKind::DotDot, "..", line, col));
                        }
                    } else {
                        tokens.push(self.token(TokenKind::Dot, ".", line, col));
                    }
                }
                '&' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Ampersand, "&", line, col));
                    self.advance();
                }
                '|' => {
                    let (line, col) = self.current_position();
                    tokens.push(self.token(TokenKind::Pipe, "|", line, col));
                    self.advance();
                }
                '+' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::PlusEq, "+=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Plus, "+", line, col));
                    }
                }
                '-' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::MinusEq, "-=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Minus, "-", line, col));
                    }
                }
                '*' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '*' {
                        tokens.push(self.token(TokenKind::StarStar, "**", line, col));
                        self.advance();
                    } else if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::StarEq, "*=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Star, "*", line, col));
                    }
                }
                '/' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '*' {
                        // Multi-line comment
                        self.advance();
                        let mut depth = 1;
                        while !self.is_at_end() && depth > 0 {
                            if self.current() == '/' && self.peek_next() == Some('*') {
                                depth += 1;
                                self.advance();
                                self.advance();
                            } else if self.current() == '*' && self.peek_next() == Some('/') {
                                depth -= 1;
                                self.advance();
                                self.advance();
                            } else {
                                self.advance();
                            }
                        }
                        if depth > 0 {
                            return Err(self.lex_error("unterminated block comment", line, col));
                        }
                    } else if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::SlashEq, "/=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Slash, "/", line, col));
                    }
                }
                '%' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::PercentEq, "%=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Percent, "%", line, col));
                    }
                }
                '=' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::EqEq, "==", line, col));
                        self.advance();
                    } else if !self.is_at_end() && self.current() == '>' {
                        tokens.push(self.token(TokenKind::Arrow, "=>", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Eq, "=", line, col));
                    }
                }
                '!' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::BangEq, "!=", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Bang, "!", line, col));
                    }
                }
                '<' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::LtEq, "<=", line, col));
                        self.advance();
                    } else if !self.is_at_end() && self.current() == '<' {
                        tokens.push(self.token(TokenKind::ShiftLeft, "<<", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Lt, "<", line, col));
                    }
                }
                '>' => {
                    let (line, col) = self.current_position();
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(self.token(TokenKind::GtEq, ">=", line, col));
                        self.advance();
                    } else if !self.is_at_end() && self.current() == '>' {
                        tokens.push(self.token(TokenKind::ShiftRight, ">>", line, col));
                        self.advance();
                    } else {
                        tokens.push(self.token(TokenKind::Gt, ">", line, col));
                    }
                }
                'f' if self.peek_next() == Some('"') => {
                    let fstring_tokens = self.lex_fstring()?;
                    tokens.extend(fstring_tokens);
                }
                '"' => tokens.push(self.lex_string()?),
                c if c.is_ascii_digit() => tokens.push(self.lex_number()?),
                c if c.is_alphabetic() || c == '_' => tokens.push(self.lex_ident()),
                other => {
                    let (line, col) = self.current_position();
                    return Err(self.lex_error(format!("unexpected character '{}'", other), line, col));
                }
            }
        }
        tokens.push(self.token(TokenKind::EOF, "", self.line, self.col));
        Ok(tokens)
    }

    fn lex_string(&mut self) -> Result<Token, DgmError> {
        let (line, col) = self.current_position();
        self.advance();
        let mut value = String::new();
        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(self.lex_error("unterminated string escape", line, col));
                }
                match self.current() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '0' => value.push('\0'),
                    other => value.push(other),
                }
            } else {
                value.push(self.current());
            }
            self.advance();
        }
        if self.is_at_end() {
            return Err(self.lex_error("unterminated string", line, col));
        }
        self.advance();
        Ok(self.token(TokenKind::StringLit, value, line, col))
    }

    fn lex_fstring(&mut self) -> Result<Vec<Token>, DgmError> {
        let (line, col) = self.current_position();
        self.advance();
        self.advance();

        let mut tokens = vec![self.token(TokenKind::FStringStart, "f\"", line, col)];
        let mut buf = String::new();
        let mut buf_line = line;
        let mut buf_col = col + 2;

        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '{' {
                if !buf.is_empty() {
                    tokens.push(self.token(TokenKind::StringLit, buf.clone(), buf_line, buf_col));
                    buf.clear();
                }
                let (brace_line, brace_col) = self.current_position();
                tokens.push(self.token(TokenKind::LBrace, "{", brace_line, brace_col));
                self.advance();

                let mut depth = 1;
                let inner_line = self.line;
                let inner_col = self.col;
                let mut inner_src = String::new();
                while !self.is_at_end() && depth > 0 {
                    if self.current() == '{' {
                        depth += 1;
                    }
                    if self.current() == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    inner_src.push(self.current());
                    self.advance();
                }

                let mut inner_lexer = Lexer::with_context(&inner_src, Arc::clone(&self.file), inner_line, inner_col);
                let inner_tokens = inner_lexer.tokenize()?;
                for token in inner_tokens {
                    if token.kind != TokenKind::EOF {
                        tokens.push(token);
                    }
                }

                if !self.is_at_end() && self.current() == '}' {
                    let (rbrace_line, rbrace_col) = self.current_position();
                    tokens.push(self.token(TokenKind::RBrace, "}", rbrace_line, rbrace_col));
                    self.advance();
                }

                buf_line = self.line;
                buf_col = self.col;
            } else if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(self.lex_error("unterminated f-string escape", line, col));
                }
                match self.current() {
                    'n' => buf.push('\n'),
                    't' => buf.push('\t'),
                    '\\' => buf.push('\\'),
                    '"' => buf.push('"'),
                    '{' => buf.push('{'),
                    '}' => buf.push('}'),
                    other => buf.push(other),
                }
                self.advance();
            } else {
                buf.push(self.current());
                self.advance();
            }
        }

        if !buf.is_empty() {
            tokens.push(self.token(TokenKind::StringLit, buf, buf_line, buf_col));
        }
        if self.is_at_end() {
            return Err(self.lex_error("unterminated f-string", line, col));
        }
        self.advance();
        tokens.push(self.token(TokenKind::RParen, ")", line, col));
        Ok(tokens)
    }

    fn lex_number(&mut self) -> Result<Token, DgmError> {
        let (line, col) = self.current_position();
        let mut value = String::new();

        if self.current() == '0' && self.peek_next().is_some_and(|c| c == 'x' || c == 'X') {
            value.push(self.current());
            self.advance();
            value.push(self.current());
            self.advance();
            while !self.is_at_end() && self.current().is_ascii_hexdigit() {
                value.push(self.current());
                self.advance();
            }
            let parsed = i64::from_str_radix(&value[2..], 16)
                .map_err(|_| self.lex_error(format!("invalid hex '{}'", value), line, col))?;
            return Ok(self.token(TokenKind::IntLit, parsed.to_string(), line, col));
        }

        if self.current() == '0' && self.peek_next().is_some_and(|c| c == 'b' || c == 'B') {
            value.push(self.current());
            self.advance();
            value.push(self.current());
            self.advance();
            while !self.is_at_end() && matches!(self.current(), '0' | '1') {
                value.push(self.current());
                self.advance();
            }
            let parsed = i64::from_str_radix(&value[2..], 2)
                .map_err(|_| self.lex_error(format!("invalid binary '{}'", value), line, col))?;
            return Ok(self.token(TokenKind::IntLit, parsed.to_string(), line, col));
        }

        if self.current() == '0' && self.peek_next().is_some_and(|c| c == 'o' || c == 'O') {
            value.push(self.current());
            self.advance();
            value.push(self.current());
            self.advance();
            while !self.is_at_end() && self.current().is_digit(8) {
                value.push(self.current());
                self.advance();
            }
            let parsed = i64::from_str_radix(&value[2..], 8)
                .map_err(|_| self.lex_error(format!("invalid octal '{}'", value), line, col))?;
            return Ok(self.token(TokenKind::IntLit, parsed.to_string(), line, col));
        }

        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
            if self.current() != '_' {
                value.push(self.current());
            }
            self.advance();
        }

        if !self.is_at_end() && self.current() == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            value.push('.');
            self.advance();
            while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
                if self.current() != '_' {
                    value.push(self.current());
                }
                self.advance();
            }
            if !self.is_at_end() && matches!(self.current(), 'e' | 'E') {
                value.push('e');
                self.advance();
                if !self.is_at_end() && matches!(self.current(), '+' | '-') {
                    value.push(self.current());
                    self.advance();
                }
                while !self.is_at_end() && self.current().is_ascii_digit() {
                    value.push(self.current());
                    self.advance();
                }
            }
            return Ok(self.token(TokenKind::FloatLit, value, line, col));
        }

        if !self.is_at_end() && matches!(self.current(), 'e' | 'E') {
            value.push('e');
            self.advance();
            if !self.is_at_end() && matches!(self.current(), '+' | '-') {
                value.push(self.current());
                self.advance();
            }
            while !self.is_at_end() && self.current().is_ascii_digit() {
                value.push(self.current());
                self.advance();
            }
            return Ok(self.token(TokenKind::FloatLit, value, line, col));
        }

        Ok(self.token(TokenKind::IntLit, value, line, col))
    }

    fn lex_ident(&mut self) -> Token {
        let (line, col) = self.current_position();
        let mut ident = String::new();
        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            ident.push(self.current());
            self.advance();
        }
        let kind = match ident.as_str() {
            "imprt" | "import" => TokenKind::Imprt,
            "writ" => TokenKind::Writ,
            "def" | "fn" => TokenKind::Def,
            "retrun" | "return" => TokenKind::Retrun,
            "iff" | "if" => TokenKind::Iff,
            "elseif" => TokenKind::Elseif,
            "els" | "else" => TokenKind::Els,
            "fr" | "for" => TokenKind::Fr,
            "whl" | "while" => TokenKind::Whl,
            "brk" | "break" => TokenKind::Brk,
            "cont" | "continue" => TokenKind::Cont,
            "tru" | "true" => TokenKind::Tru,
            "fals" | "false" => TokenKind::Fals,
            "nul" | "null" => TokenKind::Nul,
            "let" => TokenKind::Let,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "cls" | "class" => TokenKind::Cls,
            "new" => TokenKind::New,
            "ths" | "this" => TokenKind::Ths,
            "in" => TokenKind::In,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "match" => TokenKind::Match,
            "extends" => TokenKind::Extends,
            "lam" => TokenKind::Lam,
            "const" => TokenKind::Const,
            "super" => TokenKind::Super,
            _ => TokenKind::Ident,
        };
        self.token(kind, ident, line, col)
    }

    fn token(&self, kind: TokenKind, lexeme: impl Into<String>, line: usize, col: usize) -> Token {
        Token::new(kind, lexeme, Arc::clone(&self.file), line, col)
    }

    fn lex_error(&self, message: impl Into<String>, line: usize, col: usize) -> DgmError {
        DgmError::new(ErrorCode::LexError, message).with_span(Span::new(Arc::clone(&self.file), line, col))
    }

    fn current_position(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    fn current(&self) -> char {
        self.source[self.pos]
    }

    fn advance(&mut self) {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    #[test]
    fn tokenizes_literal_aliases() {
        let mut lexer = Lexer::new("tru fals nul true false null");
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().take(6).map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Tru,
                TokenKind::Fals,
                TokenKind::Nul,
                TokenKind::Tru,
                TokenKind::Fals,
                TokenKind::Nul,
            ]
        );
    }

    #[test]
    fn tokenizes_keyword_aliases() {
        let mut lexer = Lexer::new(
            "fn import return if else for while break continue class this \
             def imprt retrun iff els fr whl brk cont cls ths",
        );
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().take(22).map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Def,
                TokenKind::Imprt,
                TokenKind::Retrun,
                TokenKind::Iff,
                TokenKind::Els,
                TokenKind::Fr,
                TokenKind::Whl,
                TokenKind::Brk,
                TokenKind::Cont,
                TokenKind::Cls,
                TokenKind::Ths,
                TokenKind::Def,
                TokenKind::Imprt,
                TokenKind::Retrun,
                TokenKind::Iff,
                TokenKind::Els,
                TokenKind::Fr,
                TokenKind::Whl,
                TokenKind::Brk,
                TokenKind::Cont,
                TokenKind::Cls,
                TokenKind::Ths,
            ]
        );
    }
}
