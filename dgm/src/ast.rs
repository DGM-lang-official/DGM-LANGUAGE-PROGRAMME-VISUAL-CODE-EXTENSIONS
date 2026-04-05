use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Span {
    pub file: Arc<String>,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(file: Arc<String>, line: usize, col: usize) -> Self {
        Self { file, line, col }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

impl Expr {
    pub fn new(span: Span, kind: ExprKind) -> Self {
        Self { span, kind }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    NullLit,
    Ident(String),
    This,
    BinOp { op: String, left: Box<Expr>, right: Box<Expr> },
    UnaryOp { op: String, operand: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Index { object: Box<Expr>, index: Box<Expr> },
    FieldAccess { object: Box<Expr>, field: String },
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Assign { target: Box<Expr>, op: String, value: Box<Expr> },
    New { class_name: String, args: Vec<Expr> },
    Lambda { params: Vec<String>, body: Vec<Stmt> },
    Ternary { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> },
    StringInterp(Vec<Expr>),
    Range { start: Box<Expr>, end: Box<Expr> },
}

#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    pub span: Span,
    pub kind: StmtKind,
}

impl Stmt {
    pub fn new(span: Span, kind: StmtKind) -> Self {
        Self { span, kind }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum StmtKind {
    Expr(Expr),
    Let { name: String, value: Expr },
    Writ(Expr),
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        elseif_branches: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    While { condition: Expr, body: Vec<Stmt> },
    For { var: String, iterable: Expr, body: Vec<Stmt> },
    FuncDef { name: String, params: Vec<String>, body: Vec<Stmt> },
    Return(Option<Expr>),
    Break,
    Continue,
    ClassDef { name: String, parent: Option<String>, methods: Vec<Stmt> },
    Imprt { name: String, alias: Option<String> },
    TryCatch {
        try_block: Vec<Stmt>,
        catch_var: Option<String>,
        catch_block: Vec<Stmt>,
        finally_block: Option<Vec<Stmt>>,
    },
    Throw(Expr),
    Match {
        expr: Expr,
        arms: Vec<(Expr, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },
}
