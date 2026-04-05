# DGM Language Specification – Frozen (v0.2.0)

> Stable language surface for DGM Alpha_Major_1

---

## 1. SYNTAX RULES

### 1.1 Comments
- **Line comments**: `#` to end of line
- **Multiline comments**: Not supported
- **Example**:
  ```dgm
  # This is a comment
  let x = 5  # inline comment
  ```

### 1.2 String Literals
- **Syntax**: Double quotes `"..."`
- **Escapes**: `\n`, `\t`, `\r`, `\\`, `\"`
- **F-strings**: `f"variable: {expr}"`
- **Example**:
  ```dgm
  let s = "hello"
  let name = "Alice"
  writ(f"Hello, {name}!")
  ```

### 1.3 Number Literals
- **Integers**: `123`, `-456`, `0`
- **Floats**: `3.14`, `-2.5`, `0.0`
- **No scientific notation** (frozen as is)
- **Example**:
  ```dgm
  let i = 42
  let f = 3.14
  ```

### 1.4 Boolean & Null
- **Boolean literals**: `tru`, `fals` (not `true`/`false`)
- **Null literal**: `nul`
- **Example**:
  ```dgm
  let flag = tru
  let nothing = nul
  ```

### 1.5 Identifiers
- **Valid**: `[a-zA-Z_][a-zA-Z0-9_]*`
- **Reserved**: All keywords below
- **Example**: `my_var`, `_private`, `x1`

---

## 2. FROZEN KEYWORD SET

| Keyword | Purpose | Category |
|---------|---------|----------|
| `let` | Variable binding | Declaration |
| `def` | Function definition | Declaration |
| `cls` | Class definition | Declaration |
| `new` | Object instantiation | Expression |
| `ths` | This pointer | Expression |
| `in` | Membership test (iterator) | Operator |
| `imprt` | Module import | Statement |
| `writ` | Print output | Statement |
| `iff` | If conditional | Control |
| `elseif` | Else-if branch | Control |
| `els` | Else branch | Control |
| `fr` | For loop | Control |
| `whl` | While loop | Control |
| `brk` | Break loop | Control |
| `cont` | Continue loop | Control |
| `retrun` | Return from function | Control |
| `try` | Try block | Error Handling |
| `catch` | Catch exception | Error Handling |
| `finally` | Finally block | Error Handling |
| `throw` | Throw exception | Error Handling |
| `match` | Pattern matching | Control |
| `and` | Logical AND | Operator |
| `or` | Logical OR | Operator |
| `not` | Logical NOT | Operator |
| `tru` | Boolean true | Literal |
| `fals` | Boolean false | Literal |
| `nul` | Null value | Literal |
| `extends` | Class inheritance | Declaration |
| `lam` | Lambda expression | Expression |

**Constraint**: This set is frozen. No additions without major version bump.

---

## 3. OPERATOR PRECEDENCE & ASSOCIATIVITY

| Precedence | Operators | Associativity |
|-----------|-----------|---------------|
| 1 (Low) | `or` | Left |
| 2 | `and` | Left |
| 3 | `==`, `!=`, `<`, `>`, `<=`, `>=` | Left |
| 4 | `\|`, `&`, `^` | Left |
| 5 | `<<`, `>>` | Left |
| 6 | `+`, `-` | Left |
| 7 | `*`, `/`, `%` | Left |
| 8 | `**` | Right |
| 9 (High) | `.`, `[`, `(`, `!`, `~`, `-` (unary) | Left |

---

## 4. STATEMENT TYPES (AST)

```
Stmt::Let { name, value }              // let x = expr
Stmt::Writ(expr)                       // writ(expr)
Stmt::If { cond, then_b, else_b }      // iff (cond) { ... } els { ... }
Stmt::While { cond, body }             // whl (cond) { ... }
Stmt::For { var, iter, body }          // fr (var in iter) { ... }
Stmt::FuncDef { name, params, body }   // def name(a, b) { ... }
Stmt::ClassDef { name, parent, body }  // cls Name { ... }
Stmt::Return(expr)                     // retrun expr
Stmt::Break                            // brk
Stmt::Continue                         // cont
Stmt::TryCatch { try_b, catch_b, finally_b }  // try { ... } catch (e) { ... }
Stmt::Throw(expr)                      // throw expr
Stmt::Match { expr, arms }             // match expr { ... }
Stmt::Imprt(name)                      // imprt "module" or imprt module
Stmt::Expr(expr)                       // standalone expression
```

---

## 5. EXPRESSION TYPES (AST)

```
Expr::Literal(value)                   // 42, "hello", tru, nul
Expr::Ident(name)                      // x, my_var
Expr::Binary { left, op, right }       // a + b, x == y
Expr::Unary { op, operand }            // -x, !flag
Expr::Call { func, args }              // func(a, b)
Expr::Index { object, index }          // arr[0], map["key"]
Expr::Property { object, property }    // obj.field
Expr::FuncLit { params, body }         // lam(a, b) { a + b }
Expr::ClassInstantiate { class, args } // new MyClass(x, y)
Expr::Array(elements)                  // [1, 2, 3]
Expr::Map(pairs)                       // {"key": value, ...}
```

---

## 6. TYPE SYSTEM

DGM is **dynamically typed**. Runtime values:

```
DgmValue::Null
DgmValue::Bool(bool)
DgmValue::Number(f64)
DgmValue::String(String)
DgmValue::Array(Rc<RefCell<Vec<DgmValue>>>)
DgmValue::Map(Rc<RefCell<HashMap<String, DgmValue>>>)
DgmValue::Function { params, body, env }
DgmValue::BuiltinFunction { name, arity }
DgmValue::Object { fields, methods }
DgmValue::NativeModule(HashMap<String, DgmValue>)
```

---

## 7. ERROR MESSAGE FORMAT

All errors use standardized format:

```
[ErrorType line N] message
```

Types:
- `[LexError line N]` — Tokenization failure
- `[ParseError line N]` — Syntax parsing failure
- `[RuntimeError]` — Runtime execution failure
- `[ThrownError]` — User-thrown exception
- `[ImportError]` — Module loading failure

**Example**:
```
[ParseError line 5] expected ')' found 'iff'
[RuntimeError] undefined variable 'x'
```

---

## 8. CONTROL FLOW

### If/Else
```dgm
iff (x > 0) {
  writ("positive")
} elseif (x < 0) {
  writ("negative")
} els {
  writ("zero")
}
```

### While Loop
```dgm
whl (i < 10) {
  writ(i)
  i = i + 1
}
```

### For Loop (Iterator)
```dgm
fr (item in arr) {
  writ(item)
}
```

### Break/Continue
```dgm
fr (i in [1,2,3,4,5]) {
  iff (i == 3) { brk }
  writ(i)
}
```

### Try/Catch/Finally
```dgm
try {
  # code
} catch (err) {
  writ(err)
} finally {
  # cleanup
}
```

### Throw
```dgm
throw "error message"
```

### Return
```dgm
def add(a, b) {
  retrun a + b
}
```

---

## 9. FUNCTION DEFINITIONS

```dgm
def name(param1, param2) {
  # function body
  retrun result
}

# Lambda (anonymous)
let square = lam(x) { x * x }
```

---

## 10. CLASS DEFINITIONS

```dgm
cls Animal {
  def init(name) {
    ths.name = name
  }
  
  def speak() {
    writ(f"{ths.name} makes a sound")
  }
}

cls Dog extends Animal {
  def speak() {
    writ(f"{ths.name} barks")
  }
}

let dog = new Dog("Buddy")
dog.speak()
```

---

## 11. PATTERN MATCHING

```dgm
match x {
  0 => writ("zero"),
  1 => writ("one"),
  _ => writ("other")
}
```

---

## 12. MODULE SYSTEM

Import modules:
```dgm
imprt "math"
imprt "http"
imprt "json"

# or
imprt math
```

Available modules: `math`, `io`, `fs`, `os`, `json`, `time`, `http`, `crypto`, `regex`, `net`, `thread`, `xml`, `security`

---

## 13. OPERATOR SEMANTICS

| Operator | Type | Semantics |
|----------|------|-----------|
| `+` | Binary | Addition (numbers) or concatenation (strings) |
| `-` | Binary | Subtraction |
| `-` | Unary | Negation |
| `*` | Binary | Multiplication |
| `/` | Binary | Division |
| `%` | Binary | Modulo |
| `**` | Binary | Power |
| `==` | Binary | Equality (loose) |
| `!=` | Binary | Not equal |
| `<`, `>`, `<=`, `>=` | Binary | Comparison |
| `and`, `or`, `not` | Logical | Short-circuit evaluation |
| `&`, `\|`, `^` | Bitwise | Integer operations |
| `<<`, `>>` | Bitwise | Shift operations (integers) |
| `!` | Unary | Logical NOT (alias for `not`) |
| `~` | Unary | Bitwise NOT |
| `?` | Operator | Optional access (defined in stdlib only) |

---

## STABILITY NOTES

- **No changes** to keyword set without major version bump
- **No changes** to operator semantics in this version
- **Error format** is stable and must be preserved
- **All string escapes** must be consistent across implementations
- **Comment style** is frozen: `#` only
- **Identifier rules** are frozen: `[a-zA-Z_][a-zA-Z0-9_]*`

---

**Last Updated**: DGM v0.2.0  
**Status**: Frozen ✓
