#[derive(Debug, Clone)]
pub enum Expr {
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
    // New expressions
    Lambda { params: Vec<String>, body: Vec<Stmt> },
    Ternary { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> },
    StringInterp(Vec<Expr>), // f"hello {name} world"
    Range { start: Box<Expr>, end: Box<Expr> },
}

#[derive(Debug, Clone)]
pub enum Stmt {
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
    Imprt(String),
    // New statements
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
