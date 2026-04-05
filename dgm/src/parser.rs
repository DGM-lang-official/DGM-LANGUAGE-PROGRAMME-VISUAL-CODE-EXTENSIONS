use crate::ast::{Expr, Stmt};
use crate::error::DgmError;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, DgmError> {
        let mut stmts = vec![];
        self.skip_newlines();
        while !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn skip_newlines(&mut self) {
        while self.check(TokenKind::Newline) || self.check(TokenKind::Semicolon) { self.advance(); }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, DgmError> {
        let stmt = match self.peek().kind.clone() {
            TokenKind::Let => self.parse_let()?,
            TokenKind::Writ => self.parse_writ()?,
            TokenKind::Iff => self.parse_if()?,
            TokenKind::Whl => self.parse_while()?,
            TokenKind::Fr => self.parse_for()?,
            TokenKind::Def => self.parse_func_def()?,
            TokenKind::Cls => self.parse_class_def()?,
            TokenKind::Retrun => self.parse_return()?,
            TokenKind::Brk => { self.advance(); Stmt::Break }
            TokenKind::Cont => { self.advance(); Stmt::Continue }
            TokenKind::Try => self.parse_try_catch()?,
            TokenKind::Throw => self.parse_throw()?,
            TokenKind::Match => self.parse_match()?,
            TokenKind::Imprt => {
                self.advance();
                if self.check(TokenKind::StringLit) {
                    let tok = self.peek().clone();
                    self.advance();
                    Stmt::Imprt(tok.lexeme)
                } else {
                    let name = self.expect_ident()?;
                    Stmt::Imprt(name)
                }
            }
            _ => Stmt::Expr(self.parse_expr()?),
        };
        // consume optional trailing newline/semicolon
        if self.check(TokenKind::Newline) || self.check(TokenKind::Semicolon) { self.advance(); }
        Ok(stmt)
    }

    fn parse_let(&mut self) -> Result<Stmt, DgmError> {
        self.advance(); // consume 'let'
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let { name, value })
    }

    fn parse_writ(&mut self) -> Result<Stmt, DgmError> {
        self.advance(); // consume 'writ'
        self.expect(TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Stmt::Writ(expr))
    }

    fn parse_if(&mut self) -> Result<Stmt, DgmError> {
        self.advance(); // consume 'iff'
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let then_block = self.parse_block()?;
        let mut elseif_branches = vec![];
        let mut else_block = None;
        self.skip_newlines();
        while self.check(TokenKind::Elseif) {
            self.advance();
            let cond = self.parse_expr()?;
            self.skip_newlines();
            let block = self.parse_block()?;
            elseif_branches.push((cond, block));
            self.skip_newlines();
        }
        if self.check(TokenKind::Els) {
            self.advance();
            self.skip_newlines();
            else_block = Some(self.parse_block()?);
        }
        Ok(Stmt::If { condition, then_block, elseif_branches, else_block })
    }

    fn parse_while(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        let var = self.expect_ident()?;
        // expect 'in' keyword
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::For { var, iterable, body })
    }

    fn parse_func_def(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::FuncDef { name, params, body })
    }

    fn parse_class_def(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        let name = self.expect_ident()?;
        let parent = if self.check(TokenKind::Extends) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.skip_newlines();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut methods = vec![];
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            methods.push(self.parse_func_def()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::ClassDef { name, parent, methods })
    }

    fn parse_return(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        if self.check(TokenKind::Newline) || self.check(TokenKind::RBrace) ||
           self.check(TokenKind::Semicolon) || self.is_at_end() {
            Ok(Stmt::Return(None))
        } else {
            Ok(Stmt::Return(Some(self.parse_expr()?)))
        }
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, DgmError> {
        self.advance(); // consume 'try'
        self.skip_newlines();
        let try_block = self.parse_block()?;
        self.skip_newlines();
        self.expect(TokenKind::Catch)?;
        let catch_var = if self.check(TokenKind::LParen) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(TokenKind::RParen)?;
            Some(name)
        } else {
            None
        };
        self.skip_newlines();
        let catch_block = self.parse_block()?;
        let finally_block = if self.check_skip_newlines(TokenKind::Finally) {
            self.advance();
            self.skip_newlines();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::TryCatch { try_block, catch_var, catch_block, finally_block })
    }

    fn parse_throw(&mut self) -> Result<Stmt, DgmError> {
        self.advance();
        let expr = self.parse_expr()?;
        Ok(Stmt::Throw(expr))
    }

    fn parse_match(&mut self) -> Result<Stmt, DgmError> {
        self.advance(); // consume 'match'
        let expr = self.parse_expr()?;
        self.skip_newlines();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut arms = vec![];
        let mut default = None;
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // check for '_' default arm
            if self.peek().kind == TokenKind::Ident && self.peek().lexeme == "_" {
                self.advance();
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                let block = if self.check(TokenKind::LBrace) {
                    self.parse_block()?
                } else {
                    let e = self.parse_expr()?;
                    vec![Stmt::Expr(e)]
                };
                default = Some(block);
            } else {
                let pattern = self.parse_expr()?;
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                let block = if self.check(TokenKind::LBrace) {
                    self.parse_block()?
                } else {
                    let e = self.parse_expr()?;
                    vec![Stmt::Expr(e)]
                };
                arms.push((pattern, block));
            }
            self.skip_newlines();
            // optional comma
            if self.check(TokenKind::Comma) { self.advance(); }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::Match { expr, arms, default })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, DgmError> {
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();
        let mut stmts = vec![];
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>, DgmError> {
        let mut params = vec![];
        if self.check(TokenKind::RParen) { return Ok(params); }
        params.push(self.expect_ident()?);
        while self.check(TokenKind::Comma) {
            self.advance();
            params.push(self.expect_ident()?);
        }
        Ok(params)
    }

    // Expression parsing (precedence climbing)
    fn parse_expr(&mut self) -> Result<Expr, DgmError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, DgmError> {
        let left = self.parse_ternary()?;
        let op_kind = self.peek().kind.clone();
        let op_str = match op_kind {
            TokenKind::Eq => "=",
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",
            TokenKind::PercentEq => "%=",
            _ => return Ok(left),
        };
        self.advance();
        let value = self.parse_assignment()?;
        Ok(Expr::Assign { target: Box::new(left), op: op_str.into(), value: Box::new(value) })
    }

    fn parse_ternary(&mut self) -> Result<Expr, DgmError> {
        let expr = self.parse_or()?;
        if self.check(TokenKind::Question) {
            self.advance();
            let then_expr = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            Ok(Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_and()?;
        while self.check(TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp { op: "or".into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_or()?;
        while self.check(TokenKind::And) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::BinOp { op: "and".into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_xor()?;
        while self.check(TokenKind::Pipe) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::BinOp { op: "|".into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_and()?;
        while self.check(TokenKind::Caret) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::BinOp { op: "^".into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_equality()?;
        while self.check(TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp { op: "&".into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::EqEq => "==",
                TokenKind::BangEq => "!=",
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp { op: op.into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_in_expr()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Lt => "<",
                TokenKind::Gt => ">",
                TokenKind::LtEq => "<=",
                TokenKind::GtEq => ">=",
                _ => break,
            };
            self.advance();
            let right = self.parse_in_expr()?;
            left = Expr::BinOp { op: op.into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_in_expr(&mut self) -> Result<Expr, DgmError> {
        let left = self.parse_shift()?;
        if self.check(TokenKind::In) {
            self.advance();
            let right = self.parse_shift()?;
            Ok(Expr::BinOp { op: "in".into(), left: Box::new(left), right: Box::new(right) })
        } else {
            Ok(left)
        }
    }

    fn parse_shift(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::ShiftLeft => "<<",
                TokenKind::ShiftRight => ">>",
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            left = Expr::BinOp { op: op.into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::BinOp { op: op.into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinOp { op: op.into(), left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, DgmError> {
        let base = self.parse_unary()?;
        if self.check(TokenKind::StarStar) {
            self.advance();
            let exp = self.parse_power()?;
            return Ok(Expr::BinOp { op: "**".into(), left: Box::new(base), right: Box::new(exp) });
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, DgmError> {
        if self.check(TokenKind::Not) || self.check(TokenKind::Bang) {
            self.advance();
            return Ok(Expr::UnaryOp { op: "not".into(), operand: Box::new(self.parse_unary()?) });
        }
        if self.check(TokenKind::Minus) {
            self.advance();
            return Ok(Expr::UnaryOp { op: "-".into(), operand: Box::new(self.parse_unary()?) });
        }
        if self.check(TokenKind::Tilde) {
            self.advance();
            return Ok(Expr::UnaryOp { op: "~".into(), operand: Box::new(self.parse_unary()?) });
        }
        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expr, DgmError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(TokenKind::LParen) {
                self.advance();
                let args = self.parse_args()?;
                self.expect(TokenKind::RParen)?;
                expr = Expr::Call { callee: Box::new(expr), args };
            } else if self.check(TokenKind::Dot) {
                self.advance();
                let field = self.expect_ident()?;
                expr = Expr::FieldAccess { object: Box::new(expr), field };
            } else if self.check(TokenKind::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                expr = Expr::Index { object: Box::new(expr), index: Box::new(index) };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, DgmError> {
        let mut args = vec![];
        if self.check(TokenKind::RParen) { return Ok(args); }
        args.push(self.parse_expr()?);
        while self.check(TokenKind::Comma) {
            self.advance();
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, DgmError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::IntLit => {
                self.advance();
                let v = tok.lexeme.parse::<i64>().map_err(|_| DgmError::ParseError {
                    msg: format!("invalid int '{}'", tok.lexeme), line: tok.line
                })?;
                // Check for range operator
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::Range { start: Box::new(Expr::IntLit(v)), end: Box::new(end) });
                }
                Ok(Expr::IntLit(v))
            }
            TokenKind::FloatLit => {
                self.advance();
                let v = tok.lexeme.parse::<f64>().map_err(|_| DgmError::ParseError {
                    msg: format!("invalid float '{}'", tok.lexeme), line: tok.line
                })?;
                Ok(Expr::FloatLit(v))
            }
            TokenKind::StringLit => {
                self.advance();
                Ok(Expr::StringLit(tok.lexeme))
            }
            TokenKind::Tru => { self.advance(); Ok(Expr::BoolLit(true)) }
            TokenKind::Fals => { self.advance(); Ok(Expr::BoolLit(false)) }
            TokenKind::Nul => { self.advance(); Ok(Expr::NullLit) }
            TokenKind::Ths => { self.advance(); Ok(Expr::This) }
            TokenKind::Ident => {
                self.advance();
                let expr = Expr::Ident(tok.lexeme);
                // Check for range operator
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::Range { start: Box::new(expr), end: Box::new(end) });
                }
                Ok(expr)
            }
            TokenKind::New => {
                self.advance();
                let class_name = self.expect_ident()?;
                self.expect(TokenKind::LParen)?;
                let args = self.parse_args()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::New { class_name, args })
            }
            TokenKind::Lam => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(TokenKind::RParen)?;
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                if self.check(TokenKind::LBrace) {
                    let body = self.parse_block()?;
                    Ok(Expr::Lambda { params, body })
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Expr::Lambda { params, body: vec![Stmt::Return(Some(expr))] })
                }
            }
            TokenKind::FStringStart => {
                self.advance(); // consume FStringStart
                self.parse_fstring_body()
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                // Check for range operator
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::Range { start: Box::new(expr), end: Box::new(end) });
                }
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = vec![];
                if !self.check(TokenKind::RBracket) {
                    items.push(self.parse_expr()?);
                    while self.check(TokenKind::Comma) {
                        self.advance();
                        if self.check(TokenKind::RBracket) { break; }
                        items.push(self.parse_expr()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::List(items))
            }
            TokenKind::LBrace => {
                self.advance();
                self.skip_newlines();
                let mut pairs = vec![];
                if !self.check(TokenKind::RBrace) {
                    let k = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let v = self.parse_expr()?;
                    pairs.push((k, v));
                    while self.check(TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                        if self.check(TokenKind::RBrace) { break; }
                        let k = self.parse_expr()?;
                        self.expect(TokenKind::Colon)?;
                        let v = self.parse_expr()?;
                        pairs.push((k, v));
                    }
                }
                self.skip_newlines();
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::Map(pairs))
            }
            _ => Err(DgmError::ParseError {
                msg: format!("unexpected token '{}'", tok.lexeme),
                line: tok.line,
            })
        }
    }

    fn parse_fstring_body(&mut self) -> Result<Expr, DgmError> {
        let mut parts = vec![];
        // f-string tokens: StringLit, LBrace, <expr>, RBrace, ..., RParen (sentinel)
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            if self.check(TokenKind::StringLit) {
                let tok = self.peek().clone();
                self.advance();
                if !tok.lexeme.is_empty() {
                    parts.push(Expr::StringLit(tok.lexeme));
                }
            } else if self.check(TokenKind::LBrace) {
                self.advance();
                let expr = self.parse_expr()?;
                parts.push(expr);
                self.expect(TokenKind::RBrace)?;
            } else {
                break;
            }
        }
        if self.check(TokenKind::RParen) { self.advance(); } // consume sentinel
        Ok(Expr::StringInterp(parts))
    }

    // Helpers
    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn advance(&mut self) -> &Token { let t = &self.tokens[self.pos]; self.pos += 1; t }
    fn is_at_end(&self) -> bool { self.peek().kind == TokenKind::EOF }
    fn check(&self, kind: TokenKind) -> bool { self.peek().kind == kind }

    fn check_skip_newlines(&mut self, kind: TokenKind) -> bool {
        let saved = self.pos;
        while self.check(TokenKind::Newline) { self.advance(); }
        if self.peek().kind == kind {
            true
        } else {
            self.pos = saved;
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<&Token, DgmError> {
        if self.peek().kind == kind {
            Ok(self.advance())
        } else {
            Err(DgmError::ParseError {
                msg: format!("expected {:?}, got '{}'", kind, self.peek().lexeme),
                line: self.peek().line,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String, DgmError> {
        let tok = self.peek().clone();
        if tok.kind == TokenKind::Ident {
            self.advance();
            Ok(tok.lexeme)
        } else {
            Err(DgmError::ParseError {
                msg: format!("expected identifier, got '{}'", tok.lexeme),
                line: tok.line,
            })
        }
    }
}
