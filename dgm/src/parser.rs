use crate::ast::{Expr, ExprKind, Span, Stmt, StmtKind};
use crate::error::{DgmError, ErrorCode};
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
        while self.check(TokenKind::Newline) || self.check(TokenKind::Semicolon) {
            self.advance();
        }
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
            TokenKind::Brk => {
                let token = self.advance().clone();
                Stmt::new(token.span(), StmtKind::Break)
            }
            TokenKind::Cont => {
                let token = self.advance().clone();
                Stmt::new(token.span(), StmtKind::Continue)
            }
            TokenKind::Try => self.parse_try_catch()?,
            TokenKind::Throw => self.parse_throw()?,
            TokenKind::Match => self.parse_match()?,
            TokenKind::Imprt => self.parse_import()?,
            _ => {
                let expr = self.parse_expr()?;
                Stmt::new(expr.span.clone(), StmtKind::Expr(expr))
            }
        };

        if self.check(TokenKind::Newline) || self.check(TokenKind::Semicolon) {
            self.advance();
        }
        Ok(stmt)
    }

    fn parse_import(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let name = if self.check(TokenKind::StringLit) {
            let import_token = self.advance().clone();
            import_token.lexeme
        } else {
            self.expect_ident()?
        };
        let alias = if self.check(TokenKind::Ident) && self.peek().lexeme == "as" {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        Ok(Stmt::new(token.span(), StmtKind::Imprt { name, alias }))
    }

    fn parse_let(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::new(token.span(), StmtKind::Let { name, value }))
    }

    fn parse_writ(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        self.expect(TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        Ok(Stmt::new(token.span(), StmtKind::Writ(expr)))
    }

    fn parse_if(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let then_block = self.parse_block()?;
        let mut elseif_branches = vec![];
        let mut else_block = None;
        self.skip_newlines();
        while self.check(TokenKind::Elseif) {
            self.advance();
            let condition = self.parse_expr()?;
            self.skip_newlines();
            let block = self.parse_block()?;
            elseif_branches.push((condition, block));
            self.skip_newlines();
        }
        if self.check(TokenKind::Els) {
            self.advance();
            self.skip_newlines();
            else_block = Some(self.parse_block()?);
        }
        Ok(Stmt::new(
            token.span(),
            StmtKind::If {
                condition,
                then_block,
                elseif_branches,
                else_block,
            },
        ))
    }

    fn parse_while(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::new(token.span(), StmtKind::While { condition, body }))
    }

    fn parse_for(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let var = self.expect_ident()?;
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::new(token.span(), StmtKind::For { var, iterable, body }))
    }

    fn parse_func_def(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(Stmt::new(token.span(), StmtKind::FuncDef { name, params, body }))
    }

    fn parse_class_def(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
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
        Ok(Stmt::new(token.span(), StmtKind::ClassDef { name, parent, methods }))
    }

    fn parse_return(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let expr = if self.check(TokenKind::Newline)
            || self.check(TokenKind::RBrace)
            || self.check(TokenKind::Semicolon)
            || self.is_at_end()
        {
            None
        } else {
            Some(self.parse_expr()?)
        };
        Ok(Stmt::new(token.span(), StmtKind::Return(expr)))
    }

    fn parse_try_catch(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
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
        Ok(Stmt::new(
            token.span(),
            StmtKind::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            },
        ))
    }

    fn parse_throw(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let expr = self.parse_expr()?;
        Ok(Stmt::new(token.span(), StmtKind::Throw(expr)))
    }

    fn parse_match(&mut self) -> Result<Stmt, DgmError> {
        let token = self.advance().clone();
        let expr = self.parse_expr()?;
        self.skip_newlines();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut arms = vec![];
        let mut default = None;
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            if self.peek().kind == TokenKind::Ident && self.peek().lexeme == "_" {
                self.advance();
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                default = Some(self.parse_match_arm_block()?);
            } else {
                let pattern = self.parse_expr()?;
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                let block = self.parse_match_arm_block()?;
                arms.push((pattern, block));
            }
            self.skip_newlines();
            if self.check(TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Stmt::new(token.span(), StmtKind::Match { expr, arms, default }))
    }

    fn parse_match_arm_block(&mut self) -> Result<Vec<Stmt>, DgmError> {
        if self.check(TokenKind::LBrace) {
            self.parse_block()
        } else {
            let expr = self.parse_expr()?;
            Ok(vec![Stmt::new(expr.span.clone(), StmtKind::Expr(expr))])
        }
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

    #[allow(dead_code)]
    fn parse_param_list(&mut self) -> Result<Vec<String>, DgmError> {
        let mut params = vec![];
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        params.push(self.expect_ident()?);
        while self.check(TokenKind::Comma) {
            self.advance();
            params.push(self.expect_ident()?);
        }
        Ok(params)
    }

    fn parse_expr(&mut self) -> Result<Expr, DgmError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, DgmError> {
        let left = self.parse_ternary()?;
        let op = match self.peek().kind {
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
        let span = left.span.clone();
        Ok(Expr::new(
            span,
            ExprKind::Assign {
                target: Box::new(left),
                op: op.into(),
                value: Box::new(value),
            },
        ))
    }

    fn parse_ternary(&mut self) -> Result<Expr, DgmError> {
        let expr = self.parse_or()?;
        if self.check(TokenKind::Question) {
            self.advance();
            let then_expr = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            let span = expr.span.clone();
            Ok(Expr::new(
                span,
                ExprKind::Ternary {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
            ))
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_and()?;
        while self.check(TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: "or".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_or()?;
        while self.check(TokenKind::And) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: "and".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_xor()?;
        while self.check(TokenKind::Pipe) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: "|".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_bitwise_and()?;
        while self.check(TokenKind::Caret) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: "^".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, DgmError> {
        let mut left = self.parse_equality()?;
        while self.check(TokenKind::Ampersand) {
            self.advance();
            let right = self.parse_equality()?;
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: "&".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
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
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: op.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
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
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: op.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_in_expr(&mut self) -> Result<Expr, DgmError> {
        let left = self.parse_shift()?;
        if self.check(TokenKind::In) {
            self.advance();
            let right = self.parse_shift()?;
            let span = left.span.clone();
            Ok(Expr::new(
                span,
                ExprKind::BinOp {
                    op: "in".into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            ))
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
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: op.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
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
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: op.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
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
            let span = left.span.clone();
            left = Expr::new(
                span,
                ExprKind::BinOp {
                    op: op.into(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, DgmError> {
        let base = self.parse_unary()?;
        if self.check(TokenKind::StarStar) {
            self.advance();
            let exponent = self.parse_power()?;
            let span = base.span.clone();
            return Ok(Expr::new(
                span,
                ExprKind::BinOp {
                    op: "**".into(),
                    left: Box::new(base),
                    right: Box::new(exponent),
                },
            ));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, DgmError> {
        if self.check(TokenKind::Not) || self.check(TokenKind::Bang) {
            let token = self.advance().clone();
            return Ok(Expr::new(
                token.span(),
                ExprKind::UnaryOp {
                    op: "not".into(),
                    operand: Box::new(self.parse_unary()?),
                },
            ));
        }
        if self.check(TokenKind::Minus) {
            let token = self.advance().clone();
            return Ok(Expr::new(
                token.span(),
                ExprKind::UnaryOp {
                    op: "-".into(),
                    operand: Box::new(self.parse_unary()?),
                },
            ));
        }
        if self.check(TokenKind::Tilde) {
            let token = self.advance().clone();
            return Ok(Expr::new(
                token.span(),
                ExprKind::UnaryOp {
                    op: "~".into(),
                    operand: Box::new(self.parse_unary()?),
                },
            ));
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
                let span = expr.span.clone();
                expr = Expr::new(
                    span,
                    ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                );
            } else if self.check(TokenKind::Dot) {
                self.advance();
                let field = self.expect_ident()?;
                let span = expr.span.clone();
                expr = Expr::new(
                    span,
                    ExprKind::FieldAccess {
                        object: Box::new(expr),
                        field,
                    },
                );
            } else if self.check(TokenKind::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                let span = expr.span.clone();
                expr = Expr::new(
                    span,
                    ExprKind::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    },
                );
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, DgmError> {
        let mut args = vec![];
        if self.check(TokenKind::RParen) {
            return Ok(args);
        }
        args.push(self.parse_expr()?);
        while self.check(TokenKind::Comma) {
            self.advance();
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, DgmError> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::IntLit => {
                self.advance();
                let span = token.span();
                let value = token.lexeme.parse::<i64>().map_err(|_| {
                    DgmError::new(ErrorCode::ParseError, format!("invalid int '{}'", token.lexeme))
                        .with_span(span.clone())
                })?;
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::new(
                        span.clone(),
                        ExprKind::Range {
                            start: Box::new(Expr::new(span, ExprKind::IntLit(value))),
                            end: Box::new(end),
                        },
                    ));
                }
                Ok(Expr::new(span, ExprKind::IntLit(value)))
            }
            TokenKind::FloatLit => {
                self.advance();
                let span = token.span();
                let value = token.lexeme.parse::<f64>().map_err(|_| {
                    DgmError::new(ErrorCode::ParseError, format!("invalid float '{}'", token.lexeme))
                        .with_span(span.clone())
                })?;
                Ok(Expr::new(span, ExprKind::FloatLit(value)))
            }
            TokenKind::StringLit => {
                self.advance();
                Ok(Expr::new(token.span(), ExprKind::StringLit(token.lexeme)))
            }
            TokenKind::Tru => {
                self.advance();
                Ok(Expr::new(token.span(), ExprKind::BoolLit(true)))
            }
            TokenKind::Fals => {
                self.advance();
                Ok(Expr::new(token.span(), ExprKind::BoolLit(false)))
            }
            TokenKind::Nul => {
                self.advance();
                Ok(Expr::new(token.span(), ExprKind::NullLit))
            }
            TokenKind::Ths => {
                self.advance();
                Ok(Expr::new(token.span(), ExprKind::This))
            }
            TokenKind::Ident => {
                self.advance();
                let span = token.span();
                let expr = Expr::new(span.clone(), ExprKind::Ident(token.lexeme));
                if self.check(TokenKind::DotDot) {
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::new(
                        span,
                        ExprKind::Range {
                            start: Box::new(expr),
                            end: Box::new(end),
                        },
                    ));
                }
                Ok(expr)
            }
            TokenKind::New => {
                self.advance();
                let class_name = self.expect_ident()?;
                self.expect(TokenKind::LParen)?;
                let args = self.parse_args()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr::new(token.span(), ExprKind::New { class_name, args }))
            }
            TokenKind::Lam => {
                self.advance();
                let span = token.span();
                self.expect(TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(TokenKind::RParen)?;
                self.expect(TokenKind::Arrow)?;
                self.skip_newlines();
                let body = if self.check(TokenKind::LBrace) {
                    self.parse_block()?
                } else {
                    let expr = self.parse_expr()?;
                    vec![Stmt::new(expr.span.clone(), StmtKind::Return(Some(expr)))]
                };
                Ok(Expr::new(span, ExprKind::Lambda { params, body }))
            }
            TokenKind::FStringStart => {
                self.advance();
                self.parse_fstring_body(token.span())
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                if self.check(TokenKind::DotDot) {
                    let span = expr.span.clone();
                    self.advance();
                    let end = self.parse_unary()?;
                    return Ok(Expr::new(
                        span,
                        ExprKind::Range {
                            start: Box::new(expr),
                            end: Box::new(end),
                        },
                    ));
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
                        if self.check(TokenKind::RBracket) {
                            break;
                        }
                        items.push(self.parse_expr()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::new(token.span(), ExprKind::List(items)))
            }
            TokenKind::LBrace => {
                self.advance();
                self.skip_newlines();
                let mut pairs = vec![];
                if !self.check(TokenKind::RBrace) {
                    let key = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    pairs.push((key, value));
                    while self.check(TokenKind::Comma) {
                        self.advance();
                        self.skip_newlines();
                        if self.check(TokenKind::RBrace) {
                            break;
                        }
                        let key = self.parse_expr()?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.parse_expr()?;
                        pairs.push((key, value));
                    }
                }
                self.skip_newlines();
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::new(token.span(), ExprKind::Map(pairs)))
            }
            _ => Err(DgmError::new(
                ErrorCode::UnexpectedToken,
                format!("unexpected token '{}'", token.lexeme),
            )
            .with_span(token.span())),
        }
    }

    fn parse_fstring_body(&mut self, span: Span) -> Result<Expr, DgmError> {
        let mut parts = vec![];
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            if self.check(TokenKind::StringLit) {
                let token = self.advance().clone();
                if !token.lexeme.is_empty() {
                    parts.push(Expr::new(token.span(), ExprKind::StringLit(token.lexeme)));
                }
            } else if self.check(TokenKind::LBrace) {
                self.advance();
                parts.push(self.parse_expr()?);
                self.expect(TokenKind::RBrace)?;
            } else {
                break;
            }
        }
        if self.check(TokenKind::RParen) {
            self.advance();
        }
        Ok(Expr::new(span, ExprKind::StringInterp(parts)))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
        token
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn check_skip_newlines(&mut self, kind: TokenKind) -> bool {
        let saved = self.pos;
        while self.check(TokenKind::Newline) {
            self.advance();
        }
        let found = self.peek().kind == kind;
        if !found {
            self.pos = saved;
        }
        found
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, DgmError> {
        if self.peek().kind == kind {
            Ok(self.advance().clone())
        } else {
            let token = self.peek().clone();
            Err(DgmError::new(
                ErrorCode::ExpectedToken,
                format!("expected {:?}, got '{}'", kind, token.lexeme),
            )
            .with_span(token.span()))
        }
    }

    fn expect_ident(&mut self) -> Result<String, DgmError> {
        let token = self.peek().clone();
        if token.kind == TokenKind::Ident {
            self.advance();
            Ok(token.lexeme)
        } else {
            Err(DgmError::new(
                ErrorCode::ExpectedToken,
                format!("expected identifier, got '{}'", token.lexeme),
            )
            .with_span(token.span()))
        }
    }
}
