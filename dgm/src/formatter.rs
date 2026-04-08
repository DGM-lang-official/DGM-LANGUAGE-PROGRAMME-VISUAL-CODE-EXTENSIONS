use crate::ast::{Expr, ExprKind, Stmt, StmtKind};
use crate::error::DgmError;

pub fn format_source(source: &str) -> Result<String, DgmError> {
    format_named_source(source, "<source>")
}

pub fn format_named_source(
    source: &str,
    source_name: impl Into<String>,
) -> Result<String, DgmError> {
    let stmts = crate::parse_named_source(source, source_name)?;
    let mut formatter = Formatter::default();
    formatter.format_stmts(&stmts);
    Ok(formatter.finish())
}

#[derive(Default)]
struct Formatter {
    out: String,
    indent: usize,
}

impl Formatter {
    fn finish(mut self) -> String {
        while self.out.ends_with("\n\n") {
            self.out.pop();
        }
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    fn format_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.format_stmt(stmt);
        }
    }

    fn format_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.push_line(self.format_expr(expr)),
            StmtKind::Let { name, value } => {
                self.push_line(format!("let {} = {}", name, self.format_expr(value)));
            }
            StmtKind::Const { name, value } => {
                self.push_line(format!("const {} = {}", name, self.format_expr(value)));
            }
            StmtKind::LetDestructure { names, rest, value } => {
                let mut parts = names.clone();
                if let Some(rest_name) = rest {
                    parts.push(format!("...{}", rest_name));
                }
                self.push_line(format!(
                    "let [{}] = {}",
                    parts.join(", "),
                    self.format_expr(value)
                ));
            }
            StmtKind::Writ(expr) => self.push_line(format!("writ({})", self.format_expr(expr))),
            StmtKind::If {
                condition,
                then_block,
                elseif_branches,
                else_block,
            } => {
                self.write_indent();
                self.out.push_str("if ");
                self.out.push_str(&self.format_expr(condition));
                self.out.push(' ');
                self.format_block(then_block);
                self.out.push('\n');
                for (cond, block) in elseif_branches {
                    self.write_indent();
                    self.out.push_str("else if ");
                    self.out.push_str(&self.format_expr(cond));
                    self.out.push(' ');
                    self.format_block(block);
                    self.out.push('\n');
                }
                if let Some(block) = else_block {
                    self.write_indent();
                    self.out.push_str("else ");
                    self.format_block(block);
                    self.out.push('\n');
                }
            }
            StmtKind::While { condition, body } => {
                self.write_indent();
                self.out.push_str("while ");
                self.out.push_str(&self.format_expr(condition));
                self.out.push(' ');
                self.format_block(body);
                self.out.push('\n');
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                self.write_indent();
                self.out.push_str("for ");
                self.out.push_str(var);
                self.out.push_str(" in ");
                self.out.push_str(&self.format_expr(iterable));
                self.out.push(' ');
                self.format_block(body);
                self.out.push('\n');
            }
            StmtKind::FuncDef {
                name,
                params,
                defaults,
                rest_param,
                body,
            } => {
                self.write_indent();
                self.out.push_str("fn ");
                self.out.push_str(name);
                self.out.push('(');
                self.out
                    .push_str(&format_params(params, defaults, rest_param.as_deref(), self));
                self.out.push_str(") ");
                self.format_block(body);
                self.out.push('\n');
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.push_line(format!("return {}", self.format_expr(expr)));
                } else {
                    self.push_line("return".to_string());
                }
            }
            StmtKind::Break => self.push_line("break".to_string()),
            StmtKind::Continue => self.push_line("continue".to_string()),
            StmtKind::ClassDef {
                name,
                parent,
                methods,
            } => {
                self.write_indent();
                self.out.push_str("class ");
                self.out.push_str(name);
                if let Some(parent) = parent {
                    self.out.push_str(" extends ");
                    self.out.push_str(parent);
                }
                self.out.push_str(" {\n");
                self.indent += 1;
                for method in methods {
                    self.format_stmt(method);
                }
                self.indent -= 1;
                self.write_indent();
                self.out.push_str("}\n");
            }
            StmtKind::Imprt { name, alias } => {
                if let Some(alias) = alias {
                    self.push_line(format!("import {} as {}", format_import_name(name), alias));
                } else {
                    self.push_line(format!("import {}", format_import_name(name)));
                }
            }
            StmtKind::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                self.write_indent();
                self.out.push_str("try ");
                self.format_block(try_block);
                self.out.push('\n');
                self.write_indent();
                self.out.push_str("catch");
                if let Some(name) = catch_var {
                    self.out.push('(');
                    self.out.push_str(name);
                    self.out.push(')');
                }
                self.out.push(' ');
                self.format_block(catch_block);
                self.out.push('\n');
                if let Some(block) = finally_block {
                    self.write_indent();
                    self.out.push_str("finally ");
                    self.format_block(block);
                    self.out.push('\n');
                }
            }
            StmtKind::Throw(expr) => self.push_line(format!("throw {}", self.format_expr(expr))),
            StmtKind::Match { expr, arms, default } => {
                self.write_indent();
                self.out.push_str("match ");
                self.out.push_str(&self.format_expr(expr));
                self.out.push_str(" {\n");
                self.indent += 1;
                for (pattern, guard, block) in arms {
                    self.write_indent();
                    self.out.push_str(&self.format_expr(pattern));
                    if let Some(guard) = guard {
                        self.out.push_str(" if ");
                        self.out.push_str(&self.format_expr(guard));
                    }
                    self.out.push_str(" => ");
                    self.format_match_arm(block);
                    self.out.push('\n');
                }
                if let Some(block) = default {
                    self.write_indent();
                    self.out.push_str("_ => ");
                    self.format_match_arm(block);
                    self.out.push('\n');
                }
                self.indent -= 1;
                self.write_indent();
                self.out.push_str("}\n");
            }
        }
    }

    fn format_match_arm(&mut self, block: &[Stmt]) {
        if let [Stmt {
            kind: StmtKind::Expr(expr),
            ..
        }] = block
        {
            self.out.push_str(&self.format_expr(expr));
            return;
        }
        self.format_block(block);
    }

    fn format_block(&mut self, stmts: &[Stmt]) {
        if stmts.is_empty() {
            self.out.push_str("{}");
            return;
        }
        self.out.push_str("{\n");
        self.indent += 1;
        for stmt in stmts {
            self.format_stmt(stmt);
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_expr(&self, expr: &Expr) -> String {
        self.format_expr_prec(expr, 0)
    }

    fn format_expr_prec(&self, expr: &Expr, parent_prec: u8) -> String {
        let prec = expr_precedence(expr);
        let mut rendered = match &expr.kind {
            ExprKind::IntLit(value) => value.to_string(),
            ExprKind::FloatLit(value) => {
                let mut text = value.to_string();
                if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                    text.push_str(".0");
                }
                text
            }
            ExprKind::StringLit(value) => serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into()),
            ExprKind::BoolLit(value) => {
                if *value { "true".to_string() } else { "false".to_string() }
            }
            ExprKind::NullLit => "null".to_string(),
            ExprKind::Ident(name) => name.clone(),
            ExprKind::This => "this".to_string(),
            ExprKind::Super => "super".to_string(),
            ExprKind::BinOp { op, left, right } => format!(
                "{} {} {}",
                self.format_expr_prec(left, prec + 1),
                op,
                self.format_expr_prec(right, prec + 1)
            ),
            ExprKind::UnaryOp { op, operand } => {
                let op_text = if op == "not" { "!" } else { op.as_str() };
                format!("{}{}", op_text, self.format_expr_prec(operand, prec))
            }
            ExprKind::Call { callee, args } => format!(
                "{}({})",
                self.format_expr_prec(callee, prec),
                args.iter()
                    .map(|arg| self.format_expr(arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ExprKind::Index { object, index } => format!(
                "{}[{}]",
                self.format_expr_prec(object, prec),
                self.format_expr(index)
            ),
            ExprKind::FieldAccess { object, field } => {
                format!("{}.{}", self.format_expr_prec(object, prec), field)
            }
            ExprKind::List(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| self.format_expr(item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ExprKind::Map(entries) => {
                if entries.is_empty() {
                    "{}".to_string()
                } else {
                    format!(
                        "{{{}}}",
                        entries
                            .iter()
                            .map(|(key, value)| format!(
                                "{}: {}",
                                self.format_expr(key),
                                self.format_expr(value)
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            ExprKind::Assign { target, op, value } => format!(
                "{} {} {}",
                self.format_expr_prec(target, prec + 1),
                op,
                self.format_expr_prec(value, prec)
            ),
            ExprKind::New { class_name, args } => format!(
                "new {}({})",
                class_name,
                args.iter()
                    .map(|arg| self.format_expr(arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ExprKind::Lambda {
                params,
                defaults,
                rest_param,
                body,
            } => {
                let params = format_params(params, defaults, rest_param.as_deref(), self);
                if let [Stmt {
                    kind: StmtKind::Return(Some(expr)),
                    ..
                }] = body.as_slice()
                {
                    format!("lam({}) => {}", params, self.format_expr(expr))
                } else {
                    let mut nested = Formatter {
                        out: String::new(),
                        indent: self.indent,
                    };
                    nested.out.push_str(&format!("lam({}) ", params));
                    nested.format_block(body);
                    nested.out
                }
            }
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => format!(
                "{} ? {} : {}",
                self.format_expr_prec(condition, prec + 1),
                self.format_expr_prec(then_expr, prec + 1),
                self.format_expr_prec(else_expr, prec + 1)
            ),
            ExprKind::StringInterp(parts) => format!(
                "f\"{}\"",
                parts
                    .iter()
                    .map(format_fstring_part)
                    .collect::<Vec<_>>()
                    .join("")
            ),
            ExprKind::Range { start, end } => format!(
                "{}..{}",
                self.format_expr_prec(start, prec + 1),
                self.format_expr_prec(end, prec + 1)
            ),
        };

        if prec < parent_prec {
            rendered = format!("({rendered})");
        }
        rendered
    }

    fn write_indent(&mut self) {
        self.out.push_str(&"    ".repeat(self.indent));
    }

    fn push_line(&mut self, line: String) {
        self.write_indent();
        self.out.push_str(&line);
        self.out.push('\n');
    }
}

fn expr_precedence(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Assign { .. } => 1,
        ExprKind::Ternary { .. } => 2,
        ExprKind::BinOp { op, .. } => match op.as_str() {
            "or" => 3,
            "and" => 4,
            "|" => 5,
            "^" => 6,
            "&" => 7,
            "==" | "!=" => 8,
            "<" | ">" | "<=" | ">=" | "in" => 9,
            "<<" | ">>" => 10,
            "+" | "-" => 11,
            "*" | "/" | "%" => 12,
            "**" => 13,
            _ => 8,
        },
        ExprKind::UnaryOp { .. } => 14,
        ExprKind::Range { .. } => 15,
        ExprKind::Call { .. } | ExprKind::Index { .. } | ExprKind::FieldAccess { .. } => 16,
        ExprKind::Lambda { .. } => 1,
        _ => 17,
    }
}

fn format_params(
    params: &[String],
    defaults: &[Option<Expr>],
    rest_param: Option<&str>,
    formatter: &Formatter,
) -> String {
    let mut out = vec![];
    for (index, param) in params.iter().enumerate() {
        if let Some(Some(expr)) = defaults.get(index) {
            out.push(format!("{} = {}", param, formatter.format_expr(expr)));
        } else {
            out.push(param.clone());
        }
    }
    if let Some(rest_name) = rest_param {
        out.push(format!("...{}", rest_name));
    }
    out.join(", ")
}

fn format_import_name(name: &str) -> String {
    if name.ends_with(".dgm") || name.contains('/') || name.contains('\\') {
        serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into())
    } else {
        name.to_string()
    }
}

fn format_fstring_part(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::StringLit(value) => {
            let escaped = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into());
            escaped
                .trim_matches('"')
                .replace('{', "{{")
                .replace('}', "}}")
        }
        _ => format!("{{{}}}", Formatter::default().format_expr(expr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_function_and_if() {
        let source = "def greet(name){iff name{retrun name}else{retrun \"anon\"}}";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("fn greet(name) {"));
        assert!(formatted.contains("if name {"));
        assert!(formatted.contains("return \"anon\""));
    }

    #[test]
    fn formats_import_and_lambda() {
        let source = "imprt math as m\nlet add=lam(x,y=1)=>x+y";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("import math as m"));
        assert!(formatted.contains("let add = lam(x, y = 1) => x + y"));
    }
}
