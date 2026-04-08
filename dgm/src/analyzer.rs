use crate::ast::{Expr, ExprKind, Span, Stmt, StmtKind};
use crate::error::DgmError;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SymbolKind {
    Variable,
    Constant,
    Function,
    Parameter,
    Import,
    Class,
    CatchVar,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceInfo {
    pub name: String,
    pub span: Span,
    pub target: Option<SymbolInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportInfo {
    pub module: String,
    pub alias: String,
    pub span: Span,
    pub resolved: Option<String>,
    pub is_stdlib: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ModuleInfo {
    pub file: String,
    pub exports: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub diagnostics: Vec<DgmError>,
    pub symbols: Vec<SymbolInfo>,
    pub references: Vec<ReferenceInfo>,
    pub modules: HashMap<String, ModuleInfo>,
}

#[derive(Debug, Clone, Default)]
struct ScopeFrame {
    bindings: HashMap<String, SymbolInfo>,
    future_bindings: HashMap<String, SymbolInfo>,
}

#[derive(Debug, Clone)]
struct AnalysisContext {
    file: Arc<String>,
    scopes: Vec<ScopeFrame>,
    allow_this: bool,
    allow_super: bool,
}

#[derive(Debug, Clone)]
enum ImportState {
    Loading,
    Loaded,
    Failed { message: String, circular: bool },
}

pub fn analyze_source(source: &str) -> Result<AnalysisResult, DgmError> {
    analyze_named_source(source, "<source>")
}

pub fn analyze_named_source(
    source: &str,
    source_name: impl Into<String>,
) -> Result<AnalysisResult, DgmError> {
    let source_name = source_name.into();
    let stmts = crate::parse_named_source(source, source_name.clone())?;
    let mut analyzer = Analyzer::default();
    analyzer.analyze_file(Arc::new(source_name), &stmts)?;
    Ok(AnalysisResult {
        diagnostics: analyzer.diagnostics,
        symbols: analyzer.symbols,
        references: analyzer.references,
        modules: analyzer.modules,
    })
}

#[derive(Default)]
struct Analyzer {
    diagnostics: Vec<DgmError>,
    symbols: Vec<SymbolInfo>,
    references: Vec<ReferenceInfo>,
    imports: HashMap<PathBuf, ImportState>,
    modules: HashMap<String, ModuleInfo>,
}

impl Analyzer {
    fn analyze_file(&mut self, file: Arc<String>, stmts: &[Stmt]) -> Result<(), DgmError> {
        self.modules
            .entry(file.as_ref().clone())
            .or_insert_with(|| ModuleInfo {
                file: file.as_ref().clone(),
                exports: collect_block_bindings(stmts).into_values().collect(),
                imports: vec![],
            });

        let mut root = ScopeFrame {
            bindings: builtin_bindings(),
            future_bindings: collect_block_bindings(stmts),
        };
        root.future_bindings
            .extend(root.bindings.iter().map(|(name, symbol)| (name.clone(), symbol.clone())));

        let mut ctx = AnalysisContext {
            file,
            scopes: vec![root],
            allow_this: false,
            allow_super: false,
        };
        self.analyze_stmts(&mut ctx, stmts)
    }

    fn analyze_stmts(
        &mut self,
        ctx: &mut AnalysisContext,
        stmts: &[Stmt],
    ) -> Result<(), DgmError> {
        for stmt in stmts {
            self.analyze_stmt(ctx, stmt)?;
        }
        Ok(())
    }

    fn analyze_block(
        &mut self,
        ctx: &mut AnalysisContext,
        stmts: &[Stmt],
    ) -> Result<(), DgmError> {
        ctx.scopes.push(ScopeFrame {
            bindings: HashMap::new(),
            future_bindings: collect_block_bindings(stmts),
        });
        let result = self.analyze_stmts(ctx, stmts);
        ctx.scopes.pop();
        result
    }

    fn analyze_stmt(
        &mut self,
        ctx: &mut AnalysisContext,
        stmt: &Stmt,
    ) -> Result<(), DgmError> {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.analyze_expr(ctx, expr),
            StmtKind::Let { name, value } => {
                self.analyze_expr(ctx, value);
                self.define_current(ctx, name, SymbolKind::Variable, &stmt.span);
            }
            StmtKind::Const { name, value } => {
                self.analyze_expr(ctx, value);
                self.define_current(ctx, name, SymbolKind::Constant, &stmt.span);
            }
            StmtKind::LetDestructure { names, rest, value } => {
                self.analyze_expr(ctx, value);
                for name in names {
                    self.define_current(ctx, name, SymbolKind::Variable, &stmt.span);
                }
                if let Some(rest_name) = rest {
                    self.define_current(ctx, rest_name, SymbolKind::Variable, &stmt.span);
                }
            }
            StmtKind::Writ(expr) => self.analyze_expr(ctx, expr),
            StmtKind::If {
                condition,
                then_block,
                elseif_branches,
                else_block,
            } => {
                self.analyze_expr(ctx, condition);
                self.analyze_block(ctx, then_block)?;
                for (elseif_cond, block) in elseif_branches {
                    self.analyze_expr(ctx, elseif_cond);
                    self.analyze_block(ctx, block)?;
                }
                if let Some(block) = else_block {
                    self.analyze_block(ctx, block)?;
                }
            }
            StmtKind::While { condition, body } => {
                self.analyze_expr(ctx, condition);
                self.analyze_block(ctx, body)?;
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                self.analyze_expr(ctx, iterable);
                ctx.scopes.push(ScopeFrame {
                    bindings: HashMap::from([(
                        var.clone(),
                        SymbolInfo {
                            name: var.clone(),
                            kind: SymbolKind::Variable,
                            span: stmt.span.clone(),
                        },
                    )]),
                    future_bindings: collect_block_bindings(body),
                });
                self.symbols.push(SymbolInfo {
                    name: var.clone(),
                    kind: SymbolKind::Variable,
                    span: stmt.span.clone(),
                });
                let result = self.analyze_block(ctx, body);
                ctx.scopes.pop();
                result?;
            }
            StmtKind::FuncDef {
                name,
                params,
                defaults,
                rest_param,
                body,
            } => {
                self.define_current(ctx, name, SymbolKind::Function, &stmt.span);
                self.analyze_callable(ctx, params, defaults, rest_param.as_deref(), body, false, false);
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.analyze_expr(ctx, expr);
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::ClassDef {
                name,
                parent,
                methods,
            } => {
                self.define_current(ctx, name, SymbolKind::Class, &stmt.span);
                if let Some(parent_name) = parent {
                    if ctx.lookup(parent_name).is_none() {
                        self.push_diagnostic(
                            DgmError::undefined_variable(parent_name).with_span(stmt.span.clone()),
                        );
                    }
                }
                for method in methods {
                    if let StmtKind::FuncDef {
                        params,
                        defaults,
                        rest_param,
                        body,
                        ..
                    } = &method.kind
                    {
                        self.analyze_callable(
                            ctx,
                            params,
                            defaults,
                            rest_param.as_deref(),
                            body,
                            true,
                            parent.is_some(),
                        );
                    }
                }
            }
            StmtKind::Imprt { name, alias } => self.analyze_import(ctx, stmt, name, alias.as_deref())?,
            StmtKind::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                self.analyze_block(ctx, try_block)?;
                ctx.scopes.push(ScopeFrame {
                    bindings: HashMap::new(),
                    future_bindings: collect_block_bindings(catch_block),
                });
                if let Some(name) = catch_var {
                    self.define_current(ctx, name, SymbolKind::CatchVar, &stmt.span);
                }
                let catch_result = self.analyze_stmts(ctx, catch_block);
                ctx.scopes.pop();
                catch_result?;
                if let Some(block) = finally_block {
                    self.analyze_block(ctx, block)?;
                }
            }
            StmtKind::Throw(expr) => self.analyze_expr(ctx, expr),
            StmtKind::Match { expr, arms, default } => {
                self.analyze_expr(ctx, expr);
                for (pattern, guard, block) in arms {
                    self.analyze_expr(ctx, pattern);
                    if let Some(guard) = guard {
                        self.analyze_expr(ctx, guard);
                    }
                    self.analyze_block(ctx, block)?;
                }
                if let Some(block) = default {
                    self.analyze_block(ctx, block)?;
                }
            }
        }

        Ok(())
    }

    fn analyze_callable(
        &mut self,
        ctx: &AnalysisContext,
        params: &[String],
        defaults: &[Option<Expr>],
        rest_param: Option<&str>,
        body: &[Stmt],
        allow_this: bool,
        allow_super: bool,
    ) {
        let mut callable_ctx = ctx.with_capturable_outer_scopes();
        callable_ctx.allow_this = allow_this;
        callable_ctx.allow_super = allow_super;
        callable_ctx.scopes.push(ScopeFrame::default());

        for (idx, param) in params.iter().enumerate() {
            if let Some(Some(default_expr)) = defaults.get(idx) {
                self.analyze_expr(&callable_ctx, default_expr);
            }
            self.define_current(&mut callable_ctx, param, SymbolKind::Parameter, &body_span(body, ctx.file.clone()));
        }

        if let Some(rest_name) = rest_param {
            self.define_current(
                &mut callable_ctx,
                rest_name,
                SymbolKind::Parameter,
                &body_span(body, ctx.file.clone()),
            );
        }

        let _ = self.analyze_block(&mut callable_ctx, body);
        callable_ctx.scopes.pop();
    }

    fn analyze_import(
        &mut self,
        ctx: &mut AnalysisContext,
        stmt: &Stmt,
        name: &str,
        alias: Option<&str>,
    ) -> Result<(), DgmError> {
        let binding_name = alias
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| name.trim_end_matches(".dgm").to_string());

        if crate::stdlib::load_module(name).is_some() {
            self.record_import(
                ctx.file.as_ref(),
                ImportInfo {
                    module: name.to_string(),
                    alias: binding_name.clone(),
                    span: stmt.span.clone(),
                    resolved: Some(format!("@stdlib:{name}")),
                    is_stdlib: true,
                },
            );
            self.define_current(ctx, &binding_name, SymbolKind::Import, &stmt.span);
            return Ok(());
        }

        let path = resolve_import_path(ctx.file.as_ref(), name);
        self.record_import(
            ctx.file.as_ref(),
            ImportInfo {
                module: name.to_string(),
                alias: binding_name.clone(),
                span: stmt.span.clone(),
                resolved: Some(path.to_string_lossy().to_string()),
                is_stdlib: false,
            },
        );
        match self.imports.get(&path).cloned() {
            Some(ImportState::Loading) => {
                self.push_diagnostic(
                    DgmError::circular_import(format!("circular import detected for '{}'", name))
                        .with_span(stmt.span.clone()),
                );
                self.imports.insert(
                    path,
                    ImportState::Failed {
                        message: format!("circular import detected for '{}'", name),
                        circular: true,
                    },
                );
            }
            Some(ImportState::Loaded) => {}
            Some(ImportState::Failed { message, circular }) => {
                let error = if circular {
                    DgmError::circular_import(message)
                } else {
                    DgmError::import_fail(message)
                };
                self.push_diagnostic(error.with_span(stmt.span.clone()));
            }
            None => {
                self.imports.insert(path.clone(), ImportState::Loading);
                match fs::read_to_string(&path) {
                    Ok(source) => {
                        let source_name = path.to_string_lossy().to_string();
                        let stmts = crate::parse_named_source(&source, source_name.clone())?;
                        self.analyze_file(Arc::new(source_name), &stmts)?;
                        self.imports.insert(path, ImportState::Loaded);
                    }
                    Err(err) => {
                        let message = format!("cannot import '{}': {}", name, err);
                        self.imports.insert(
                            path,
                            ImportState::Failed {
                                message: message.clone(),
                                circular: false,
                            },
                        );
                        self.push_diagnostic(
                            DgmError::import_fail(message).with_span(stmt.span.clone()),
                        );
                    }
                }
            }
        }

        self.define_current(ctx, &binding_name, SymbolKind::Import, &stmt.span);
        Ok(())
    }

    fn analyze_expr(&mut self, ctx: &AnalysisContext, expr: &Expr) {
        match &expr.kind {
            ExprKind::IntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::StringLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::NullLit => {}
            ExprKind::Ident(name) => {
                if let Some(target) = ctx.lookup(name) {
                    self.references.push(ReferenceInfo {
                        name: name.clone(),
                        span: expr.span.clone(),
                        target: Some(target),
                    });
                } else {
                    self.push_diagnostic(DgmError::undefined_variable(name).with_span(expr.span.clone()));
                }
            }
            ExprKind::This => {
                if !ctx.allow_this {
                    self.push_diagnostic(
                        DgmError::runtime("'this' used outside class").with_span(expr.span.clone()),
                    );
                }
            }
            ExprKind::Super => {
                if !ctx.allow_super {
                    self.push_diagnostic(
                        DgmError::runtime("'super' used outside subclass").with_span(expr.span.clone()),
                    );
                }
            }
            ExprKind::BinOp { left, right, .. } => {
                self.analyze_expr(ctx, left);
                self.analyze_expr(ctx, right);
            }
            ExprKind::UnaryOp { operand, .. } => self.analyze_expr(ctx, operand),
            ExprKind::Call { callee, args } => {
                self.analyze_expr(ctx, callee);
                for arg in args {
                    self.analyze_expr(ctx, arg);
                }
            }
            ExprKind::Index { object, index } => {
                self.analyze_expr(ctx, object);
                self.analyze_expr(ctx, index);
            }
            ExprKind::FieldAccess { object, .. } => self.analyze_expr(ctx, object),
            ExprKind::List(items) => {
                for item in items {
                    self.analyze_expr(ctx, item);
                }
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.analyze_expr(ctx, key);
                    self.analyze_expr(ctx, value);
                }
            }
            ExprKind::Assign { target, value, .. } => {
                self.analyze_assign_target(ctx, target);
                self.analyze_expr(ctx, value);
            }
            ExprKind::New { class_name, args } => {
                if ctx.lookup(class_name).is_none() {
                    self.push_diagnostic(
                        DgmError::undefined_variable(class_name).with_span(expr.span.clone()),
                    );
                }
                for arg in args {
                    self.analyze_expr(ctx, arg);
                }
            }
            ExprKind::Lambda {
                params,
                defaults,
                rest_param,
                body,
            } => self.analyze_callable(
                ctx,
                params,
                defaults,
                rest_param.as_deref(),
                body,
                ctx.allow_this,
                ctx.allow_super,
            ),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.analyze_expr(ctx, condition);
                self.analyze_expr(ctx, then_expr);
                self.analyze_expr(ctx, else_expr);
            }
            ExprKind::StringInterp(parts) => {
                for part in parts {
                    self.analyze_expr(ctx, part);
                }
            }
            ExprKind::Range { start, end } => {
                self.analyze_expr(ctx, start);
                self.analyze_expr(ctx, end);
            }
        }
    }

    fn analyze_assign_target(&mut self, ctx: &AnalysisContext, target: &Expr) {
        match &target.kind {
            ExprKind::Ident(name) => {
                if let Some(symbol) = ctx.lookup(name) {
                    self.references.push(ReferenceInfo {
                        name: name.clone(),
                        span: target.span.clone(),
                        target: Some(symbol),
                    });
                } else {
                    self.push_diagnostic(
                        DgmError::undefined_variable(name).with_span(target.span.clone()),
                    );
                }
            }
            ExprKind::FieldAccess { object, .. } => self.analyze_expr(ctx, object),
            ExprKind::Index { object, index } => {
                self.analyze_expr(ctx, object);
                self.analyze_expr(ctx, index);
            }
            _ => self.analyze_expr(ctx, target),
        }
    }

    fn define_current(
        &mut self,
        ctx: &mut AnalysisContext,
        name: &str,
        kind: SymbolKind,
        span: &Span,
    ) {
        if let Some(scope) = ctx.scopes.last_mut() {
            let symbol = SymbolInfo {
                name: name.to_string(),
                kind,
                span: span.clone(),
            };
            scope.bindings.insert(name.to_string(), symbol.clone());
            self.symbols.push(symbol);
        }
    }

    fn push_diagnostic(&mut self, err: DgmError) {
        self.diagnostics.push(err);
    }

    fn record_import(&mut self, file: &str, import: ImportInfo) {
        self.modules
            .entry(file.to_string())
            .or_insert_with(|| ModuleInfo {
                file: file.to_string(),
                exports: vec![],
                imports: vec![],
            })
            .imports
            .push(import);
    }
}

impl AnalysisContext {
    fn lookup(&self, name: &str) -> Option<SymbolInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name).cloned())
    }

    fn with_capturable_outer_scopes(&self) -> Self {
        let mut clone = self.clone();
        for scope in &mut clone.scopes {
            for (name, kind) in scope.future_bindings.clone() {
                scope.bindings.entry(name).or_insert(kind);
            }
        }
        clone
    }
}

fn collect_block_bindings(stmts: &[Stmt]) -> HashMap<String, SymbolInfo> {
    let mut bindings = HashMap::new();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Let { name, .. } => {
                bindings.insert(
                    name.clone(),
                    SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        span: stmt.span.clone(),
                    },
                );
            }
            StmtKind::Const { name, .. } => {
                bindings.insert(
                    name.clone(),
                    SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Constant,
                        span: stmt.span.clone(),
                    },
                );
            }
            StmtKind::LetDestructure { names, rest, .. } => {
                for name in names {
                    bindings.insert(
                        name.clone(),
                        SymbolInfo {
                            name: name.clone(),
                            kind: SymbolKind::Variable,
                            span: stmt.span.clone(),
                        },
                    );
                }
                if let Some(rest_name) = rest {
                    bindings.insert(
                        rest_name.clone(),
                        SymbolInfo {
                            name: rest_name.clone(),
                            kind: SymbolKind::Variable,
                            span: stmt.span.clone(),
                        },
                    );
                }
            }
            StmtKind::FuncDef { name, .. } => {
                bindings.insert(
                    name.clone(),
                    SymbolInfo {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        span: stmt.span.clone(),
                    },
                );
            }
            StmtKind::Imprt { name, alias } => {
                let binding_name = alias
                    .clone()
                    .unwrap_or_else(|| name.trim_end_matches(".dgm").to_string());
                bindings.insert(
                    binding_name.clone(),
                    SymbolInfo {
                        name: binding_name,
                        kind: SymbolKind::Import,
                        span: stmt.span.clone(),
                    },
                );
            }
            _ => {}
        }
    }
    bindings
}

fn builtin_bindings() -> HashMap<String, SymbolInfo> {
    let mut bindings = HashMap::new();
    for name in [
        "len",
        "type",
        "str",
        "int",
        "float",
        "push",
        "pop",
        "range",
        "input",
        "abs",
        "min",
        "max",
        "sort",
        "reverse",
        "keys",
        "values",
        "has_key",
        "slice",
        "join",
        "split",
        "replace",
        "upper",
        "lower",
        "trim",
        "contains",
        "starts_with",
        "ends_with",
        "chars",
        "format",
        "index_of",
        "flat",
        "zip",
        "sum",
        "print",
        "println",
        "chr",
        "ord",
        "hex",
        "bin",
        "exit",
        "assert",
        "map",
        "filter",
        "reduce",
        "each",
        "find",
        "any",
        "all",
    ] {
        bindings.insert(
            name.to_string(),
            SymbolInfo {
                name: name.to_string(),
                kind: SymbolKind::Function,
                span: Span::new(Arc::new("<builtin>".to_string()), 1, 1),
            },
        );
    }
    bindings
}

fn resolve_import_path(current_file: &str, name: &str) -> PathBuf {
    let raw = if name.ends_with(".dgm") {
        PathBuf::from(name)
    } else {
        PathBuf::from(format!("{}.dgm", name))
    };

    let resolved = if raw.is_absolute() {
        raw
    } else if !current_file.starts_with('<') {
        Path::new(current_file)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(raw)
    } else {
        raw
    };

    resolved.canonicalize().unwrap_or(resolved)
}

fn body_span(body: &[Stmt], fallback_file: Arc<String>) -> Span {
    body.first()
        .map(|stmt| stmt.span.clone())
        .unwrap_or_else(|| Span::new(fallback_file, 1, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_undefined_variable() {
        let result = analyze_source("let x = missing").unwrap();
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].message, "undefined variable 'missing'");
    }

    #[test]
    fn allows_future_outer_bindings_in_functions() {
        let result = analyze_source(
            "fn greet() { writ(name) }\nlet name = \"dgm\"\ngreet()\n",
        )
        .unwrap();
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn reports_block_scope_escape() {
        let result = analyze_source("if true { let x = 1 }\nwrit(x)\n").unwrap();
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].message, "undefined variable 'x'");
    }

    #[test]
    fn resolves_relative_imports() {
        let temp_dir = std::env::temp_dir().join(format!(
            "dgm-analyzer-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let helper = temp_dir.join("helper.dgm");
        let main = temp_dir.join("main.dgm");
        fs::write(&helper, "fn value() { return 1 }\n").unwrap();
        fs::write(&main, "import helper\nwrit(helper.value())\n").unwrap();

        let source = fs::read_to_string(&main).unwrap();
        let result = analyze_named_source(&source, main.to_string_lossy().to_string()).unwrap();
        assert!(result.diagnostics.is_empty());

        let _ = fs::remove_file(&helper);
        let _ = fs::remove_file(&main);
        let _ = fs::remove_dir(&temp_dir);
    }

    #[test]
    fn reports_unknown_class_construction() {
        let result = analyze_source("let value = new Missing()\n").unwrap();
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].message, "undefined variable 'Missing'");
    }
}
