use crate::error::DgmError;
use crate::token::{Token, TokenKind};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self { source: source.chars().collect(), pos: 0, line: 1 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, DgmError> {
        let mut tokens = vec![];
        while !self.is_at_end() {
            let ch = self.current();
            match ch {
                ' ' | '\t' | '\r' => { self.advance(); }
                '\n' => {
                    tokens.push(Token::new(TokenKind::Newline, "\\n", self.line));
                    self.line += 1;
                    self.advance();
                }
                '#' => { while !self.is_at_end() && self.current() != '\n' { self.advance(); } }
                '(' => { tokens.push(Token::new(TokenKind::LParen, "(", self.line)); self.advance(); }
                ')' => { tokens.push(Token::new(TokenKind::RParen, ")", self.line)); self.advance(); }
                '{' => { tokens.push(Token::new(TokenKind::LBrace, "{", self.line)); self.advance(); }
                '}' => { tokens.push(Token::new(TokenKind::RBrace, "}", self.line)); self.advance(); }
                '[' => { tokens.push(Token::new(TokenKind::LBracket, "[", self.line)); self.advance(); }
                ']' => { tokens.push(Token::new(TokenKind::RBracket, "]", self.line)); self.advance(); }
                ',' => { tokens.push(Token::new(TokenKind::Comma, ",", self.line)); self.advance(); }
                ':' => { tokens.push(Token::new(TokenKind::Colon, ":", self.line)); self.advance(); }
                ';' => { tokens.push(Token::new(TokenKind::Semicolon, ";", self.line)); self.advance(); }
                '?' => { tokens.push(Token::new(TokenKind::Question, "?", self.line)); self.advance(); }
                '~' => { tokens.push(Token::new(TokenKind::Tilde, "~", self.line)); self.advance(); }
                '^' => { tokens.push(Token::new(TokenKind::Caret, "^", self.line)); self.advance(); }
                '.' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '.' {
                        tokens.push(Token::new(TokenKind::DotDot, "..", self.line));
                        self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Dot, ".", self.line));
                    }
                }
                '&' => {
                    tokens.push(Token::new(TokenKind::Ampersand, "&", self.line));
                    self.advance();
                }
                '|' => {
                    tokens.push(Token::new(TokenKind::Pipe, "|", self.line));
                    self.advance();
                }
                '+' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::PlusEq, "+=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Plus, "+", self.line));
                    }
                }
                '-' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::MinusEq, "-=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, "-", self.line));
                    }
                }
                '*' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '*' {
                        tokens.push(Token::new(TokenKind::StarStar, "**", self.line)); self.advance();
                    } else if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::StarEq, "*=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Star, "*", self.line));
                    }
                }
                '/' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::SlashEq, "/=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Slash, "/", self.line));
                    }
                }
                '%' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::PercentEq, "%=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Percent, "%", self.line));
                    }
                }
                '=' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::EqEq, "==", self.line)); self.advance();
                    } else if !self.is_at_end() && self.current() == '>' {
                        tokens.push(Token::new(TokenKind::Arrow, "=>", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Eq, "=", self.line));
                    }
                }
                '!' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::BangEq, "!=", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Bang, "!", self.line));
                    }
                }
                '<' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::LtEq, "<=", self.line)); self.advance();
                    } else if !self.is_at_end() && self.current() == '<' {
                        tokens.push(Token::new(TokenKind::ShiftLeft, "<<", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, "<", self.line));
                    }
                }
                '>' => {
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        tokens.push(Token::new(TokenKind::GtEq, ">=", self.line)); self.advance();
                    } else if !self.is_at_end() && self.current() == '>' {
                        tokens.push(Token::new(TokenKind::ShiftRight, ">>", self.line)); self.advance();
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, ">", self.line));
                    }
                }
                'f' if self.peek_next() == Some('"') => {
                    let t = self.lex_fstring()?;
                    tokens.extend(t);
                }
                '"' => { let t = self.lex_string()?; tokens.push(t); }
                c if c.is_ascii_digit() => { let t = self.lex_number()?; tokens.push(t); }
                c if c.is_alphabetic() || c == '_' => { let t = self.lex_ident(); tokens.push(t); }
                other => {
                    return Err(DgmError::LexError {
                        msg: format!("unexpected character '{}'", other),
                        line: self.line,
                    });
                }
            }
        }
        tokens.push(Token::new(TokenKind::EOF, "", self.line));
        Ok(tokens)
    }

    fn lex_string(&mut self) -> Result<Token, DgmError> {
        let line = self.line;
        self.advance(); // skip opening "
        let mut s = String::new();
        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(DgmError::LexError { msg: "unterminated string escape".into(), line });
                }
                match self.current() {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    '0' => s.push('\0'),
                    other => s.push(other),
                }
            } else {
                s.push(self.current());
            }
            self.advance();
        }
        if self.is_at_end() {
            return Err(DgmError::LexError { msg: "unterminated string".into(), line });
        }
        self.advance(); // skip closing "
        Ok(Token::new(TokenKind::StringLit, s, line))
    }

    /// Lex f-string: f"hello {expr} world"
    /// Produces: FStringStart, StringLit, LBrace, <expr tokens>, RBrace, StringLit, ..., RParen
    fn lex_fstring(&mut self) -> Result<Vec<Token>, DgmError> {
        let line = self.line;
        self.advance(); // skip 'f'
        self.advance(); // skip '"'
        let mut tokens = vec![];
        tokens.push(Token::new(TokenKind::FStringStart, "f\"", line));

        let mut buf = String::new();
        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '{' {
                // push accumulated string part
                tokens.push(Token::new(TokenKind::StringLit, buf.clone(), line));
                buf.clear();
                tokens.push(Token::new(TokenKind::LBrace, "{", self.line));
                self.advance(); // skip '{'
                // lex tokens inside {} until matching '}'
                let mut depth = 1;
                let mut inner_src = String::new();
                while !self.is_at_end() && depth > 0 {
                    if self.current() == '{' { depth += 1; }
                    if self.current() == '}' {
                        depth -= 1;
                        if depth == 0 { break; }
                    }
                    inner_src.push(self.current());
                    self.advance();
                }
                // Lex inner expression
                let mut inner_lexer = Lexer::new(&inner_src);
                let inner_tokens = inner_lexer.tokenize()?;
                // push all except EOF
                for t in inner_tokens {
                    if t.kind != TokenKind::EOF {
                        tokens.push(Token { kind: t.kind, lexeme: t.lexeme, line: self.line });
                    }
                }
                if !self.is_at_end() && self.current() == '}' {
                    tokens.push(Token::new(TokenKind::RBrace, "}", self.line));
                    self.advance();
                }
            } else if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(DgmError::LexError { msg: "unterminated f-string escape".into(), line });
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
            tokens.push(Token::new(TokenKind::StringLit, buf, line));
        }
        if self.is_at_end() {
            return Err(DgmError::LexError { msg: "unterminated f-string".into(), line });
        }
        self.advance(); // skip closing "
        tokens.push(Token::new(TokenKind::RParen, ")", line)); // sentinel for parser
        Ok(tokens)
    }

    fn lex_number(&mut self) -> Result<Token, DgmError> {
        let line = self.line;
        let mut s = String::new();

        // Hex: 0x...
        if self.current() == '0' && self.peek_next().map(|c| c == 'x' || c == 'X').unwrap_or(false) {
            s.push(self.current()); self.advance(); // '0'
            s.push(self.current()); self.advance(); // 'x'
            while !self.is_at_end() && self.current().is_ascii_hexdigit() {
                s.push(self.current());
                self.advance();
            }
            let val = i64::from_str_radix(&s[2..], 16).map_err(|_| DgmError::LexError {
                msg: format!("invalid hex '{}'", s), line
            })?;
            return Ok(Token::new(TokenKind::IntLit, val.to_string(), line));
        }

        // Binary: 0b...
        if self.current() == '0' && self.peek_next().map(|c| c == 'b' || c == 'B').unwrap_or(false) {
            s.push(self.current()); self.advance();
            s.push(self.current()); self.advance();
            while !self.is_at_end() && (self.current() == '0' || self.current() == '1') {
                s.push(self.current());
                self.advance();
            }
            let val = i64::from_str_radix(&s[2..], 2).map_err(|_| DgmError::LexError {
                msg: format!("invalid binary '{}'", s), line
            })?;
            return Ok(Token::new(TokenKind::IntLit, val.to_string(), line));
        }

        // Octal: 0o...
        if self.current() == '0' && self.peek_next().map(|c| c == 'o' || c == 'O').unwrap_or(false) {
            s.push(self.current()); self.advance();
            s.push(self.current()); self.advance();
            while !self.is_at_end() && self.current().is_digit(8) {
                s.push(self.current());
                self.advance();
            }
            let val = i64::from_str_radix(&s[2..], 8).map_err(|_| DgmError::LexError {
                msg: format!("invalid octal '{}'", s), line
            })?;
            return Ok(Token::new(TokenKind::IntLit, val.to_string(), line));
        }

        // Decimal
        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
            if self.current() != '_' { s.push(self.current()); }
            self.advance();
        }
        if !self.is_at_end() && self.current() == '.' && self.peek_next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            s.push('.');
            self.advance();
            while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
                if self.current() != '_' { s.push(self.current()); }
                self.advance();
            }
            // Scientific notation
            if !self.is_at_end() && (self.current() == 'e' || self.current() == 'E') {
                s.push('e');
                self.advance();
                if !self.is_at_end() && (self.current() == '+' || self.current() == '-') {
                    s.push(self.current());
                    self.advance();
                }
                while !self.is_at_end() && self.current().is_ascii_digit() {
                    s.push(self.current());
                    self.advance();
                }
            }
            Ok(Token::new(TokenKind::FloatLit, s, line))
        } else {
            // Scientific notation on integer
            if !self.is_at_end() && (self.current() == 'e' || self.current() == 'E') {
                s.push('e');
                self.advance();
                if !self.is_at_end() && (self.current() == '+' || self.current() == '-') {
                    s.push(self.current());
                    self.advance();
                }
                while !self.is_at_end() && self.current().is_ascii_digit() {
                    s.push(self.current());
                    self.advance();
                }
                Ok(Token::new(TokenKind::FloatLit, s, line))
            } else {
                Ok(Token::new(TokenKind::IntLit, s, line))
            }
        }
    }

    fn lex_ident(&mut self) -> Token {
        let line = self.line;
        let mut s = String::new();
        while !self.is_at_end() && (self.current().is_alphanumeric() || self.current() == '_') {
            s.push(self.current());
            self.advance();
        }
        let kind = match s.as_str() {
            "imprt" => TokenKind::Imprt, "writ" => TokenKind::Writ,
            "def" => TokenKind::Def, "retrun" => TokenKind::Retrun,
            "iff" => TokenKind::Iff, "elseif" => TokenKind::Elseif,
            "els" => TokenKind::Els, "fr" => TokenKind::Fr,
            "whl" => TokenKind::Whl, "brk" => TokenKind::Brk,
            "cont" => TokenKind::Cont, "tru" => TokenKind::Tru,
            "fals" => TokenKind::Fals, "nul" => TokenKind::Nul,
            "let" => TokenKind::Let, "and" => TokenKind::And,
            "or" => TokenKind::Or, "not" => TokenKind::Not,
            "cls" => TokenKind::Cls, "new" => TokenKind::New,
            "ths" => TokenKind::Ths, "in" => TokenKind::In,
            // New keywords
            "try" => TokenKind::Try, "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally, "throw" => TokenKind::Throw,
            "match" => TokenKind::Match, "extends" => TokenKind::Extends,
            "lam" => TokenKind::Lam,
            _ => TokenKind::Ident,
        };
        Token::new(kind, s, line)
    }

    fn current(&self) -> char { self.source[self.pos] }
    fn advance(&mut self) { self.pos += 1; }
    fn is_at_end(&self) -> bool { self.pos >= self.source.len() }
    fn peek_next(&self) -> Option<char> { self.source.get(self.pos + 1).copied() }
}
