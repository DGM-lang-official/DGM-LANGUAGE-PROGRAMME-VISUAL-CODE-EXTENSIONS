use crate::ast::{Expr, ExprKind, Span, Stmt, StmtKind};
use crate::environment::Environment;
use crate::error::{DgmError, ErrorCode, StackFrame};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

macro_rules! numeric_op {
    ($left:expr, $right:expr, $int_op:ident, $float_op:tt, $name:expr) => {
        match ($left, $right) {
            (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a.$int_op(b))),
            (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a $float_op b)),
            (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a as f64 $float_op b)),
            (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a $float_op b as f64)),
            _ => Err(DgmError::runtime(format!("'{}' type mismatch", $name))),
        }
    };
}

macro_rules! cmp_op {
    ($left:expr, $right:expr, $op:tt, $name:expr) => {
        match ($left, $right) {
            (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Bool(a $op b)),
            (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Bool(a $op b)),
            (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Bool((a as f64) $op b)),
            (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Bool(a $op (b as f64))),
            (DgmValue::Str(a), DgmValue::Str(b)) => Ok(DgmValue::Bool(a $op b)),
            _ => Err(DgmError::runtime(format!("'{}' type mismatch", $name))),
        }
    };
}

#[derive(Debug, Clone)]
pub enum DgmValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    List(Rc<RefCell<Vec<DgmValue>>>),
    Map(Rc<RefCell<HashMap<String, DgmValue>>>),
    Function {
        name: Option<String>,
        params: Vec<String>,
        defaults: Vec<Option<Expr>>,
        rest_param: Option<String>,
        body: Vec<Stmt>,
        closure: Rc<RefCell<Environment>>,
    },
    NativeFunction {
        name: String,
        func: NativeFunction,
    },
    Instance {
        class_name: String,
        fields: Rc<RefCell<HashMap<String, DgmValue>>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum NativeFunction {
    Simple(fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>),
    Contextual(fn(&mut Interpreter, Vec<DgmValue>, &Span) -> Result<DgmValue, DgmError>),
}

impl NativeFunction {
    pub fn simple(func: fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>) -> Self {
        Self::Simple(func)
    }

    pub fn contextual(
        func: fn(&mut Interpreter, Vec<DgmValue>, &Span) -> Result<DgmValue, DgmError>,
    ) -> Self {
        Self::Contextual(func)
    }

    pub fn invoke(
        self,
        interp: &mut Interpreter,
        args: Vec<DgmValue>,
        span: &Span,
    ) -> Result<DgmValue, DgmError> {
        match self {
            Self::Simple(func) => func(args),
            Self::Contextual(func) => func(interp, args, span),
        }
    }
}

impl fmt::Display for DgmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DgmValue::Int(n) => write!(f, "{}", n),
            DgmValue::Float(n) => write!(f, "{}", n),
            DgmValue::Str(s) => write!(f, "{}", s),
            DgmValue::Bool(b) => write!(f, "{}", if *b { "tru" } else { "fals" }),
            DgmValue::Null => write!(f, "nul"),
            DgmValue::List(l) => {
                let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            DgmValue::Map(m) => {
                let pairs: Vec<String> = m
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", pairs.join(", "))
            }
            DgmValue::Function { params, .. } => write!(f, "<fn({})>", params.join(", ")),
            DgmValue::NativeFunction { name, .. } => write!(f, "<native {}>", name),
            DgmValue::Instance { class_name, fields, .. } => {
                if let Some(DgmValue::Function { .. }) = fields.borrow().get("__str__") {
                    // __str__ display handled at call site
                    write!(f, "<{} instance>", class_name)
                } else {
                    write!(f, "<{} instance>", class_name)
                }
            }
        }
    }
}

pub enum ControlFlow {
    None,
    Return(DgmValue),
    Break,
    Continue,
}

#[derive(Clone)]
enum ModuleState {
    Loading,
    Loaded(DgmValue),
    Failed(DgmError),
}

pub struct Interpreter {
    pub globals: Rc<RefCell<Environment>>,
    classes: HashMap<String, ClassDef>,
    modules: HashMap<String, ModuleState>,
    current_source: Arc<String>,
    call_stack: Vec<StackFrame>,
}

#[derive(Clone)]
struct ClassDef {
    methods: Vec<Stmt>,
    parent: Option<String>,
}

#[derive(Clone)]
enum Callable {
    User {
        frame_name: String,
        params: Vec<String>,
        defaults: Vec<Option<Expr>>,
        rest_param: Option<String>,
        body: Vec<Stmt>,
        closure: Rc<RefCell<Environment>>,
        bound_self: Option<DgmValue>,
    },
    Native {
        frame_name: String,
        func: NativeFunction,
    },
}

impl Callable {
    fn frame_name(&self) -> &str {
        match self {
            Self::User { frame_name, .. } | Self::Native { frame_name, .. } => frame_name,
        }
    }
}

impl Interpreter {
    pub fn new(source_name: Arc<String>) -> Self {
        let globals = Rc::new(RefCell::new(Environment::new()));
        let simple_natives: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
            ("len", native_len),
            ("type", native_type),
            ("str", native_str),
            ("int", native_int),
            ("float", native_float),
            ("push", native_push),
            ("pop", native_pop),
            ("range", native_range),
            ("input", native_input),
            ("abs", native_abs),
            ("min", native_min),
            ("max", native_max),
            ("sort", native_sort),
            ("reverse", native_reverse),
            ("keys", native_keys),
            ("values", native_values),
            ("has_key", native_has_key),
            ("slice", native_slice),
            ("join", native_join),
            ("split", native_split),
            ("replace", native_replace),
            ("upper", native_upper),
            ("lower", native_lower),
            ("trim", native_trim),
            ("contains", native_contains),
            ("starts_with", native_starts_with),
            ("ends_with", native_ends_with),
            ("chars", native_chars),
            ("format", native_format),
            ("index_of", native_index_of),
            ("flat", native_flat),
            ("zip", native_zip),
            ("sum", native_sum),
            ("print", native_print),
            ("println", native_println),
            ("chr", native_chr),
            ("ord", native_ord),
            ("hex", native_hex),
            ("bin", native_bin),
            ("exit", native_exit),
            ("assert", native_assert),
        ];
        for (name, func) in simple_natives {
            globals.borrow_mut().set(
                name,
                DgmValue::NativeFunction {
                    name: name.to_string(),
                    func: NativeFunction::simple(*func),
                },
            );
        }

        let contextual_natives: &[(
            &str,
            fn(&mut Interpreter, Vec<DgmValue>, &Span) -> Result<DgmValue, DgmError>,
        )] = &[
            ("map", native_map_fn),
            ("filter", native_filter),
            ("reduce", native_reduce),
            ("each", native_each),
            ("find", native_find),
            ("any", native_any),
            ("all", native_all),
        ];
        for (name, func) in contextual_natives {
            globals.borrow_mut().set(
                name,
                DgmValue::NativeFunction {
                    name: name.to_string(),
                    func: NativeFunction::contextual(*func),
                },
            );
        }
        Self {
            globals,
            classes: HashMap::new(),
            modules: HashMap::new(),
            current_source: source_name,
            call_stack: vec![],
        }
    }

    pub fn run(&mut self, stmts: Vec<Stmt>) -> Result<(), DgmError> {
        let env = Rc::clone(&self.globals);
        for stmt in &stmts {
            match self.exec_stmt(stmt, Rc::clone(&env))? {
                ControlFlow::None => {}
                ControlFlow::Return(_) | ControlFlow::Break | ControlFlow::Continue => break,
            }
        }
        Ok(())
    }

    pub fn exec_stmt(
        &mut self,
        stmt: &Stmt,
        env: Rc<RefCell<Environment>>,
    ) -> Result<ControlFlow, DgmError> {
        let result = match &stmt.kind {
            StmtKind::Expr(expr) => {
                self.eval_expr(expr, env)?;
                Ok(ControlFlow::None)
            }
            StmtKind::Let { name, value } => {
                let v = self.eval_expr(value, Rc::clone(&env))?;
                env.borrow_mut().set(name, v);
                Ok(ControlFlow::None)
            }
            StmtKind::Const { name, value } => {
                let v = self.eval_expr(value, Rc::clone(&env))?;
                env.borrow_mut().set_const(name, v);
                Ok(ControlFlow::None)
            }
            StmtKind::LetDestructure { names, rest, value } => {
                let v = self.eval_expr(value, Rc::clone(&env))?;
                match v {
                    DgmValue::List(l) => {
                        let list = l.borrow();
                        for (i, name) in names.iter().enumerate() {
                            let val = list.get(i).cloned().unwrap_or(DgmValue::Null);
                            env.borrow_mut().set(name, val);
                        }
                        if let Some(rest_name) = rest {
                            let rest_vals: Vec<DgmValue> = list.iter().skip(names.len()).cloned().collect();
                            env.borrow_mut().set(rest_name, DgmValue::List(Rc::new(RefCell::new(rest_vals))));
                        }
                    }
                    _ => return Err(DgmError::runtime("destructuring requires a list")),
                }
                Ok(ControlFlow::None)
            }
            StmtKind::Writ(expr) => {
                let v = self.eval_expr(expr, Rc::clone(&env))?;
                let display = self.value_to_string(&v, &stmt.span)?;
                println!("{}", display);
                Ok(ControlFlow::None)
            }
            StmtKind::If {
                condition,
                then_block,
                elseif_branches,
                else_block,
            } => {
                let cond_val = self.eval_expr(condition, Rc::clone(&env))?;
                if self.is_truthy(&cond_val) {
                    return self.exec_block(then_block, Rc::clone(&env));
                }
                for (cond_expr, block) in elseif_branches {
                    let branch_val = self.eval_expr(cond_expr, Rc::clone(&env))?;
                    if self.is_truthy(&branch_val) {
                        return self.exec_block(block, Rc::clone(&env));
                    }
                }
                if let Some(block) = else_block {
                    return self.exec_block(block, Rc::clone(&env));
                }
                Ok(ControlFlow::None)
            }
            StmtKind::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expr(condition, Rc::clone(&env))?;
                    if !self.is_truthy(&cond_val) {
                        break;
                    }
                    match self.exec_block(body, Rc::clone(&env))? {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Continue | ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            StmtKind::For {
                var,
                iterable,
                body,
            } => {
                let iter_val = self.eval_expr(iterable, Rc::clone(&env))?;
                let items = match iter_val {
                    DgmValue::List(l) => l.borrow().clone(),
                    DgmValue::Str(s) => s
                        .chars()
                        .map(|c| DgmValue::Str(c.to_string()))
                        .collect(),
                    DgmValue::Map(m) => m
                        .borrow()
                        .keys()
                        .map(|k| DgmValue::Str(k.clone()))
                        .collect(),
                    _ => {
                        return Err(self.context_error(
                            DgmError::runtime("for loop requires iterable"),
                            &stmt.span,
                        ))
                    }
                };
                for item in items {
                    let loop_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&env))));
                    loop_env.borrow_mut().set(var, item);
                    match self.exec_block(body, loop_env)? {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::Continue | ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            StmtKind::FuncDef { name, params, defaults, rest_param, body } => {
                let func = DgmValue::Function {
                    name: Some(name.clone()),
                    params: params.clone(),
                    defaults: defaults.clone(),
                    rest_param: rest_param.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&env),
                };
                env.borrow_mut().set(name, func);
                Ok(ControlFlow::None)
            }
            StmtKind::Return(expr) => {
                let v = if let Some(e) = expr {
                    self.eval_expr(e, env)?
                } else {
                    DgmValue::Null
                };
                Ok(ControlFlow::Return(v))
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Continue => Ok(ControlFlow::Continue),
            StmtKind::ClassDef {
                name,
                parent,
                methods,
            } => {
                self.classes.insert(
                    name.clone(),
                    ClassDef {
                        methods: methods.clone(),
                        parent: parent.clone(),
                    },
                );
                Ok(ControlFlow::None)
            }
            StmtKind::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                let result = self.exec_block(try_block, Rc::clone(&env));
                let cf = match result {
                    Ok(cf) => cf,
                    Err(e) => {
                        let catch_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&env))));
                        if let Some(var) = catch_var {
                            catch_env.borrow_mut().set(var, DgmValue::Str(e.summary()));
                        }
                        self.exec_block(catch_block, catch_env)?
                    }
                };
                if let Some(fb) = finally_block {
                    self.exec_block(fb, Rc::clone(&env))?;
                }
                Ok(cf)
            }
            StmtKind::Throw(expr) => {
                let v = self.eval_expr(expr, env)?;
                Err(DgmError::thrown(format!("{}", v)))
            }
            StmtKind::Match { expr, arms, default } => {
                let val = self.eval_expr(expr, Rc::clone(&env))?;
                for (pattern, guard, block) in arms {
                    let pat_val = self.eval_expr(pattern, Rc::clone(&env))?;
                    if dgm_eq(&val, &pat_val) {
                        if let Some(guard_expr) = guard {
                            let guard_val = self.eval_expr(guard_expr, Rc::clone(&env))?;
                            if !self.is_truthy(&guard_val) {
                                continue;
                            }
                        }
                        return self.exec_block(block, Rc::clone(&env));
                    }
                }
                if let Some(block) = default {
                    return self.exec_block(block, Rc::clone(&env));
                }
                Ok(ControlFlow::None)
            }
            StmtKind::Imprt { name, alias } => {
                self.do_import(name, alias.as_deref(), env)
                    .map_err(|err| self.context_error(err, &stmt.span))?;
                Ok(ControlFlow::None)
            }
        };

        result.map_err(|err| self.context_error(err, &stmt.span))
    }

    fn exec_block(
        &mut self,
        stmts: &[Stmt],
        parent_env: Rc<RefCell<Environment>>,
    ) -> Result<ControlFlow, DgmError> {
        let env = Rc::new(RefCell::new(Environment::new_child(parent_env)));
        for stmt in stmts {
            match self.exec_stmt(stmt, Rc::clone(&env))? {
                ControlFlow::None => {}
                cf => return Ok(cf),
            }
        }
        Ok(ControlFlow::None)
    }

    pub fn eval_expr(
        &mut self,
        expr: &Expr,
        env: Rc<RefCell<Environment>>,
    ) -> Result<DgmValue, DgmError> {
        let result = match &expr.kind {
            ExprKind::IntLit(n) => Ok(DgmValue::Int(*n)),
            ExprKind::FloatLit(f) => Ok(DgmValue::Float(*f)),
            ExprKind::StringLit(s) => Ok(DgmValue::Str(s.clone())),
            ExprKind::BoolLit(b) => Ok(DgmValue::Bool(*b)),
            ExprKind::NullLit => Ok(DgmValue::Null),
            ExprKind::This => env
                .borrow()
                .get("__self__")
                .ok_or_else(|| DgmError::runtime("'ths' used outside class")),
            ExprKind::Super => env
                .borrow()
                .get("__super__")
                .ok_or_else(|| DgmError::runtime("'super' used outside subclass")),
            ExprKind::Ident(name) => env
                .borrow()
                .get(name)
                .ok_or_else(|| DgmError::undefined_variable(name)),
            ExprKind::BinOp { op, left, right } => {
                if op == "and" {
                    let l = self.eval_expr(left, Rc::clone(&env))?;
                    if !self.is_truthy(&l) {
                        return Ok(DgmValue::Bool(false));
                    }
                    let r = self.eval_expr(right, env)?;
                    return Ok(DgmValue::Bool(self.is_truthy(&r)));
                }
                if op == "or" {
                    let l = self.eval_expr(left, Rc::clone(&env))?;
                    if self.is_truthy(&l) {
                        return Ok(DgmValue::Bool(true));
                    }
                    let r = self.eval_expr(right, env)?;
                    return Ok(DgmValue::Bool(self.is_truthy(&r)));
                }
                let l = self.eval_expr(left, Rc::clone(&env))?;
                let r = self.eval_expr(right, Rc::clone(&env))?;
                self.apply_binop(op, l, r)
            }
            ExprKind::UnaryOp { op, operand } => {
                let v = self.eval_expr(operand, env)?;
                match op.as_str() {
                    "not" => Ok(DgmValue::Bool(!self.is_truthy(&v))),
                    "-" => match v {
                        DgmValue::Int(n) => Ok(DgmValue::Int(-n)),
                        DgmValue::Float(f) => Ok(DgmValue::Float(-f)),
                        _ => Err(DgmError::runtime("unary '-' on non-number")),
                    },
                    "~" => match v {
                        DgmValue::Int(n) => Ok(DgmValue::Int(!n)),
                        _ => Err(DgmError::runtime("bitwise '~' requires int")),
                    },
                    _ => Err(DgmError::runtime(format!("unknown unary op '{}'", op))),
                }
            }
            ExprKind::Assign { target, op, value } => {
                let val = self.eval_expr(value, Rc::clone(&env))?;
                self.do_assign(target, op, val, env)
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::FieldAccess { object, field } = &callee.kind {
                    // super.method() - bind self from current scope
                    if matches!(object.kind, ExprKind::Super) {
                        let super_map = env.borrow().get("__super__")
                            .ok_or_else(|| DgmError::runtime("'super' used outside subclass"))?;
                        let current_self = env.borrow().get("__self__");
                        if let DgmValue::Map(ref m) = super_map {
                            if let Some(func) = m.borrow().get(field).cloned() {
                                let arg_vals: Vec<DgmValue> = args
                                    .iter()
                                    .map(|a| self.eval_expr(a, Rc::clone(&env)))
                                    .collect::<Result<_, _>>()?;
                                return self.call_value(
                                    func,
                                    arg_vals,
                                    &expr.span,
                                    current_self,
                                    Some(field.as_str()),
                                );
                            }
                        }
                        return Err(DgmError::invalid_call(format!("no super method '{}'", field)));
                    }
                    let obj = self.eval_expr(object, Rc::clone(&env))?;
                    if let DgmValue::Instance { ref fields, .. } = obj {
                        let method = fields.borrow().get(field).cloned().ok_or_else(|| {
                            DgmError::invalid_call(format!("no method '{}'", field))
                        })?;
                        let arg_vals: Vec<DgmValue> = args
                            .iter()
                            .map(|a| self.eval_expr(a, Rc::clone(&env)))
                            .collect::<Result<_, _>>()?;
                        return self.call_value(
                            method,
                            arg_vals,
                            &expr.span,
                            Some(obj.clone()),
                            Some(field.as_str()),
                        );
                    }
                    if let DgmValue::Map(ref m) = obj {
                        if let Some(func) = m.borrow().get(field).cloned() {
                            let arg_vals: Vec<DgmValue> = args
                                .iter()
                                .map(|a| self.eval_expr(a, Rc::clone(&env)))
                                .collect::<Result<_, _>>()?;
                            return self.call_value(
                                func,
                                arg_vals,
                                &expr.span,
                                None,
                                Some(field.as_str()),
                            );
                        }
                    }
                    return Err(DgmError::invalid_call(format!(
                        "cannot call '{}' on {}",
                        field, obj
                    )));
                }

                let callee_val = self.eval_expr(callee, Rc::clone(&env))?;
                let arg_vals: Vec<DgmValue> = args
                    .iter()
                    .map(|a| self.eval_expr(a, Rc::clone(&env)))
                    .collect::<Result<_, _>>()?;
                self.call_value(callee_val, arg_vals, &expr.span, None, None)
            }
            ExprKind::FieldAccess { object, field } => {
                let obj = self.eval_expr(object, env)?;
                match &obj {
                    DgmValue::Instance { fields, .. } => fields
                        .borrow()
                        .get(field)
                        .cloned()
                        .ok_or_else(|| DgmError::runtime(format!("no field '{}'", field))),
                    DgmValue::Map(m) => m
                        .borrow()
                        .get(field)
                        .cloned()
                        .ok_or_else(|| DgmError::runtime(format!("key '{}' not found", field))),
                    DgmValue::Str(s) => match field.as_str() {
                        "length" => Ok(DgmValue::Int(s.len() as i64)),
                        _ => Err(DgmError::runtime(format!(
                            "no property '{}' on string",
                            field
                        ))),
                    },
                    DgmValue::List(l) => match field.as_str() {
                        "length" => Ok(DgmValue::Int(l.borrow().len() as i64)),
                        _ => Err(DgmError::runtime(format!(
                            "no property '{}' on list",
                            field
                        ))),
                    },
                    _ => Err(DgmError::runtime(format!(
                        "cannot access field '{}' on {}",
                        field, obj
                    ))),
                }
            }
            ExprKind::Index { object, index } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                let idx = self.eval_expr(index, env)?;
                match (&obj, &idx) {
                    (DgmValue::List(l), DgmValue::Int(i)) => {
                        let list = l.borrow();
                        let i = if *i < 0 { list.len() as i64 + i } else { *i } as usize;
                        list.get(i)
                            .cloned()
                            .ok_or_else(|| DgmError::invalid_index("list index out of range"))
                    }
                    (DgmValue::Map(m), DgmValue::Str(k)) => m
                        .borrow()
                        .get(k)
                        .cloned()
                        .ok_or_else(|| DgmError::invalid_index(format!("key '{}' not found", k))),
                    (DgmValue::Str(s), DgmValue::Int(i)) => {
                        let i = if *i < 0 { s.len() as i64 + i } else { *i } as usize;
                        s.chars().nth(i).map(|c| DgmValue::Str(c.to_string())).ok_or_else(
                            || DgmError::invalid_index("string index out of range"),
                        )
                    }
                    _ => Err(DgmError::invalid_index("invalid index operation")),
                }
            }
            ExprKind::List(items) => {
                let vals: Vec<DgmValue> = items
                    .iter()
                    .map(|e| self.eval_expr(e, Rc::clone(&env)))
                    .collect::<Result<_, _>>()?;
                Ok(DgmValue::List(Rc::new(RefCell::new(vals))))
            }
            ExprKind::Map(pairs) => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    let key = match self.eval_expr(k, Rc::clone(&env))? {
                        DgmValue::Str(s) => s,
                        other => format!("{}", other),
                    };
                    let val = self.eval_expr(v, Rc::clone(&env))?;
                    map.insert(key, val);
                }
                Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
            }
            ExprKind::New { class_name, args } => {
                self.instantiate_class(class_name, args, env, &expr.span)
            }
            ExprKind::Lambda { params, defaults, rest_param, body } => Ok(DgmValue::Function {
                name: None,
                params: params.clone(),
                defaults: defaults.clone(),
                rest_param: rest_param.clone(),
                body: body.clone(),
                closure: Rc::clone(&env),
            }),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = self.eval_expr(condition, Rc::clone(&env))?;
                if self.is_truthy(&cond) {
                    self.eval_expr(then_expr, env)
                } else {
                    self.eval_expr(else_expr, env)
                }
            }
            ExprKind::StringInterp(parts) => {
                let mut result = String::new();
                for part in parts {
                    result.push_str(&format!("{}", self.eval_expr(part, Rc::clone(&env))?));
                }
                Ok(DgmValue::Str(result))
            }
            ExprKind::Range { start, end } => {
                let s = match self.eval_expr(start, Rc::clone(&env))? {
                    DgmValue::Int(n) => n,
                    _ => return Err(DgmError::runtime("range requires int")),
                };
                let e = match self.eval_expr(end, env)? {
                    DgmValue::Int(n) => n,
                    _ => return Err(DgmError::runtime("range requires int")),
                };
                let list: Vec<DgmValue> = (s..e).map(DgmValue::Int).collect();
                Ok(DgmValue::List(Rc::new(RefCell::new(list))))
            }
        };

        result.map_err(|err| self.context_error(err, &expr.span))
    }

    fn instantiate_class(
        &mut self,
        class_name: &str,
        args: &[Expr],
        env: Rc<RefCell<Environment>>,
        span: &Span,
    ) -> Result<DgmValue, DgmError> {
        let class = self
            .classes
            .get(class_name)
            .cloned()
            .ok_or_else(|| DgmError::runtime(format!("undefined class '{}'", class_name)))?;
        let fields: Rc<RefCell<HashMap<String, DgmValue>>> = Rc::new(RefCell::new(HashMap::new()));
        let instance = DgmValue::Instance {
            class_name: class_name.to_string(),
            fields: Rc::clone(&fields),
        };

        let all_methods = self.collect_methods(&class);

        // Collect parent-only methods for super
        let mut super_methods: HashMap<String, DgmValue> = HashMap::new();
        if let Some(ref parent_name) = class.parent {
            if let Some(parent_class) = self.classes.get(parent_name).cloned() {
                let parent_meths = self.collect_methods(&parent_class);
                for method in &parent_meths {
                    if let StmtKind::FuncDef { name, params, defaults, rest_param, body } = &method.kind {
                        super_methods.insert(
                            name.clone(),
                            DgmValue::Function {
                                name: Some(name.clone()),
                                params: params.clone(),
                                defaults: defaults.clone(),
                                rest_param: rest_param.clone(),
                                body: body.clone(),
                                closure: Rc::clone(&env),
                            },
                        );
                    }
                }
            }
        }

        for method in &all_methods {
            if let StmtKind::FuncDef { name, params, defaults, rest_param, body } = &method.kind {
                fields.borrow_mut().insert(
                    name.clone(),
                    DgmValue::Function {
                        name: Some(name.clone()),
                        params: params.clone(),
                        defaults: defaults.clone(),
                        rest_param: rest_param.clone(),
                        body: body.clone(),
                        closure: Rc::clone(&env),
                    },
                );
            }
        }

        // Store __super__ for super.method() calls
        if !super_methods.is_empty() {
            fields.borrow_mut().insert(
                "__super__".to_string(),
                DgmValue::Map(Rc::new(RefCell::new(super_methods))),
            );
        }

        if let Some(init_fn) = fields.borrow().get("init").cloned() {
            let arg_vals: Vec<DgmValue> = args
                .iter()
                .map(|a| self.eval_expr(a, Rc::clone(&env)))
                .collect::<Result<_, _>>()?;
            self.call_value(init_fn, arg_vals, span, Some(instance.clone()), Some("init"))?;
        }

        Ok(instance)
    }

    fn collect_methods(&self, class: &ClassDef) -> Vec<Stmt> {
        let mut methods = vec![];
        if let Some(ref parent_name) = class.parent {
            if let Some(parent_class) = self.classes.get(parent_name) {
                methods = self.collect_methods(parent_class);
            }
        }
        for method in &class.methods {
            if let StmtKind::FuncDef { name: method_name, .. } = &method.kind {
                methods.retain(|existing| match &existing.kind {
                    StmtKind::FuncDef { name: existing_name, .. } => existing_name != method_name,
                    _ => true,
                });
            }
            methods.push(method.clone());
        }
        methods
    }

    fn do_assign(
        &mut self,
        target: &Expr,
        op: &str,
        val: DgmValue,
        env: Rc<RefCell<Environment>>,
    ) -> Result<DgmValue, DgmError> {
        match &target.kind {
            ExprKind::Ident(name) => {
                let final_val = if op == "=" {
                    val
                } else {
                    let current = env
                        .borrow()
                        .get(name)
                        .ok_or_else(|| DgmError::undefined_variable(name))?;
                    self.apply_binop(&op[..op.len() - 1], current, val)?
                };
                if env.borrow().get(name).is_some() {
                    env.borrow_mut().assign(name, final_val.clone())?;
                } else {
                    env.borrow_mut().set(name, final_val.clone());
                }
                Ok(final_val)
            }
            ExprKind::FieldAccess { object, field } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                match obj {
                    DgmValue::Instance { fields, .. } => {
                        let final_val = if op == "=" {
                            val
                        } else {
                            let current = fields.borrow().get(field).cloned().unwrap_or(DgmValue::Null);
                            self.apply_binop(&op[..op.len() - 1], current, val)?
                        };
                        fields.borrow_mut().insert(field.clone(), final_val.clone());
                        Ok(final_val)
                    }
                    DgmValue::Map(m) => {
                        let final_val = if op == "=" {
                            val
                        } else {
                            let current = m.borrow().get(field).cloned().unwrap_or(DgmValue::Null);
                            self.apply_binop(&op[..op.len() - 1], current, val)?
                        };
                        m.borrow_mut().insert(field.clone(), final_val.clone());
                        Ok(final_val)
                    }
                    _ => Err(DgmError::runtime("field assign on non-instance")),
                }
            }
            ExprKind::Index { object, index } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                let idx = self.eval_expr(index, Rc::clone(&env))?;
                match (&obj, &idx) {
                    (DgmValue::List(l), DgmValue::Int(i)) => {
                        let len = l.borrow().len();
                        let i = if *i < 0 { len as i64 + i } else { *i } as usize;
                        if i >= len {
                            return Err(DgmError::invalid_index("index out of range"));
                        }
                        let final_val = if op == "=" {
                            val
                        } else {
                            self.apply_binop(&op[..op.len() - 1], l.borrow()[i].clone(), val)?
                        };
                        l.borrow_mut()[i] = final_val.clone();
                        Ok(final_val)
                    }
                    (DgmValue::Map(m), DgmValue::Str(k)) => {
                        m.borrow_mut().insert(k.clone(), val.clone());
                        Ok(val)
                    }
                    _ => Err(DgmError::invalid_index("invalid index assign")),
                }
            }
            _ => Err(DgmError::runtime("invalid assignment target")),
        }
    }

    fn prepare_callable(
        &self,
        callee: DgmValue,
        bound_self: Option<DgmValue>,
        preferred_name: Option<&str>,
        call_span: &Span,
    ) -> Result<Callable, DgmError> {
        match callee {
            DgmValue::NativeFunction { name, func } => Ok(Callable::Native {
                frame_name: preferred_name.unwrap_or(name.as_str()).to_string(),
                func,
            }),
            DgmValue::Function {
                name,
                params,
                defaults,
                rest_param,
                body,
                closure,
            } => Ok(Callable::User {
                frame_name: preferred_name
                    .map(|name| name.to_string())
                    .or(name)
                    .unwrap_or_else(|| "<lambda>".to_string()),
                params,
                defaults,
                rest_param,
                body,
                closure,
                bound_self,
            }),
            _ => Err(self.runtime_error(
                ErrorCode::InvalidCall,
                "value is not callable",
                call_span,
            )),
        }
    }

    pub fn call_value(
        &mut self,
        callee: DgmValue,
        args: Vec<DgmValue>,
        call_span: &Span,
        bound_self: Option<DgmValue>,
        preferred_name: Option<&str>,
    ) -> Result<DgmValue, DgmError> {
        let callable = self.prepare_callable(callee, bound_self, preferred_name, call_span)?;
        self.call_callable(callable, args, call_span)
    }

    fn call_callable(
        &mut self,
        callable: Callable,
        args: Vec<DgmValue>,
        call_span: &Span,
    ) -> Result<DgmValue, DgmError> {
        let frame_name = callable.frame_name().to_string();
        let call_span = call_span.clone();
        self.with_frame(frame_name, call_span.clone(), move |interp| match callable {
            Callable::Native { func, .. } => func.invoke(interp, args, &call_span),
            Callable::User {
                params,
                defaults,
                rest_param,
                body,
                closure,
                bound_self,
                ..
            } => {
                let min_args = params.iter().zip(defaults.iter())
                    .filter(|(_, d)| d.is_none())
                    .count();
                let max_args = params.len();

                if rest_param.is_some() {
                    if args.len() < min_args {
                        return Err(DgmError::invalid_call(format!(
                            "expected at least {} args, got {}",
                            min_args, args.len()
                        )));
                    }
                } else if args.len() < min_args || args.len() > max_args {
                    return Err(DgmError::invalid_call(format!(
                        "expected {} args, got {}",
                        if min_args == max_args {
                            format!("{}", max_args)
                        } else {
                            format!("{}-{}", min_args, max_args)
                        },
                        args.len()
                    )));
                }

                let call_env = Rc::new(RefCell::new(Environment::new_child(closure)));
                if let Some(this_value) = &bound_self {
                    call_env.borrow_mut().set("__self__", this_value.clone());
                    // propagate __super__ from instance fields
                    if let DgmValue::Instance { fields, .. } = this_value {
                        if let Some(super_val) = fields.borrow().get("__super__").cloned() {
                            call_env.borrow_mut().set("__super__", super_val);
                        }
                    }
                }

                // Bind params with defaults
                for (i, p) in params.iter().enumerate() {
                    let val = if i < args.len() {
                        args[i].clone()
                    } else if let Some(Some(default_expr)) = defaults.get(i) {
                        interp.eval_expr(default_expr, Rc::clone(&call_env))?
                    } else {
                        DgmValue::Null
                    };
                    call_env.borrow_mut().set(p, val);
                }

                // Bind rest param
                if let Some(ref rest_name) = rest_param {
                    let rest_vals: Vec<DgmValue> = args.into_iter().skip(params.len()).collect();
                    call_env.borrow_mut().set(rest_name, DgmValue::List(Rc::new(RefCell::new(rest_vals))));
                }

                match interp.exec_block(&body, call_env)? {
                    ControlFlow::Return(v) => Ok(v),
                    ControlFlow::None | ControlFlow::Break | ControlFlow::Continue => {
                        Ok(DgmValue::Null)
                    }
                }
            }
        })
    }

    fn value_to_string(&mut self, val: &DgmValue, span: &Span) -> Result<String, DgmError> {
        if let DgmValue::Instance { fields, .. } = val {
            if let Some(str_fn) = fields.borrow().get("__str__").cloned() {
                let result = self.call_value(str_fn, vec![], span, Some(val.clone()), Some("__str__"))?;
                return Ok(format!("{}", result));
            }
        }
        Ok(format!("{}", val))
    }

    fn is_truthy(&self, value: &DgmValue) -> bool {
        match value {
            DgmValue::Bool(b) => *b,
            DgmValue::Null => false,
            DgmValue::Int(n) => *n != 0,
            DgmValue::Float(f) => *f != 0.0,
            DgmValue::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    fn apply_binop(&self, op: &str, left: DgmValue, right: DgmValue) -> Result<DgmValue, DgmError> {
        match op {
            "+" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a.wrapping_add(b))),
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a + b)),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a as f64 + b)),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a + b as f64)),
                (DgmValue::Str(a), DgmValue::Str(b)) => Ok(DgmValue::Str(a + &b)),
                (DgmValue::Str(a), b) => Ok(DgmValue::Str(a + &format!("{}", b))),
                (a, DgmValue::Str(b)) => Ok(DgmValue::Str(format!("{}", a) + &b)),
                (DgmValue::List(a), DgmValue::List(b)) => {
                    let mut v = a.borrow().clone();
                    v.extend(b.borrow().clone());
                    Ok(DgmValue::List(Rc::new(RefCell::new(v))))
                }
                _ => Err(DgmError::runtime("'+' type mismatch")),
            },
            "-" => numeric_op!(left, right, wrapping_sub, -, "-"),
            "*" => numeric_op!(left, right, wrapping_mul, *, "*"),
            "/" => match (&left, &right) {
                (_, DgmValue::Int(0)) => Err(DgmError::divide_by_zero("division by zero")),
                (_, DgmValue::Float(f)) if *f == 0.0 => {
                    Err(DgmError::divide_by_zero("division by zero"))
                }
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a / b)),
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a / b)),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(*a as f64 / b)),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a / *b as f64)),
                _ => Err(DgmError::runtime("'/' type mismatch")),
            },
            "%" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => {
                    if b == 0 {
                        Err(DgmError::divide_by_zero("modulo by zero"))
                    } else {
                        Ok(DgmValue::Int(a % b))
                    }
                }
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a % b)),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a as f64 % b)),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a % b as f64)),
                _ => Err(DgmError::runtime("'%' type mismatch")),
            },
            "**" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => {
                    if b >= 0 {
                        Ok(DgmValue::Int(a.wrapping_pow(b as u32)))
                    } else {
                        Ok(DgmValue::Float((a as f64).powi(b as i32)))
                    }
                }
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a.powf(b))),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float((a as f64).powf(b))),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a.powi(b as i32))),
                _ => Err(DgmError::runtime("'**' type mismatch")),
            },
            "&" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a & b)),
                _ => Err(DgmError::runtime("'&' requires ints")),
            },
            "|" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a | b)),
                _ => Err(DgmError::runtime("'|' requires ints")),
            },
            "^" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a ^ b)),
                _ => Err(DgmError::runtime("'^' requires ints")),
            },
            "<<" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a << b)),
                _ => Err(DgmError::runtime("'<<' requires ints")),
            },
            ">>" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a >> b)),
                _ => Err(DgmError::runtime("'>>' requires ints")),
            },
            "==" => Ok(DgmValue::Bool(dgm_eq(&left, &right))),
            "!=" => Ok(DgmValue::Bool(!dgm_eq(&left, &right))),
            "<" => cmp_op!(left, right, <, "<"),
            ">" => cmp_op!(left, right, >, ">"),
            "<=" => cmp_op!(left, right, <=, "<="),
            ">=" => cmp_op!(left, right, >=, ">="),
            "and" => Ok(DgmValue::Bool(self.is_truthy(&left) && self.is_truthy(&right))),
            "or" => Ok(DgmValue::Bool(self.is_truthy(&left) || self.is_truthy(&right))),
            "in" => match right {
                DgmValue::List(l) => Ok(DgmValue::Bool(l.borrow().iter().any(|v| dgm_eq(&left, v)))),
                DgmValue::Map(m) => {
                    if let DgmValue::Str(k) = &left {
                        Ok(DgmValue::Bool(m.borrow().contains_key(k)))
                    } else {
                        Ok(DgmValue::Bool(false))
                    }
                }
                DgmValue::Str(s) => {
                    if let DgmValue::Str(sub) = &left {
                        Ok(DgmValue::Bool(s.contains(sub.as_str())))
                    } else {
                        Ok(DgmValue::Bool(false))
                    }
                }
                _ => Err(DgmError::runtime("'in' requires list/map/string")),
            },
            _ => Err(DgmError::runtime(format!("unknown op '{}'", op))),
        }
    }

    fn do_import(
        &mut self,
        name: &str,
        alias: Option<&str>,
        env: Rc<RefCell<Environment>>,
    ) -> Result<(), DgmError> {
        let binding_name = alias.unwrap_or_else(|| name.trim_end_matches(".dgm"));
        let stdlib_key = format!("@stdlib:{name}");
        if crate::stdlib::load_module(name).is_some() {
            match self.modules.get(&stdlib_key) {
                Some(ModuleState::Loading) => {
                    return Err(DgmError::circular_import(format!(
                        "circular import detected for '{}'",
                        name
                    )))
                }
                Some(ModuleState::Loaded(module)) => {
                    env.borrow_mut().set(binding_name, module.clone());
                    return Ok(());
                }
                Some(ModuleState::Failed(err)) => return Err(err.clone()),
                None => {}
            }

            let module = crate::stdlib::load_module(name).unwrap();
            self.modules
                .insert(stdlib_key, ModuleState::Loaded(module.clone()));
            env.borrow_mut().set(binding_name, module);
            return Ok(());
        }

        let path = self.resolve_import_path(name);
        let module_key = path.to_string_lossy().to_string();
        match self.modules.get(&module_key) {
            Some(ModuleState::Loading) => {
                return Err(DgmError::circular_import(format!(
                    "circular import detected for '{}'",
                    name
                )))
            }
            Some(ModuleState::Loaded(module)) => {
                env.borrow_mut().set(binding_name, module.clone());
                return Ok(());
            }
            Some(ModuleState::Failed(err)) => return Err(err.clone()),
            None => {}
        }

        self.modules
            .insert(module_key.clone(), ModuleState::Loading);
        let module_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&self.globals))));
        let module_source = Arc::new(module_key.clone());
        let module_span = Span::new(module_source.clone(), 1, 1);

        let result = self.with_source(module_source.clone(), |interp| {
            interp.with_frame(
                format!("<module {}>", binding_name),
                module_span.clone(),
                |interp| {
                    let source = std::fs::read_to_string(&path).map_err(|e| {
                        DgmError::import_fail(format!("cannot import '{}': {}", name, e))
                    })?;
                    let mut lexer = Lexer::with_file(&source, module_source.clone());
                    let tokens = lexer.tokenize()?;
                    let mut parser = Parser::new(tokens);
                    let stmts = parser.parse()?;

                    for stmt in &stmts {
                        match interp.exec_stmt(stmt, Rc::clone(&module_env))? {
                            ControlFlow::None => {}
                            ControlFlow::Return(_) | ControlFlow::Break | ControlFlow::Continue => break,
                        }
                    }

                    let mut exports = HashMap::new();
                    for key in module_env.borrow().keys() {
                        if let Some(val) = module_env.borrow().get(&key) {
                            exports.insert(key, val);
                        }
                    }
                    Ok(DgmValue::Map(Rc::new(RefCell::new(exports))))
                },
            )
        });

        match result {
            Ok(module) => {
                self.modules
                    .insert(module_key, ModuleState::Loaded(module.clone()));
                env.borrow_mut().set(binding_name, module);
                Ok(())
            }
            Err(err) => {
                self.modules
                    .insert(module_key, ModuleState::Failed(err.clone()));
                Err(err)
            }
        }
    }

    fn resolve_import_path(&self, name: &str) -> PathBuf {
        let raw = if name.ends_with(".dgm") {
            PathBuf::from(name)
        } else {
            PathBuf::from(format!("{}.dgm", name))
        };

        let resolved = if raw.is_absolute() {
            raw
        } else if !self.current_source.starts_with('<') {
            Path::new(self.current_source.as_ref())
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(raw)
        } else {
            raw
        };

        resolved.canonicalize().unwrap_or(resolved)
    }

    fn context_error(&self, err: DgmError, span: &Span) -> DgmError {
        err.with_fallback_span(span.clone())
            .with_fallback_stack(&self.call_stack)
    }

    fn runtime_error(
        &self,
        code: ErrorCode,
        message: impl Into<String>,
        span: &Span,
    ) -> DgmError {
        self.context_error(DgmError::runtime_code(code, message), span)
    }

    fn with_frame<T, F>(
        &mut self,
        function: impl Into<String>,
        span: Span,
        f: F,
    ) -> Result<T, DgmError>
    where
        F: FnOnce(&mut Self) -> Result<T, DgmError>,
    {
        self.call_stack.push(StackFrame {
            function: function.into(),
            span,
        });
        let stack_snapshot = self.call_stack.clone();
        let result = f(self).map_err(|err| err.with_fallback_stack(&stack_snapshot));
        self.call_stack.pop();
        result
    }

    fn with_source<T, F>(&mut self, source: Arc<String>, f: F) -> Result<T, DgmError>
    where
        F: FnOnce(&mut Self) -> Result<T, DgmError>,
    {
        let previous = std::mem::replace(&mut self.current_source, source);
        let result = f(self);
        self.current_source = previous;
        result
    }
}

fn dgm_eq(a: &DgmValue, b: &DgmValue) -> bool {
    match (a, b) {
        (DgmValue::Int(x), DgmValue::Int(y)) => x == y,
        (DgmValue::Float(x), DgmValue::Float(y)) => x == y,
        (DgmValue::Int(x), DgmValue::Float(y)) => (*x as f64) == *y,
        (DgmValue::Float(x), DgmValue::Int(y)) => *x == (*y as f64),
        (DgmValue::Str(x), DgmValue::Str(y)) => x == y,
        (DgmValue::Bool(x), DgmValue::Bool(y)) => x == y,
        (DgmValue::Null, DgmValue::Null) => true,
        _ => false,
    }
}

fn native_len(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => Ok(DgmValue::Int(l.borrow().len() as i64)),
        Some(DgmValue::Str(s)) => Ok(DgmValue::Int(s.len() as i64)),
        Some(DgmValue::Map(m)) => Ok(DgmValue::Int(m.borrow().len() as i64)),
        _ => Err(DgmError::runtime("len() requires list/string/map")),
    }
}

fn native_type(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let t = match args.first() {
        Some(DgmValue::Int(_)) => "int",
        Some(DgmValue::Float(_)) => "float",
        Some(DgmValue::Str(_)) => "str",
        Some(DgmValue::Bool(_)) => "bool",
        Some(DgmValue::Null) => "nul",
        Some(DgmValue::List(_)) => "list",
        Some(DgmValue::Map(_)) => "map",
        Some(DgmValue::Function { .. }) | Some(DgmValue::NativeFunction { .. }) => "function",
        Some(DgmValue::Instance { class_name, .. }) => return Ok(DgmValue::Str(class_name.clone())),
        None => return Err(DgmError::runtime("type() requires 1 arg")),
    };
    Ok(DgmValue::Str(t.into()))
}

fn native_str(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Str(format!(
        "{}",
        args.first().unwrap_or(&DgmValue::Null)
    )))
}

fn native_int(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Int(*n)),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Int(*f as i64)),
        Some(DgmValue::Bool(b)) => Ok(DgmValue::Int(if *b { 1 } else { 0 })),
        Some(DgmValue::Str(s)) => s
            .trim()
            .parse::<i64>()
            .map(DgmValue::Int)
            .map_err(|_| DgmError::runtime(format!("cannot convert '{}' to int", s))),
        _ => Err(DgmError::runtime("int() invalid arg")),
    }
}

fn native_float(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Float(*n as f64)),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Float(*f)),
        Some(DgmValue::Str(s)) => s
            .trim()
            .parse::<f64>()
            .map(DgmValue::Float)
            .map_err(|_| DgmError::runtime(format!("cannot convert '{}' to float", s))),
        _ => Err(DgmError::runtime("float() invalid arg")),
    }
}

fn native_push(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(v)) => {
            l.borrow_mut().push(v.clone());
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::runtime("push(list, value) required")),
    }
}

fn native_pop(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => l
            .borrow_mut()
            .pop()
            .ok_or_else(|| DgmError::runtime("pop() on empty list")),
        _ => Err(DgmError::runtime("pop() requires a list")),
    }
}

fn native_range(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let (start, end, step) = match args.len() {
        1 => match &args[0] {
            DgmValue::Int(n) => (0, *n, 1),
            _ => return Err(DgmError::runtime("range() requires int")),
        },
        2 => match (&args[0], &args[1]) {
            (DgmValue::Int(a), DgmValue::Int(b)) => (*a, *b, 1),
            _ => return Err(DgmError::runtime("range() requires ints")),
        },
        3 => match (&args[0], &args[1], &args[2]) {
            (DgmValue::Int(a), DgmValue::Int(b), DgmValue::Int(c)) => (*a, *b, *c),
            _ => return Err(DgmError::runtime("range() requires ints")),
        },
        _ => {
            return Err(DgmError::runtime(
                "range(end) or range(start, end) or range(start, end, step)",
            ))
        }
    };
    if step == 0 {
        return Err(DgmError::runtime("range() step cannot be 0"));
    }
    let mut list = vec![];
    let mut i = start;
    if step > 0 {
        while i < end {
            list.push(DgmValue::Int(i));
            i += step;
        }
    } else {
        while i > end {
            list.push(DgmValue::Int(i));
            i += step;
        }
    }
    Ok(DgmValue::List(Rc::new(RefCell::new(list))))
}

fn native_input(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if let Some(DgmValue::Str(prompt)) = args.first() {
        print!("{}", prompt);
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| DgmError::runtime(format!("input error: {}", e)))?;
    Ok(DgmValue::Str(
        line.trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string(),
    ))
}

fn native_abs(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Int(n.abs())),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Float(f.abs())),
        _ => Err(DgmError::runtime("abs() requires number")),
    }
}

fn native_min(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.len() == 1 {
        if let DgmValue::List(l) = &args[0] {
            let b = l.borrow();
            return b
                .iter()
                .cloned()
                .reduce(|a, b| if cmp_lt(&a, &b) { a } else { b })
                .ok_or_else(|| DgmError::runtime("min() empty list"));
        }
    }
    args.into_iter()
        .reduce(|a, b| if cmp_lt(&a, &b) { a } else { b })
        .ok_or_else(|| DgmError::runtime("min() requires args"))
}

fn native_max(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.len() == 1 {
        if let DgmValue::List(l) = &args[0] {
            let b = l.borrow();
            return b
                .iter()
                .cloned()
                .reduce(|a, b| if cmp_lt(&a, &b) { b } else { a })
                .ok_or_else(|| DgmError::runtime("max() empty list"));
        }
    }
    args.into_iter()
        .reduce(|a, b| if cmp_lt(&a, &b) { b } else { a })
        .ok_or_else(|| DgmError::runtime("max() requires args"))
}

fn cmp_lt(a: &DgmValue, b: &DgmValue) -> bool {
    match (a, b) {
        (DgmValue::Int(x), DgmValue::Int(y)) => x < y,
        (DgmValue::Float(x), DgmValue::Float(y)) => x < y,
        (DgmValue::Int(x), DgmValue::Float(y)) => (*x as f64) < *y,
        (DgmValue::Float(x), DgmValue::Int(y)) => *x < (*y as f64),
        _ => false,
    }
}

fn native_sort(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => {
            let mut v = l.borrow().clone();
            v.sort_by(|a, b| {
                if cmp_lt(a, b) {
                    std::cmp::Ordering::Less
                } else if dgm_eq(a, b) {
                    std::cmp::Ordering::Equal
                } else {
                    std::cmp::Ordering::Greater
                }
            });
            Ok(DgmValue::List(Rc::new(RefCell::new(v))))
        }
        _ => Err(DgmError::runtime("sort() requires list")),
    }
}

fn native_reverse(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => {
            let mut v = l.borrow().clone();
            v.reverse();
            Ok(DgmValue::List(Rc::new(RefCell::new(v))))
        }
        Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.chars().rev().collect())),
        _ => Err(DgmError::runtime("reverse() requires list or string")),
    }
}

fn native_keys(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Map(m)) => Ok(DgmValue::List(Rc::new(RefCell::new(
            m.borrow().keys().map(|k| DgmValue::Str(k.clone())).collect(),
        )))),
        _ => Err(DgmError::runtime("keys() requires map")),
    }
}

fn native_values(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Map(m)) => Ok(DgmValue::List(Rc::new(RefCell::new(
            m.borrow().values().cloned().collect(),
        )))),
        _ => Err(DgmError::runtime("values() requires map")),
    }
}

fn native_has_key(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Map(m)), Some(DgmValue::Str(k))) => {
            Ok(DgmValue::Bool(m.borrow().contains_key(k)))
        }
        _ => Err(DgmError::runtime("has_key(map, key) required")),
    }
}

fn native_slice(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::List(l)), Some(DgmValue::Int(start)), end) => {
            let list = l.borrow();
            let s = *start as usize;
            let e = end
                .and_then(|v| {
                    if let DgmValue::Int(n) = v {
                        Some(*n as usize)
                    } else {
                        None
                    }
                })
                .unwrap_or(list.len());
            Ok(DgmValue::List(Rc::new(RefCell::new(
                list[s..e.min(list.len())].to_vec(),
            ))))
        }
        (Some(DgmValue::Str(s)), Some(DgmValue::Int(start)), end) => {
            let st = *start as usize;
            let e = end
                .and_then(|v| {
                    if let DgmValue::Int(n) = v {
                        Some(*n as usize)
                    } else {
                        None
                    }
                })
                .unwrap_or(s.len());
            Ok(DgmValue::Str(
                s.chars().skip(st).take(e.saturating_sub(st)).collect(),
            ))
        }
        _ => Err(DgmError::runtime("slice(list/str, start, end?) required")),
    }
}

fn native_join(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(DgmValue::Str(sep))) => {
            let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect();
            Ok(DgmValue::Str(items.join(sep)))
        }
        (Some(DgmValue::List(l)), None) => {
            let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect();
            Ok(DgmValue::Str(items.join("")))
        }
        _ => Err(DgmError::runtime("join(list, sep?) required")),
    }
}

fn native_split(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sep))) => Ok(DgmValue::List(Rc::new(
            RefCell::new(
                s.split(sep.as_str())
                    .map(|p| DgmValue::Str(p.to_string()))
                    .collect(),
            ),
        ))),
        _ => Err(DgmError::runtime("split(str, sep) required")),
    }
}

fn native_replace(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(from)), Some(DgmValue::Str(to))) => {
            Ok(DgmValue::Str(s.replace(from.as_str(), to)))
        }
        _ => Err(DgmError::runtime("replace(str, from, to) required")),
    }
}

fn native_upper(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.to_uppercase())),
        _ => Err(DgmError::runtime("upper() requires string")),
    }
}

fn native_lower(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.to_lowercase())),
        _ => Err(DgmError::runtime("lower() requires string")),
    }
}

fn native_trim(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.trim().to_string())),
        _ => Err(DgmError::runtime("trim() requires string")),
    }
}

fn native_contains(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sub))) => {
            Ok(DgmValue::Bool(s.contains(sub.as_str())))
        }
        (Some(DgmValue::List(l)), Some(v)) => {
            Ok(DgmValue::Bool(l.borrow().iter().any(|x| dgm_eq(x, v))))
        }
        _ => Err(DgmError::runtime("contains(str/list, val) required")),
    }
}

fn native_starts_with(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(p))) => {
            Ok(DgmValue::Bool(s.starts_with(p.as_str())))
        }
        _ => Err(DgmError::runtime("starts_with(str, prefix) required")),
    }
}

fn native_ends_with(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(p))) => {
            Ok(DgmValue::Bool(s.ends_with(p.as_str())))
        }
        _ => Err(DgmError::runtime("ends_with(str, suffix) required")),
    }
}

fn native_chars(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Str(s)) => Ok(DgmValue::List(Rc::new(RefCell::new(
            s.chars().map(|c| DgmValue::Str(c.to_string())).collect(),
        )))),
        _ => Err(DgmError::runtime("chars() requires string")),
    }
}

fn native_format(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.is_empty() {
        return Err(DgmError::runtime("format() requires args"));
    }
    let template = match &args[0] {
        DgmValue::Str(s) => s.clone(),
        _ => return Err(DgmError::runtime("format() first arg must be string")),
    };
    let mut result = template;
    for arg in &args[1..] {
        result = result.replacen("{}", &format!("{}", arg), 1);
    }
    Ok(DgmValue::Str(result))
}

fn native_map_fn(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            let mut results = vec![];
            for item in items {
                let r = interp.call_value(func.clone(), vec![item], span, None, None)?;
                results.push(r);
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(results))))
        }
        _ => Err(DgmError::runtime("map(list, fn) required")),
    }
}

fn native_filter(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            let mut results = vec![];
            for item in items {
                let r = interp.call_value(func.clone(), vec![item.clone()], span, None, None)?;
                if matches!(r, DgmValue::Bool(true)) {
                    results.push(item);
                }
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(results))))
        }
        _ => Err(DgmError::runtime("filter(list, fn) required")),
    }
}

fn native_reduce(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::List(l)), Some(init), Some(func)) => {
            let items = l.borrow().clone();
            let mut acc = init.clone();
            for item in items {
                acc = interp.call_value(func.clone(), vec![acc, item], span, None, None)?;
            }
            Ok(acc)
        }
        _ => Err(DgmError::runtime("reduce(list, init, fn) required")),
    }
}

fn native_each(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            for item in items {
                interp.call_value(func.clone(), vec![item], span, None, None)?;
            }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::runtime("each(list, fn) required")),
    }
}

fn native_find(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            for item in items {
                let r = interp.call_value(func.clone(), vec![item.clone()], span, None, None)?;
                if matches!(r, DgmValue::Bool(true)) {
                    return Ok(item);
                }
            }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::runtime("find(list, fn) required")),
    }
}

fn native_index_of(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(val)) => {
            for (i, item) in l.borrow().iter().enumerate() {
                if dgm_eq(item, val) {
                    return Ok(DgmValue::Int(i as i64));
                }
            }
            Ok(DgmValue::Int(-1))
        }
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sub))) => {
            Ok(DgmValue::Int(s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1)))
        }
        _ => Err(DgmError::runtime("index_of(list/str, val) required")),
    }
}

fn native_flat(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => {
            let mut result = vec![];
            for item in l.borrow().iter() {
                if let DgmValue::List(inner) = item {
                    result.extend(inner.borrow().clone());
                } else {
                    result.push(item.clone());
                }
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(result))))
        }
        _ => Err(DgmError::runtime("flat() requires list")),
    }
}

fn native_zip(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(a)), Some(DgmValue::List(b))) => {
            let a = a.borrow();
            let b = b.borrow();
            let result: Vec<DgmValue> = a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| {
                    DgmValue::List(Rc::new(RefCell::new(vec![x.clone(), y.clone()])))
                })
                .collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(result))))
        }
        _ => Err(DgmError::runtime("zip(list, list) required")),
    }
}

fn native_sum(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => {
            let mut total = DgmValue::Int(0);
            for item in l.borrow().iter() {
                total = match (&total, item) {
                    (DgmValue::Int(a), DgmValue::Int(b)) => DgmValue::Int(a + b),
                    (DgmValue::Int(a), DgmValue::Float(b)) => DgmValue::Float(*a as f64 + b),
                    (DgmValue::Float(a), DgmValue::Int(b)) => DgmValue::Float(a + *b as f64),
                    (DgmValue::Float(a), DgmValue::Float(b)) => DgmValue::Float(a + b),
                    _ => return Err(DgmError::runtime("sum() requires list of numbers")),
                };
            }
            Ok(total)
        }
        _ => Err(DgmError::runtime("sum() requires list")),
    }
}

fn native_any(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            for item in items {
                let r = interp.call_value(func.clone(), vec![item], span, None, None)?;
                if matches!(r, DgmValue::Bool(true)) {
                    return Ok(DgmValue::Bool(true));
                }
            }
            Ok(DgmValue::Bool(false))
        }
        (Some(DgmValue::List(l)), None) => Ok(DgmValue::Bool(
            l.borrow()
                .iter()
                .any(|v| !matches!(v, DgmValue::Bool(false) | DgmValue::Null)),
        )),
        _ => Err(DgmError::runtime("any(list, fn?) required")),
    }
}

fn native_all(
    interp: &mut Interpreter,
    args: Vec<DgmValue>,
    span: &Span,
) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let items = l.borrow().clone();
            for item in items {
                let r = interp.call_value(func.clone(), vec![item], span, None, None)?;
                if !matches!(r, DgmValue::Bool(true)) {
                    return Ok(DgmValue::Bool(false));
                }
            }
            Ok(DgmValue::Bool(true))
        }
        (Some(DgmValue::List(l)), None) => Ok(DgmValue::Bool(
            l.borrow()
                .iter()
                .all(|v| !matches!(v, DgmValue::Bool(false) | DgmValue::Null)),
        )),
        _ => Err(DgmError::runtime("all(list, fn?) required")),
    }
}

fn native_print(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let parts: Vec<String> = args.iter().map(|v| format!("{}", v)).collect();
    print!("{}", parts.join(" "));
    use std::io::Write;
    std::io::stdout().flush().ok();
    Ok(DgmValue::Null)
}

fn native_println(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let parts: Vec<String> = args.iter().map(|v| format!("{}", v)).collect();
    println!("{}", parts.join(" "));
    Ok(DgmValue::Null)
}

fn native_chr(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Str(
            char::from_u32(*n as u32).unwrap_or('\0').to_string(),
        )),
        _ => Err(DgmError::runtime("chr() requires int")),
    }
}

fn native_ord(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Str(s)) => s
            .chars()
            .next()
            .map(|c| DgmValue::Int(c as i64))
            .ok_or_else(|| DgmError::runtime("ord() empty string")),
        _ => Err(DgmError::runtime("ord() requires string")),
    }
}

fn native_hex(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Str(format!("0x{:x}", n))),
        _ => Err(DgmError::runtime("hex() requires int")),
    }
}

fn native_bin(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Str(format!("0b{:b}", n))),
        _ => Err(DgmError::runtime("bin() requires int")),
    }
}

fn native_exit(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let code = match args.first() {
        Some(DgmValue::Int(n)) => *n as i32,
        _ => 0,
    };
    std::process::exit(code);
}

fn native_assert(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let condition = args.first().unwrap_or(&DgmValue::Null);
    let is_truthy = match condition {
        DgmValue::Bool(b) => *b,
        DgmValue::Null => false,
        DgmValue::Int(n) => *n != 0,
        DgmValue::Float(f) => *f != 0.0,
        DgmValue::Str(s) => !s.is_empty(),
        _ => true,
    };
    if !is_truthy {
        let msg = match args.get(1) {
            Some(DgmValue::Str(s)) => s.clone(),
            _ => "assertion failed".to_string(),
        };
        return Err(DgmError::runtime(msg));
    }
    Ok(DgmValue::Bool(true))
}
