use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use crate::ast::{Expr, Stmt};
use crate::environment::Environment;
use crate::error::DgmError;

macro_rules! numeric_op {
    ($left:expr, $right:expr, $int_op:ident, $float_op:tt, $name:expr) => {
        match ($left, $right) {
            (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a.$int_op(b))),
            (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a $float_op b)),
            (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a as f64 $float_op b)),
            (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a $float_op b as f64)),
            _ => Err(DgmError::RuntimeError { msg: format!("'{}' type mismatch", $name) }),
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
            _ => Err(DgmError::RuntimeError { msg: format!("'{}' type mismatch", $name) }),
        }
    };
}

#[derive(Debug, Clone)]
pub enum DgmValue {
    Int(i64), Float(f64), Str(String), Bool(bool), Null,
    List(Rc<RefCell<Vec<DgmValue>>>),
    Map(Rc<RefCell<HashMap<String, DgmValue>>>),
    Function { params: Vec<String>, body: Vec<Stmt>, closure: Rc<RefCell<Environment>> },
    NativeFunction { name: String, func: fn(Vec<DgmValue>) -> Result<DgmValue, DgmError> },
    Instance { class_name: String, fields: Rc<RefCell<HashMap<String, DgmValue>>> },
}

impl fmt::Display for DgmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DgmValue::Int(n) => write!(f, "{}", n),
            DgmValue::Float(n) => write!(f, "{}", n),
            DgmValue::Str(s) => write!(f, "{}", s),
            DgmValue::Bool(b) => write!(f, "{}", if *b { "tru" } else { "fals" }),
            DgmValue::Null => write!(f, "nul"),
            DgmValue::List(l) => { let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect(); write!(f, "[{}]", items.join(", ")) }
            DgmValue::Map(m) => { let pairs: Vec<String> = m.borrow().iter().map(|(k, v)| format!("{}: {}", k, v)).collect(); write!(f, "{{{}}}", pairs.join(", ")) }
            DgmValue::Function { params, .. } => write!(f, "<fn({})>", params.join(", ")),
            DgmValue::NativeFunction { name, .. } => write!(f, "<native {}>", name),
            DgmValue::Instance { class_name, .. } => write!(f, "<{} instance>", class_name),
        }
    }
}

pub enum ControlFlow { None, Return(DgmValue), Break, Continue }

pub struct Interpreter {
    pub globals: Rc<RefCell<Environment>>,
    classes: HashMap<String, ClassDef>,
    imported_modules: HashMap<String, bool>,
}

#[derive(Clone)]
struct ClassDef {
    methods: Vec<Stmt>,
    parent: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(Environment::new()));
        let natives: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
            ("len", native_len), ("type", native_type), ("str", native_str),
            ("int", native_int), ("float", native_float), ("push", native_push),
            ("pop", native_pop), ("range", native_range), ("input", native_input),
            ("abs", native_abs), ("min", native_min), ("max", native_max),
            ("sort", native_sort), ("reverse", native_reverse), ("keys", native_keys),
            ("values", native_values), ("has_key", native_has_key),
            ("slice", native_slice), ("join", native_join), ("split", native_split),
            ("replace", native_replace), ("upper", native_upper), ("lower", native_lower),
            ("trim", native_trim), ("contains", native_contains),
            ("starts_with", native_starts_with), ("ends_with", native_ends_with),
            ("chars", native_chars), ("format", native_format),
            ("map", native_map_fn), ("filter", native_filter),
            ("reduce", native_reduce), ("each", native_each),
            ("find", native_find), ("index_of", native_index_of),
            ("flat", native_flat), ("zip", native_zip),
            ("sum", native_sum), ("any", native_any), ("all", native_all),
            ("print", native_print), ("println", native_println),
            ("chr", native_chr), ("ord", native_ord),
            ("hex", native_hex), ("bin", native_bin),
            ("exit", native_exit),
        ];
        for (name, func) in natives {
            globals.borrow_mut().set(name, DgmValue::NativeFunction { name: name.to_string(), func: *func });
        }
        Self { globals, classes: HashMap::new(), imported_modules: HashMap::new() }
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

    pub fn exec_stmt(&mut self, stmt: &Stmt, env: Rc<RefCell<Environment>>) -> Result<ControlFlow, DgmError> {
        match stmt {
            Stmt::Expr(expr) => { self.eval_expr(expr, env)?; Ok(ControlFlow::None) }
            Stmt::Let { name, value } => {
                let v = self.eval_expr(value, Rc::clone(&env))?;
                env.borrow_mut().set(name, v);
                Ok(ControlFlow::None)
            }
            Stmt::Writ(expr) => { let v = self.eval_expr(expr, env)?; println!("{}", v); Ok(ControlFlow::None) }
            Stmt::If { condition, then_block, elseif_branches, else_block } => {
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
                if let Some(block) = else_block { return self.exec_block(block, Rc::clone(&env)); }
                Ok(ControlFlow::None)
            }
            Stmt::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expr(condition, Rc::clone(&env))?;
                    if !self.is_truthy(&cond_val) { break; }
                    match self.exec_block(body, Rc::clone(&env))? {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        _ => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            Stmt::For { var, iterable, body } => {
                let iter_val = self.eval_expr(iterable, Rc::clone(&env))?;
                let items = match iter_val {
                    DgmValue::List(l) => l.borrow().clone(),
                    DgmValue::Str(s) => s.chars().map(|c| DgmValue::Str(c.to_string())).collect(),
                    DgmValue::Map(m) => m.borrow().keys().map(|k| DgmValue::Str(k.clone())).collect(),
                    _ => return Err(DgmError::RuntimeError { msg: "for loop requires iterable".into() }),
                };
                for item in items {
                    let loop_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&env))));
                    loop_env.borrow_mut().set(var, item);
                    match self.exec_block(body, loop_env)? {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        _ => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            Stmt::FuncDef { name, params, body } => {
                let func = DgmValue::Function { params: params.clone(), body: body.clone(), closure: Rc::clone(&env) };
                env.borrow_mut().set(name, func);
                Ok(ControlFlow::None)
            }
            Stmt::Return(expr) => {
                let v = if let Some(e) = expr { self.eval_expr(e, env)? } else { DgmValue::Null };
                Ok(ControlFlow::Return(v))
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Continue => Ok(ControlFlow::Continue),
            Stmt::ClassDef { name, parent, methods } => {
                self.classes.insert(name.clone(), ClassDef { methods: methods.clone(), parent: parent.clone() });
                Ok(ControlFlow::None)
            }
            Stmt::TryCatch { try_block, catch_var, catch_block, finally_block } => {
                let result = self.exec_block(try_block, Rc::clone(&env));
                let cf = match result {
                    Ok(cf) => cf,
                    Err(e) => {
                        let catch_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&env))));
                        if let Some(var) = catch_var {
                            catch_env.borrow_mut().set(var, DgmValue::Str(format!("{}", e)));
                        }
                        self.exec_block(catch_block, catch_env)?
                    }
                };
                if let Some(fb) = finally_block { self.exec_block(fb, Rc::clone(&env))?; }
                Ok(cf)
            }
            Stmt::Throw(expr) => {
                let v = self.eval_expr(expr, env)?;
                Err(DgmError::ThrownError { value: format!("{}", v) })
            }
            Stmt::Match { expr, arms, default } => {
                let val = self.eval_expr(expr, Rc::clone(&env))?;
                for (pattern, block) in arms {
                    let pat_val = self.eval_expr(pattern, Rc::clone(&env))?;
                    if dgm_eq(&val, &pat_val) { return self.exec_block(block, Rc::clone(&env)); }
                }
                if let Some(block) = default { return self.exec_block(block, Rc::clone(&env)); }
                Ok(ControlFlow::None)
            }
            Stmt::Imprt(name) => { self.do_import(name, env)?; Ok(ControlFlow::None) }
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt], parent_env: Rc<RefCell<Environment>>) -> Result<ControlFlow, DgmError> {
        let env = Rc::new(RefCell::new(Environment::new_child(parent_env)));
        for stmt in stmts {
            match self.exec_stmt(stmt, Rc::clone(&env))? {
                ControlFlow::None => {}
                cf => return Ok(cf),
            }
        }
        Ok(ControlFlow::None)
    }

    pub fn eval_expr(&mut self, expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<DgmValue, DgmError> {
        match expr {
            Expr::IntLit(n) => Ok(DgmValue::Int(*n)),
            Expr::FloatLit(f) => Ok(DgmValue::Float(*f)),
            Expr::StringLit(s) => Ok(DgmValue::Str(s.clone())),
            Expr::BoolLit(b) => Ok(DgmValue::Bool(*b)),
            Expr::NullLit => Ok(DgmValue::Null),
            Expr::This => env.borrow().get("__self__").ok_or_else(|| DgmError::RuntimeError { msg: "'ths' used outside class".into() }),
            Expr::Ident(name) => env.borrow().get(name).ok_or_else(|| DgmError::RuntimeError { msg: format!("undefined variable '{}'", name) }),
            Expr::BinOp { op, left, right } => {
                // Short-circuit for and/or
                if op == "and" {
                    let l = self.eval_expr(left, Rc::clone(&env))?;
                    if !self.is_truthy(&l) { return Ok(DgmValue::Bool(false)); }
                    let r = self.eval_expr(right, env)?;
                    return Ok(DgmValue::Bool(self.is_truthy(&r)));
                }
                if op == "or" {
                    let l = self.eval_expr(left, Rc::clone(&env))?;
                    if self.is_truthy(&l) { return Ok(DgmValue::Bool(true)); }
                    let r = self.eval_expr(right, env)?;
                    return Ok(DgmValue::Bool(self.is_truthy(&r)));
                }
                let l = self.eval_expr(left, Rc::clone(&env))?;
                let r = self.eval_expr(right, Rc::clone(&env))?;
                self.apply_binop(op, l, r)
            }
            Expr::UnaryOp { op, operand } => {
                let v = self.eval_expr(operand, env)?;
                match op.as_str() {
                    "not" => Ok(DgmValue::Bool(!self.is_truthy(&v))),
                    "-" => match v {
                        DgmValue::Int(n) => Ok(DgmValue::Int(-n)),
                        DgmValue::Float(f) => Ok(DgmValue::Float(-f)),
                        _ => Err(DgmError::RuntimeError { msg: "unary '-' on non-number".into() }),
                    },
                    "~" => match v {
                        DgmValue::Int(n) => Ok(DgmValue::Int(!n)),
                        _ => Err(DgmError::RuntimeError { msg: "bitwise '~' requires int".into() }),
                    },
                    _ => Err(DgmError::RuntimeError { msg: format!("unknown unary op '{}'", op) }),
                }
            }
            Expr::Assign { target, op, value } => {
                let val = self.eval_expr(value, Rc::clone(&env))?;
                self.do_assign(target, op, val, env)
            }
            Expr::Call { callee, args } => {
                if let Expr::FieldAccess { object, field } = callee.as_ref() {
                    let obj = self.eval_expr(object, Rc::clone(&env))?;
                    if let DgmValue::Instance { ref fields, .. } = obj {
                        let method = fields.borrow().get(field).cloned()
                            .ok_or_else(|| DgmError::RuntimeError { msg: format!("no method '{}'", field) })?;
                        let arg_vals: Vec<DgmValue> = args.iter().map(|a| self.eval_expr(a, Rc::clone(&env))).collect::<Result<_, _>>()?;
                        if let DgmValue::Function { params, body, closure } = method {
                            if params.len() != arg_vals.len() { return Err(DgmError::RuntimeError { msg: format!("expected {} args, got {}", params.len(), arg_vals.len()) }); }
                            let call_env = Rc::new(RefCell::new(Environment::new_child(closure)));
                            call_env.borrow_mut().set("__self__", obj.clone());
                            for (p, v) in params.iter().zip(arg_vals) { call_env.borrow_mut().set(p, v); }
                            return match self.exec_block(&body, call_env)? { ControlFlow::Return(v) => Ok(v), _ => Ok(DgmValue::Null) };
                        }
                        return Err(DgmError::RuntimeError { msg: format!("'{}' is not callable", field) });
                    }
                    // Map method calls
                    if let DgmValue::Map(ref m) = obj {
                        if let Some(func) = m.borrow().get(field).cloned() {
                            let arg_vals: Vec<DgmValue> = args.iter().map(|a| self.eval_expr(a, Rc::clone(&env))).collect::<Result<_, _>>()?;
                            return self.call_function(func, arg_vals, Rc::clone(&env));
                        }
                    }
                    return Err(DgmError::RuntimeError { msg: format!("cannot call '{}' on {:?}", field, obj) });
                }
                let callee_val = self.eval_expr(callee, Rc::clone(&env))?;
                let arg_vals: Vec<DgmValue> = args.iter().map(|a| self.eval_expr(a, Rc::clone(&env))).collect::<Result<_, _>>()?;
                self.call_function(callee_val, arg_vals, env)
            }
            Expr::FieldAccess { object, field } => {
                let obj = self.eval_expr(object, env)?;
                match &obj {
                    DgmValue::Instance { fields, .. } => fields.borrow().get(field).cloned().ok_or_else(|| DgmError::RuntimeError { msg: format!("no field '{}'", field) }),
                    DgmValue::Map(m) => m.borrow().get(field).cloned().ok_or_else(|| DgmError::RuntimeError { msg: format!("key '{}' not found", field) }),
                    DgmValue::Str(s) => match field.as_str() {
                        "length" => Ok(DgmValue::Int(s.len() as i64)),
                        _ => Err(DgmError::RuntimeError { msg: format!("no property '{}' on string", field) }),
                    },
                    DgmValue::List(l) => match field.as_str() {
                        "length" => Ok(DgmValue::Int(l.borrow().len() as i64)),
                        _ => Err(DgmError::RuntimeError { msg: format!("no property '{}' on list", field) }),
                    },
                    _ => Err(DgmError::RuntimeError { msg: format!("cannot access field '{}' on {}", field, obj) }),
                }
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                let idx = self.eval_expr(index, env)?;
                match (&obj, &idx) {
                    (DgmValue::List(l), DgmValue::Int(i)) => {
                        let list = l.borrow();
                        let i = if *i < 0 { list.len() as i64 + i } else { *i } as usize;
                        list.get(i).cloned().ok_or_else(|| DgmError::RuntimeError { msg: "list index out of range".into() })
                    }
                    (DgmValue::Map(m), DgmValue::Str(k)) => m.borrow().get(k).cloned().ok_or_else(|| DgmError::RuntimeError { msg: format!("key '{}' not found", k) }),
                    (DgmValue::Str(s), DgmValue::Int(i)) => {
                        let i = if *i < 0 { s.len() as i64 + i } else { *i } as usize;
                        s.chars().nth(i).map(|c| DgmValue::Str(c.to_string())).ok_or_else(|| DgmError::RuntimeError { msg: "string index out of range".into() })
                    }
                    _ => Err(DgmError::RuntimeError { msg: "invalid index operation".into() }),
                }
            }
            Expr::List(items) => {
                let vals: Vec<DgmValue> = items.iter().map(|e| self.eval_expr(e, Rc::clone(&env))).collect::<Result<_, _>>()?;
                Ok(DgmValue::List(Rc::new(RefCell::new(vals))))
            }
            Expr::Map(pairs) => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    let key = match self.eval_expr(k, Rc::clone(&env))? { DgmValue::Str(s) => s, other => format!("{}", other) };
                    let val = self.eval_expr(v, Rc::clone(&env))?;
                    map.insert(key, val);
                }
                Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
            }
            Expr::New { class_name, args } => self.instantiate_class(class_name, args, env),
            Expr::Lambda { params, body } => Ok(DgmValue::Function { params: params.clone(), body: body.clone(), closure: Rc::clone(&env) }),
            Expr::Ternary { condition, then_expr, else_expr } => {
                let cond = self.eval_expr(condition, Rc::clone(&env))?;
                if self.is_truthy(&cond) { self.eval_expr(then_expr, env) } else { self.eval_expr(else_expr, env) }
            }
            Expr::StringInterp(parts) => {
                let mut result = String::new();
                for part in parts { result.push_str(&format!("{}", self.eval_expr(part, Rc::clone(&env))?)); }
                Ok(DgmValue::Str(result))
            }
            Expr::Range { start, end } => {
                let s = match self.eval_expr(start, Rc::clone(&env))? { DgmValue::Int(n) => n, _ => return Err(DgmError::RuntimeError { msg: "range requires int".into() }) };
                let e = match self.eval_expr(end, env)? { DgmValue::Int(n) => n, _ => return Err(DgmError::RuntimeError { msg: "range requires int".into() }) };
                let list: Vec<DgmValue> = (s..e).map(DgmValue::Int).collect();
                Ok(DgmValue::List(Rc::new(RefCell::new(list))))
            }
        }
    }

    fn instantiate_class(&mut self, class_name: &str, args: &[Expr], env: Rc<RefCell<Environment>>) -> Result<DgmValue, DgmError> {
        let class = self.classes.get(class_name).cloned()
            .ok_or_else(|| DgmError::RuntimeError { msg: format!("undefined class '{}'", class_name) })?;
        let fields: Rc<RefCell<HashMap<String, DgmValue>>> = Rc::new(RefCell::new(HashMap::new()));
        let instance = DgmValue::Instance { class_name: class_name.to_string(), fields: Rc::clone(&fields) };
        // Collect all methods including inherited
        let all_methods = self.collect_methods(&class);
        for method in &all_methods {
            if let Stmt::FuncDef { name, params, body } = method {
                fields.borrow_mut().insert(name.clone(), DgmValue::Function { params: params.clone(), body: body.clone(), closure: Rc::clone(&env) });
            }
        }
        let init_fn = fields.borrow().get("init").cloned();
        if let Some(init_fn) = init_fn {
            let arg_vals: Vec<DgmValue> = args.iter().map(|a| self.eval_expr(a, Rc::clone(&env))).collect::<Result<_, _>>()?;
            let call_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&env))));
            call_env.borrow_mut().set("__self__", instance.clone());
            if let DgmValue::Function { params, body, .. } = init_fn {
                if params.len() != arg_vals.len() { return Err(DgmError::RuntimeError { msg: format!("init expects {} args, got {}", params.len(), arg_vals.len()) }); }
                for (p, v) in params.iter().zip(arg_vals) { call_env.borrow_mut().set(p, v); }
                for stmt in &body { self.exec_stmt(stmt, Rc::clone(&call_env))?; }
            }
        }
        Ok(instance)
    }

    fn collect_methods(&self, class: &ClassDef) -> Vec<Stmt> {
        let mut methods = vec![];
        if let Some(ref parent_name) = class.parent {
            if let Some(parent_class) = self.classes.get(parent_name) {
                methods = self.collect_methods(&parent_class.clone());
            }
        }
        // Child methods override parent
        for m in &class.methods {
            if let Stmt::FuncDef { name, .. } = m {
                methods.retain(|existing| { if let Stmt::FuncDef { name: n, .. } = existing { n != name } else { true } });
            }
            methods.push(m.clone());
        }
        methods
    }

    fn do_assign(&mut self, target: &Expr, op: &str, val: DgmValue, env: Rc<RefCell<Environment>>) -> Result<DgmValue, DgmError> {
        match target {
            Expr::Ident(name) => {
                let final_val = if op == "=" { val } else {
                    let current = env.borrow().get(name).ok_or_else(|| DgmError::RuntimeError { msg: format!("undefined '{}'", name) })?;
                    self.apply_binop(&op[..op.len()-1], current, val)?
                };
                if env.borrow().get(name).is_some() { env.borrow_mut().assign(name, final_val.clone())?; }
                else { env.borrow_mut().set(name, final_val.clone()); }
                Ok(final_val)
            }
            Expr::FieldAccess { object, field } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                match obj {
                    DgmValue::Instance { fields, .. } => {
                        let final_val = if op == "=" { val } else {
                            let current = fields.borrow().get(field).cloned().unwrap_or(DgmValue::Null);
                            self.apply_binop(&op[..op.len()-1], current, val)?
                        };
                        fields.borrow_mut().insert(field.clone(), final_val.clone());
                        Ok(final_val)
                    }
                    DgmValue::Map(m) => {
                        let final_val = if op == "=" { val } else {
                            let current = m.borrow().get(field).cloned().unwrap_or(DgmValue::Null);
                            self.apply_binop(&op[..op.len()-1], current, val)?
                        };
                        m.borrow_mut().insert(field.clone(), final_val.clone());
                        Ok(final_val)
                    }
                    _ => Err(DgmError::RuntimeError { msg: "field assign on non-instance".into() }),
                }
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object, Rc::clone(&env))?;
                let idx = self.eval_expr(index, Rc::clone(&env))?;
                match (&obj, &idx) {
                    (DgmValue::List(l), DgmValue::Int(i)) => {
                        let len = l.borrow().len();
                        let i = if *i < 0 { len as i64 + i } else { *i } as usize;
                        if i >= len { return Err(DgmError::RuntimeError { msg: "index out of range".into() }); }
                        let final_val = if op == "=" { val } else { self.apply_binop(&op[..op.len()-1], l.borrow()[i].clone(), val)? };
                        l.borrow_mut()[i] = final_val.clone();
                        Ok(final_val)
                    }
                    (DgmValue::Map(m), DgmValue::Str(k)) => { m.borrow_mut().insert(k.clone(), val.clone()); Ok(val) }
                    _ => Err(DgmError::RuntimeError { msg: "invalid index assign".into() }),
                }
            }
            _ => Err(DgmError::RuntimeError { msg: "invalid assignment target".into() }),
        }
    }

    fn call_function(&mut self, callee: DgmValue, args: Vec<DgmValue>, _env: Rc<RefCell<Environment>>) -> Result<DgmValue, DgmError> {
        match callee {
            DgmValue::NativeFunction { func, .. } => func(args),
            DgmValue::Function { params, body, closure } => {
                if params.len() != args.len() { return Err(DgmError::RuntimeError { msg: format!("expected {} args, got {}", params.len(), args.len()) }); }
                let call_env = Rc::new(RefCell::new(Environment::new_child(closure)));
                for (p, v) in params.iter().zip(args) { call_env.borrow_mut().set(p, v); }
                match self.exec_block(&body, call_env)? { ControlFlow::Return(v) => Ok(v), _ => Ok(DgmValue::Null) }
            }
            _ => Err(DgmError::RuntimeError { msg: "value is not callable".into() }),
        }
    }

    fn is_truthy(&self, value: &DgmValue) -> bool {
        match value {
            DgmValue::Bool(b) => *b, DgmValue::Null => false,
            DgmValue::Int(n) => *n != 0, DgmValue::Float(f) => *f != 0.0,
            DgmValue::Str(s) => !s.is_empty(), _ => true,
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
                (DgmValue::List(a), DgmValue::List(b)) => { let mut v = a.borrow().clone(); v.extend(b.borrow().clone()); Ok(DgmValue::List(Rc::new(RefCell::new(v)))) }
                _ => Err(DgmError::RuntimeError { msg: "'+' type mismatch".into() }),
            },
            "-" => numeric_op!(left, right, wrapping_sub, -,  "-"),
            "*" => numeric_op!(left, right, wrapping_mul, *, "*"),
            "/" => {
                match (&left, &right) {
                    (_, DgmValue::Int(0)) => Err(DgmError::RuntimeError { msg: "division by zero".into() }),
                    (_, DgmValue::Float(f)) if *f == 0.0 => Err(DgmError::RuntimeError { msg: "division by zero".into() }),
                    (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a / b)),
                    (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a / b)),
                    (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(*a as f64 / b)),
                    (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a / *b as f64)),
                    _ => Err(DgmError::RuntimeError { msg: "'/' type mismatch".into() }),
                }
            }
            "%" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => { if b == 0 { Err(DgmError::RuntimeError { msg: "modulo by zero".into() }) } else { Ok(DgmValue::Int(a % b)) } }
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a % b)),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a as f64 % b)),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a % b as f64)),
                _ => Err(DgmError::RuntimeError { msg: "'%' type mismatch".into() }),
            },
            "**" => match (left, right) {
                (DgmValue::Int(a), DgmValue::Int(b)) => { if b >= 0 { Ok(DgmValue::Int(a.wrapping_pow(b as u32))) } else { Ok(DgmValue::Float((a as f64).powi(b as i32))) } }
                (DgmValue::Float(a), DgmValue::Float(b)) => Ok(DgmValue::Float(a.powf(b))),
                (DgmValue::Int(a), DgmValue::Float(b)) => Ok(DgmValue::Float((a as f64).powf(b))),
                (DgmValue::Float(a), DgmValue::Int(b)) => Ok(DgmValue::Float(a.powi(b as i32))),
                _ => Err(DgmError::RuntimeError { msg: "'**' type mismatch".into() }),
            },
            "&" => match (left, right) { (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a & b)), _ => Err(DgmError::RuntimeError { msg: "'&' requires ints".into() }) },
            "|" => match (left, right) { (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a | b)), _ => Err(DgmError::RuntimeError { msg: "'|' requires ints".into() }) },
            "^" => match (left, right) { (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a ^ b)), _ => Err(DgmError::RuntimeError { msg: "'^' requires ints".into() }) },
            "<<" => match (left, right) { (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a << b)), _ => Err(DgmError::RuntimeError { msg: "'<<' requires ints".into() }) },
            ">>" => match (left, right) { (DgmValue::Int(a), DgmValue::Int(b)) => Ok(DgmValue::Int(a >> b)), _ => Err(DgmError::RuntimeError { msg: "'>>' requires ints".into() }) },
            "==" => Ok(DgmValue::Bool(dgm_eq(&left, &right))),
            "!=" => Ok(DgmValue::Bool(!dgm_eq(&left, &right))),
            "<" => cmp_op!(left, right, <, "<"),
            ">" => cmp_op!(left, right, >, ">"),
            "<=" => cmp_op!(left, right, <=, "<="),
            ">=" => cmp_op!(left, right, >=, ">="),
            "and" => Ok(DgmValue::Bool(self.is_truthy(&left) && self.is_truthy(&right))),
            "or" => Ok(DgmValue::Bool(self.is_truthy(&left) || self.is_truthy(&right))),
            "in" => {
                match right {
                    DgmValue::List(l) => Ok(DgmValue::Bool(l.borrow().iter().any(|v| dgm_eq(&left, v)))),
                    DgmValue::Map(m) => { if let DgmValue::Str(k) = &left { Ok(DgmValue::Bool(m.borrow().contains_key(k))) } else { Ok(DgmValue::Bool(false)) } }
                    DgmValue::Str(s) => { if let DgmValue::Str(sub) = &left { Ok(DgmValue::Bool(s.contains(sub.as_str()))) } else { Ok(DgmValue::Bool(false)) } }
                    _ => Err(DgmError::RuntimeError { msg: "'in' requires list/map/string".into() }),
                }
            }
            _ => Err(DgmError::RuntimeError { msg: format!("unknown op '{}'", op) }),
        }
    }

    fn do_import(&mut self, name: &str, env: Rc<RefCell<Environment>>) -> Result<(), DgmError> {
        if self.imported_modules.contains_key(name) { return Ok(()); }
        self.imported_modules.insert(name.to_string(), true);
        // Try stdlib first
        if let Some(module) = crate::stdlib::load_module(name) {
            env.borrow_mut().set(name, module);
            return Ok(());
        }
        // Try file import
        let path = if name.ends_with(".dgm") { name.to_string() } else { format!("{}.dgm", name) };
        let source = std::fs::read_to_string(&path).map_err(|e| DgmError::ImportError { msg: format!("cannot import '{}': {}", name, e) })?;
        let mut lexer = crate::lexer::Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        let mut parser = crate::parser::Parser::new(tokens);
        let stmts = parser.parse()?;
        let module_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(&self.globals))));
        for stmt in &stmts {
            self.exec_stmt(stmt, Rc::clone(&module_env))?;
        }
        // Export all module-level bindings as a map
        let mut exports = HashMap::new();
        for key in module_env.borrow().keys() {
            if let Some(val) = module_env.borrow().get(&key) { exports.insert(key, val); }
        }
        env.borrow_mut().set(name.trim_end_matches(".dgm"), DgmValue::Map(Rc::new(RefCell::new(exports))));
        Ok(())
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

// ─── Native Functions ────────────────────────────────────────
fn native_len(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => Ok(DgmValue::Int(l.borrow().len() as i64)),
        Some(DgmValue::Str(s)) => Ok(DgmValue::Int(s.len() as i64)),
        Some(DgmValue::Map(m)) => Ok(DgmValue::Int(m.borrow().len() as i64)),
        _ => Err(DgmError::RuntimeError { msg: "len() requires list/string/map".into() }),
    }
}
fn native_type(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let t = match args.first() {
        Some(DgmValue::Int(_)) => "int", Some(DgmValue::Float(_)) => "float",
        Some(DgmValue::Str(_)) => "str", Some(DgmValue::Bool(_)) => "bool",
        Some(DgmValue::Null) => "nul", Some(DgmValue::List(_)) => "list",
        Some(DgmValue::Map(_)) => "map",
        Some(DgmValue::Function { .. }) | Some(DgmValue::NativeFunction { .. }) => "function",
        Some(DgmValue::Instance { class_name, .. }) => return Ok(DgmValue::Str(class_name.clone())),
        None => return Err(DgmError::RuntimeError { msg: "type() requires 1 arg".into() }),
    };
    Ok(DgmValue::Str(t.into()))
}
fn native_str(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Str(format!("{}", args.first().unwrap_or(&DgmValue::Null)))) }
fn native_int(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Int(*n)),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Int(*f as i64)),
        Some(DgmValue::Bool(b)) => Ok(DgmValue::Int(if *b { 1 } else { 0 })),
        Some(DgmValue::Str(s)) => s.trim().parse::<i64>().map(DgmValue::Int).map_err(|_| DgmError::RuntimeError { msg: format!("cannot convert '{}' to int", s) }),
        _ => Err(DgmError::RuntimeError { msg: "int() invalid arg".into() }),
    }
}
fn native_float(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Float(*n as f64)),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Float(*f)),
        Some(DgmValue::Str(s)) => s.trim().parse::<f64>().map(DgmValue::Float).map_err(|_| DgmError::RuntimeError { msg: format!("cannot convert '{}' to float", s) }),
        _ => Err(DgmError::RuntimeError { msg: "float() invalid arg".into() }),
    }
}
fn native_push(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(v)) => { l.borrow_mut().push(v.clone()); Ok(DgmValue::Null) }
        _ => Err(DgmError::RuntimeError { msg: "push(list, value) required".into() }),
    }
}
fn native_pop(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => l.borrow_mut().pop().ok_or_else(|| DgmError::RuntimeError { msg: "pop() on empty list".into() }),
        _ => Err(DgmError::RuntimeError { msg: "pop() requires a list".into() }),
    }
}
fn native_range(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let (start, end, step) = match args.len() {
        1 => match &args[0] { DgmValue::Int(n) => (0, *n, 1), _ => return Err(DgmError::RuntimeError { msg: "range() requires int".into() }) },
        2 => match (&args[0], &args[1]) { (DgmValue::Int(a), DgmValue::Int(b)) => (*a, *b, 1), _ => return Err(DgmError::RuntimeError { msg: "range() requires ints".into() }) },
        3 => match (&args[0], &args[1], &args[2]) { (DgmValue::Int(a), DgmValue::Int(b), DgmValue::Int(c)) => (*a, *b, *c), _ => return Err(DgmError::RuntimeError { msg: "range() requires ints".into() }) },
        _ => return Err(DgmError::RuntimeError { msg: "range(end) or range(start, end) or range(start, end, step)".into() }),
    };
    if step == 0 { return Err(DgmError::RuntimeError { msg: "range() step cannot be 0".into() }); }
    let mut list = vec![];
    let mut i = start;
    if step > 0 { while i < end { list.push(DgmValue::Int(i)); i += step; } }
    else { while i > end { list.push(DgmValue::Int(i)); i += step; } }
    Ok(DgmValue::List(Rc::new(RefCell::new(list))))
}
fn native_input(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if let Some(DgmValue::Str(prompt)) = args.first() { print!("{}", prompt); use std::io::Write; std::io::stdout().flush().ok(); }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).map_err(|e| DgmError::RuntimeError { msg: format!("input error: {}", e) })?;
    Ok(DgmValue::Str(line.trim_end_matches('\n').trim_end_matches('\r').to_string()))
}
fn native_abs(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Int(n)) => Ok(DgmValue::Int(n.abs())),
        Some(DgmValue::Float(f)) => Ok(DgmValue::Float(f.abs())),
        _ => Err(DgmError::RuntimeError { msg: "abs() requires number".into() }),
    }
}
fn native_min(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.len() == 1 { if let DgmValue::List(l) = &args[0] { let b = l.borrow(); return b.iter().cloned().reduce(|a, b| if cmp_lt(&a, &b) { a } else { b }).ok_or_else(|| DgmError::RuntimeError { msg: "min() empty list".into() }); } }
    args.into_iter().reduce(|a, b| if cmp_lt(&a, &b) { a } else { b }).ok_or_else(|| DgmError::RuntimeError { msg: "min() requires args".into() })
}
fn native_max(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.len() == 1 { if let DgmValue::List(l) = &args[0] { let b = l.borrow(); return b.iter().cloned().reduce(|a, b| if cmp_lt(&a, &b) { b } else { a }).ok_or_else(|| DgmError::RuntimeError { msg: "max() empty list".into() }); } }
    args.into_iter().reduce(|a, b| if cmp_lt(&a, &b) { b } else { a }).ok_or_else(|| DgmError::RuntimeError { msg: "max() requires args".into() })
}
fn cmp_lt(a: &DgmValue, b: &DgmValue) -> bool {
    match (a, b) { (DgmValue::Int(x), DgmValue::Int(y)) => x < y, (DgmValue::Float(x), DgmValue::Float(y)) => x < y,
        (DgmValue::Int(x), DgmValue::Float(y)) => (*x as f64) < *y, (DgmValue::Float(x), DgmValue::Int(y)) => *x < (*y as f64), _ => false }
}
fn native_sort(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => { let mut v = l.borrow().clone(); v.sort_by(|a, b| { if cmp_lt(a, b) { std::cmp::Ordering::Less } else if dgm_eq(a, b) { std::cmp::Ordering::Equal } else { std::cmp::Ordering::Greater } }); Ok(DgmValue::List(Rc::new(RefCell::new(v)))) }
        _ => Err(DgmError::RuntimeError { msg: "sort() requires list".into() }),
    }
}
fn native_reverse(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => { let mut v = l.borrow().clone(); v.reverse(); Ok(DgmValue::List(Rc::new(RefCell::new(v)))) }
        Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.chars().rev().collect())),
        _ => Err(DgmError::RuntimeError { msg: "reverse() requires list or string".into() }),
    }
}
fn native_keys(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Map(m)) => Ok(DgmValue::List(Rc::new(RefCell::new(m.borrow().keys().map(|k| DgmValue::Str(k.clone())).collect())))),
        _ => Err(DgmError::RuntimeError { msg: "keys() requires map".into() }),
    }
}
fn native_values(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::Map(m)) => Ok(DgmValue::List(Rc::new(RefCell::new(m.borrow().values().cloned().collect())))),
        _ => Err(DgmError::RuntimeError { msg: "values() requires map".into() }),
    }
}
fn native_has_key(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Map(m)), Some(DgmValue::Str(k))) => Ok(DgmValue::Bool(m.borrow().contains_key(k))),
        _ => Err(DgmError::RuntimeError { msg: "has_key(map, key) required".into() }),
    }
}
fn native_slice(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::List(l)), Some(DgmValue::Int(start)), end) => {
            let list = l.borrow(); let s = *start as usize;
            let e = end.and_then(|v| if let DgmValue::Int(n) = v { Some(*n as usize) } else { None }).unwrap_or(list.len());
            Ok(DgmValue::List(Rc::new(RefCell::new(list[s..e.min(list.len())].to_vec()))))
        }
        (Some(DgmValue::Str(s)), Some(DgmValue::Int(start)), end) => {
            let st = *start as usize;
            let e = end.and_then(|v| if let DgmValue::Int(n) = v { Some(*n as usize) } else { None }).unwrap_or(s.len());
            Ok(DgmValue::Str(s.chars().skip(st).take(e.saturating_sub(st)).collect()))
        }
        _ => Err(DgmError::RuntimeError { msg: "slice(list/str, start, end?) required".into() }),
    }
}
fn native_join(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(DgmValue::Str(sep))) => { let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect(); Ok(DgmValue::Str(items.join(sep))) }
        (Some(DgmValue::List(l)), None) => { let items: Vec<String> = l.borrow().iter().map(|v| format!("{}", v)).collect(); Ok(DgmValue::Str(items.join(""))) }
        _ => Err(DgmError::RuntimeError { msg: "join(list, sep?) required".into() }),
    }
}
fn native_split(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sep))) => Ok(DgmValue::List(Rc::new(RefCell::new(s.split(sep.as_str()).map(|p| DgmValue::Str(p.to_string())).collect())))),
        _ => Err(DgmError::RuntimeError { msg: "split(str, sep) required".into() }),
    }
}
fn native_replace(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(from)), Some(DgmValue::Str(to))) => Ok(DgmValue::Str(s.replace(from.as_str(), to))),
        _ => Err(DgmError::RuntimeError { msg: "replace(str, from, to) required".into() }),
    }
}
fn native_upper(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> { match args.first() { Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.to_uppercase())), _ => Err(DgmError::RuntimeError { msg: "upper() requires string".into() }) } }
fn native_lower(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> { match args.first() { Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.to_lowercase())), _ => Err(DgmError::RuntimeError { msg: "lower() requires string".into() }) } }
fn native_trim(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> { match args.first() { Some(DgmValue::Str(s)) => Ok(DgmValue::Str(s.trim().to_string())), _ => Err(DgmError::RuntimeError { msg: "trim() requires string".into() }) } }
fn native_contains(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sub))) => Ok(DgmValue::Bool(s.contains(sub.as_str()))),
        (Some(DgmValue::List(l)), Some(v)) => Ok(DgmValue::Bool(l.borrow().iter().any(|x| dgm_eq(x, v)))),
        _ => Err(DgmError::RuntimeError { msg: "contains(str/list, val) required".into() }),
    }
}
fn native_starts_with(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) { (Some(DgmValue::Str(s)), Some(DgmValue::Str(p))) => Ok(DgmValue::Bool(s.starts_with(p.as_str()))), _ => Err(DgmError::RuntimeError { msg: "starts_with(str, prefix) required".into() }) }
}
fn native_ends_with(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) { (Some(DgmValue::Str(s)), Some(DgmValue::Str(p))) => Ok(DgmValue::Bool(s.ends_with(p.as_str()))), _ => Err(DgmError::RuntimeError { msg: "ends_with(str, suffix) required".into() }) }
}
fn native_chars(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() { Some(DgmValue::Str(s)) => Ok(DgmValue::List(Rc::new(RefCell::new(s.chars().map(|c| DgmValue::Str(c.to_string())).collect())))), _ => Err(DgmError::RuntimeError { msg: "chars() requires string".into() }) }
}
fn native_format(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if args.is_empty() { return Err(DgmError::RuntimeError { msg: "format() requires args".into() }); }
    let template = match &args[0] { DgmValue::Str(s) => s.clone(), _ => return Err(DgmError::RuntimeError { msg: "format() first arg must be string".into() }) };
    let mut result = template;
    for arg in &args[1..] { result = result.replacen("{}", &format!("{}", arg), 1); }
    Ok(DgmValue::Str(result))
}
fn native_map_fn(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let mut results = vec![];
            for item in l.borrow().iter() {
                let r = call_native_hof(func, vec![item.clone()])?;
                results.push(r);
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(results))))
        }
        _ => Err(DgmError::RuntimeError { msg: "map(list, fn) required".into() }),
    }
}
fn native_filter(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            let mut results = vec![];
            for item in l.borrow().iter() {
                let r = call_native_hof(func, vec![item.clone()])?;
                if matches!(r, DgmValue::Bool(true)) { results.push(item.clone()); }
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(results))))
        }
        _ => Err(DgmError::RuntimeError { msg: "filter(list, fn) required".into() }),
    }
}
fn native_reduce(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1), args.get(2)) {
        (Some(DgmValue::List(l)), Some(init), Some(func)) => {
            let mut acc = init.clone();
            for item in l.borrow().iter() { acc = call_native_hof(func, vec![acc, item.clone()])?; }
            Ok(acc)
        }
        _ => Err(DgmError::RuntimeError { msg: "reduce(list, init, fn) required".into() }),
    }
}
fn native_each(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            for item in l.borrow().iter() { call_native_hof(func, vec![item.clone()])?; }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::RuntimeError { msg: "each(list, fn) required".into() }),
    }
}
fn native_find(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            for item in l.borrow().iter() {
                let r = call_native_hof(func, vec![item.clone()])?;
                if matches!(r, DgmValue::Bool(true)) { return Ok(item.clone()); }
            }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::RuntimeError { msg: "find(list, fn) required".into() }),
    }
}
fn native_index_of(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(val)) => {
            for (i, item) in l.borrow().iter().enumerate() { if dgm_eq(item, val) { return Ok(DgmValue::Int(i as i64)); } }
            Ok(DgmValue::Int(-1))
        }
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(sub))) => { Ok(DgmValue::Int(s.find(sub.as_str()).map(|i| i as i64).unwrap_or(-1))) }
        _ => Err(DgmError::RuntimeError { msg: "index_of(list/str, val) required".into() }),
    }
}
fn native_flat(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() {
        Some(DgmValue::List(l)) => {
            let mut result = vec![];
            for item in l.borrow().iter() {
                if let DgmValue::List(inner) = item { result.extend(inner.borrow().clone()); }
                else { result.push(item.clone()); }
            }
            Ok(DgmValue::List(Rc::new(RefCell::new(result))))
        }
        _ => Err(DgmError::RuntimeError { msg: "flat() requires list".into() }),
    }
}
fn native_zip(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(a)), Some(DgmValue::List(b))) => {
            let a = a.borrow(); let b = b.borrow();
            let result: Vec<DgmValue> = a.iter().zip(b.iter()).map(|(x, y)| DgmValue::List(Rc::new(RefCell::new(vec![x.clone(), y.clone()])))).collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(result))))
        }
        _ => Err(DgmError::RuntimeError { msg: "zip(list, list) required".into() }),
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
                    _ => return Err(DgmError::RuntimeError { msg: "sum() requires list of numbers".into() }),
                };
            }
            Ok(total)
        }
        _ => Err(DgmError::RuntimeError { msg: "sum() requires list".into() }),
    }
}
fn native_any(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            for item in l.borrow().iter() {
                let r = call_native_hof(func, vec![item.clone()])?;
                if matches!(r, DgmValue::Bool(true)) { return Ok(DgmValue::Bool(true)); }
            }
            Ok(DgmValue::Bool(false))
        }
        (Some(DgmValue::List(l)), None) => { Ok(DgmValue::Bool(l.borrow().iter().any(|v| !matches!(v, DgmValue::Bool(false) | DgmValue::Null)))) }
        _ => Err(DgmError::RuntimeError { msg: "any(list, fn?) required".into() }),
    }
}
fn native_all(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (args.get(0), args.get(1)) {
        (Some(DgmValue::List(l)), Some(func)) => {
            for item in l.borrow().iter() {
                let r = call_native_hof(func, vec![item.clone()])?;
                if !matches!(r, DgmValue::Bool(true)) { return Ok(DgmValue::Bool(false)); }
            }
            Ok(DgmValue::Bool(true))
        }
        (Some(DgmValue::List(l)), None) => { Ok(DgmValue::Bool(l.borrow().iter().all(|v| !matches!(v, DgmValue::Bool(false) | DgmValue::Null)))) }
        _ => Err(DgmError::RuntimeError { msg: "all(list, fn?) required".into() }),
    }
}
fn native_print(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let parts: Vec<String> = args.iter().map(|v| format!("{}", v)).collect();
    print!("{}", parts.join(" "));
    use std::io::Write; std::io::stdout().flush().ok();
    Ok(DgmValue::Null)
}
fn native_println(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let parts: Vec<String> = args.iter().map(|v| format!("{}", v)).collect();
    println!("{}", parts.join(" "));
    Ok(DgmValue::Null)
}
fn native_chr(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() { Some(DgmValue::Int(n)) => Ok(DgmValue::Str(char::from_u32(*n as u32).unwrap_or('\0').to_string())), _ => Err(DgmError::RuntimeError { msg: "chr() requires int".into() }) }
}
fn native_ord(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() { Some(DgmValue::Str(s)) => s.chars().next().map(|c| DgmValue::Int(c as i64)).ok_or_else(|| DgmError::RuntimeError { msg: "ord() empty string".into() }), _ => Err(DgmError::RuntimeError { msg: "ord() requires string".into() }) }
}
fn native_hex(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() { Some(DgmValue::Int(n)) => Ok(DgmValue::Str(format!("0x{:x}", n))), _ => Err(DgmError::RuntimeError { msg: "hex() requires int".into() }) }
}
fn native_bin(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match args.first() { Some(DgmValue::Int(n)) => Ok(DgmValue::Str(format!("0b{:b}", n))), _ => Err(DgmError::RuntimeError { msg: "bin() requires int".into() }) }
}
fn native_exit(args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let code = match args.first() { Some(DgmValue::Int(n)) => *n as i32, _ => 0 };
    std::process::exit(code);
}

/// Helper to call a DgmValue::Function from native context (for map/filter/reduce)
fn call_native_hof(func: &DgmValue, args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match func {
        DgmValue::NativeFunction { func: f, .. } => f(args),
        DgmValue::Function { params, body, closure } => {
            if params.len() != args.len() { return Err(DgmError::RuntimeError { msg: format!("expected {} args, got {}", params.len(), args.len()) }); }
            let call_env = Rc::new(RefCell::new(Environment::new_child(Rc::clone(closure))));
            for (p, v) in params.iter().zip(args) { call_env.borrow_mut().set(p, v); }
            // Mini interpreter for HOF - we need a temporary interpreter
            let mut interp = Interpreter { globals: Rc::clone(closure), classes: HashMap::new(), imported_modules: HashMap::new() };
            for stmt in body {
                match interp.exec_stmt(stmt, Rc::clone(&call_env))? {
                    ControlFlow::Return(v) => return Ok(v),
                    _ => {}
                }
            }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::RuntimeError { msg: "not callable".into() }),
    }
}
