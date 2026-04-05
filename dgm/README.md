# DGM Programming Language

<div align="center">

**A dynamically typed, interpreted programming language — written in pure Rust.**

Named after **Đặng Gia Minh** · Built from scratch · Zero parser generators · Zero external parser combinators

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-Alpha__Major__1-green.svg)](Cargo.toml)
[![Tests](https://img.shields.io/badge/tests-17%20passed-brightgreen.svg)](#testing)

</div>

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Language Syntax](#language-syntax)
  - [Variables & Types](#variables--types)
  - [Operators](#operators)
  - [Control Flow](#control-flow)
  - [Functions](#functions)
  - [Classes](#classes)
  - [Error Handling](#error-handling)
  - [Pattern Matching](#pattern-matching)
- [Standard Library](#standard-library)
  - [math](#math)
  - [io](#io)
  - [fs](#fs--sandboxed-filesystem)
  - [os](#os)
  - [json](#json)
  - [http](#http)
  - [crypto](#crypto)
  - [regex](#regex)
  - [net](#net)
  - [time](#time)
  - [thread](#thread)
  - [xml](#xml)
  - [security](#security)
- [Security Model](#security-model)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Dependencies](#dependencies)
- [Performance](#performance)
- [Testing](#testing)
- [License](#license)

---

## Overview

DGM is a **tree-walk interpreted** programming language with:

- A **hand-written lexer and recursive descent parser** — no yacc, no pest, no nom
- A **single-pass tree-walk interpreter** with lexical scoping via `Rc<RefCell<>>`
- A **13-module standard library** covering I/O, networking, cryptography, JSON, XML, and more
- A **thread-local security layer** with filesystem sandboxing, exec gating, and network host whitelisting
- A **byte-level JSON encoder** using `itoa` and `ryu` — zero intermediate `serde_json::Value` allocations
- An **HTTP server** with zero-copy response path and pooled key buffers

---

## Features

| Feature | Description |
|---|---|
| Dynamic typing | Int, Float, Str, Bool, Null, List, Map, Function, Instance |
| First-class functions | Closures, lambdas, higher-order functions |
| Classes & inheritance | `cls`, `new`, `ths`, single-parent inheritance |
| Exception handling | `try / catch / finally / thr` |
| Pattern matching | `match / case` |
| String interpolation | `f"Hello, {name}!"` |
| Module system | `imprt <module>` — lazy loading |
| REPL | Interactive shell with history (`rustyline`) |
| HTTP server | Built-in TCP server via `tiny_http` |
| Security sandboxing | Filesystem sandbox, exec gate, network whitelist |

---

## Installation

### Prerequisites

- Rust 1.75+ ([install](https://rustup.rs))

### Build from source

```bash
git clone <repository-url>
cd dgm-source/dgm
cargo build --release
```

The compiled binary will be at `target/release/dgm`.

### Add to PATH (Linux/macOS)

```bash
sudo cp target/release/dgm /usr/local/bin/dgm
```

---

## Usage

```bash
# Run a DGM script
dgm run script.dgm

# Start interactive REPL
dgm repl

# Show version
dgm version

# Show help
dgm help
```

### REPL commands

```
>>> help          — Show help and available modules
>>> .clear        — Clear the screen
>>> exit / quit   — Exit the REPL
```

---

## Language Syntax

### Variables & Types

```dgm
# Variables — dynamically typed
let x = 42
let pi = 3.14159
let name = "DGM"
let active = tru
let empty = nul

# Lists
let nums = [1, 2, 3, 4, 5]
let mixed = [1, "two", tru, nul]

# Maps (dictionaries)
let person = {"name": "Alice", "age": 30}

# String interpolation
let greeting = f"Hello, {name}! You have {len(nums)} items."
```

### Operators

```dgm
# Arithmetic
let a = 10 + 3    # 13
let b = 10 - 3    # 7
let c = 10 * 3    # 30
let d = 10 / 3    # 3
let e = 10 % 3    # 1
let f = 2 ** 8    # 256

# Bitwise
let g = 0xFF & 0x0F    # 15
let h = 1 << 4          # 16

# Comparison
let eq  = (a == b)    # fals
let neq = (a != b)    # tru
let lt  = (a < 20)    # tru

# Logical
let both = tru and fals    # fals
let either = tru or fals   # tru
let inv = not tru           # fals

# Membership
let inList = 3 in [1, 2, 3, 4]    # tru
let inMap  = "name" in person       # tru

# Compound assignment
x += 5
x -= 2
x *= 3
x /= 2
```

### Control Flow

```dgm
# if / else if / else
iff x > 0 {
    writ("positive")
} elseif x == 0 {
    writ("zero")
} els {
    writ("negative")
}

# Ternary
let label = x > 0 ? "pos" : "non-pos"

# for loop
fr i in range(10) {
    writ(i)
}

fr item in ["a", "b", "c"] {
    writ(item)
}

# while loop
let n = 5
whl n > 0 {
    writ(n)
    n -= 1
}

# break / continue
fr i in range(100) {
    iff i == 5 { brk }
    iff i % 2 == 0 { cont }
    writ(i)
}
```

### Functions

```dgm
# Function definition
def add(a, b) {
    retrun a + b
}

# Default usage
let result = add(3, 4)    # 7

# Lambda / anonymous function
let square = |x| x * x
writ(square(5))    # 25

# Higher-order functions
let nums = [1, 2, 3, 4, 5]
let doubled = map(nums, |x| x * 2)
let evens   = filter(nums, |x| x % 2 == 0)
let total   = reduce(nums, 0, |acc, x| acc + x)

# Closures
def make_counter() {
    let count = 0
    retrun def() {
        count += 1
        retrun count
    }
}
let counter = make_counter()
writ(counter())    # 1
writ(counter())    # 2
```

### Classes

```dgm
cls Animal {
    def init(name, sound) {
        ths.name = name
        ths.sound = sound
    }

    def speak() {
        writ(f"{ths.name} says {ths.sound}!")
    }

    def to_str() {
        retrun f"Animal({ths.name})"
    }
}

# Inheritance
cls Dog extnd Animal {
    def init(name) {
        ths.name = name
        ths.sound = "Woof"
    }

    def fetch(item) {
        writ(f"{ths.name} fetches the {item}!")
    }
}

let dog = new Dog("Rex")
dog.speak()       # Rex says Woof!
dog.fetch("ball") # Rex fetches the ball!
```

### Error Handling

```dgm
try {
    let result = risky_operation()
    writ(result)
} catch err {
    writ(f"Error caught: {err}")
} finally {
    writ("Always runs")
}

# Throw an error
def divide(a, b) {
    iff b == 0 {
        thr "Division by zero"
    }
    retrun a / b
}
```

### Pattern Matching

```dgm
let status = 404

match status {
    case 200 { writ("OK") }
    case 404 { writ("Not Found") }
    case 500 { writ("Server Error") }
    default  { writ(f"Unknown: {status}") }
}
```

---

## Standard Library

Use `imprt <module>` to load a module:

```dgm
imprt json
let data = json.parse('{"key": "value"}')
```

### math

```dgm
imprt math
math.sin(math.PI / 2)       # 1.0
math.sqrt(16)               # 4.0
math.pow(2, 10)             # 1024
math.random()               # 0.0..1.0
math.floor(3.7)             # 3
math.ceil(3.2)              # 4
math.log(math.E)            # 1.0
```

### io

```dgm
imprt io
let content = io.read_file("data.txt")
io.write_file("output.txt", "Hello!")
io.append_file("log.txt", "new line\n")
let lines = io.read_lines("data.txt")
let files = io.list_dir("./src")
io.mkdir("new_folder")
io.rename("old.txt", "new.txt")
io.copy("src.txt", "dst.txt")
io.delete("file.txt")
let exists = io.exists("file.txt")    # tru / fals
let size   = io.file_size("file.txt") # bytes
let path   = io.abs_path("./relative")
let cwd    = io.cwd()
```

### fs — Sandboxed Filesystem

```dgm
imprt fs
# All operations respect the sandbox_root security config

let content    = fs.read("data.txt")              # str
let bytes      = fs.read_bytes("data.bin")        # list[int]
fs.write("out.txt", "content")
fs.write_bytes("out.bin", [0x48, 0x65, 0x6C, 0x6C, 0x6F])
fs.append("log.txt", "new line\n")
fs.delete("file.txt")
let exists     = fs.exists("file.txt")            # bool
let entries    = fs.list("./dir")                 # list[str]
fs.mkdir("new_dir")
fs.rmdir("old_dir")
fs.rename("old.txt", "new.txt")
let copied     = fs.copy("from.txt", "to.txt")   # bytes copied
let size       = fs.size("file.txt")              # int bytes
let is_file    = fs.is_file("file.txt")           # bool
let is_dir     = fs.is_dir("./dir")               # bool
let meta       = fs.metadata("file.txt")          # map {size, is_file, is_dir, readonly, modified}
```

### os

```dgm
imprt os
let result = os.exec("ls -la")           # {stdout, stderr, code, ok}
let proc   = os.spawn("long_process")    # {pid, handle, ok}
let done   = os.wait(proc.handle, 5000) # {code, ok, timed_out}
let proc2  = os.run("git", ["status"])  # {stdout, stderr, code, ok}
let res    = os.run_timeout("prog", ["arg"], 3000)  # + {timed_out}

let home   = os.env_get("HOME")
os.env_set("KEY", "value")
let cwd    = os.cwd()
os.chdir("/tmp")
let pid    = os.pid()
os.sleep(1000)          # ms
let plat   = os.platform()   # "linux" / "macos" / "windows"
let cpus   = os.num_cpus()
```

### json

```dgm
imprt json
let parsed = json.parse('{"name":"Alice","age":30}')
let str    = json.stringify(parsed)         # fast byte-level encode
let pretty = json.pretty(parsed)            # indented
let resp   = json.raw_parts("users", data) # {"ok":true,"users":<data>}
```

### http

```dgm
imprt http

# HTTP client
let res = http.get("https://api.example.com/users")
writ(res.status)     # 200
writ(res.body)       # response body string
writ(res.ok)         # tru

let created = http.post("https://api.example.com/users",
    json.stringify({"name": "Alice"}))

# HTTP server
let routes = {
    "GET /":      json.raw_parts("message", "Hello, DGM!"),
    "GET /users": json.stringify(get_users()),
    "GET /health": '{"ok":true,"status":"healthy"}'
}
http.serve(8080, routes)
```

### crypto

```dgm
imprt crypto
let hash     = crypto.sha256("hello world")
let md5hash  = crypto.md5("hello world")
let encoded  = crypto.base64_encode("binary data")
let decoded  = crypto.base64_decode(encoded)
let hmac     = crypto.hmac_sha256("secret", "message")
let randbytes = crypto.random_bytes(32)    # list[int]
```

### regex

```dgm
imprt regex
let match  = regex.test("[0-9]+", "abc123def")     # tru
let found  = regex.find("[0-9]+", "abc123def")     # "123"
let all    = regex.find_all("[0-9]+", "a1b2c3")    # ["1","2","3"]
let groups = regex.match("(\\w+)@(\\w+)", "user@host") # list
let rep    = regex.replace("[aeiou]", "hello", "*")    # "h*ll*"
```

### net

```dgm
imprt net
let sock = net.connect("127.0.0.1", 9000)
net.send(sock, "Hello, server!")
let data = net.recv(sock, 4096)
net.close(sock)

let server = net.listen("0.0.0.0", 9000)
```

### time

```dgm
imprt time
let ts  = time.now()                        # unix ms
let fmt = time.format(ts, "%Y-%m-%d %H:%M:%S")
let day = time.local_date()
time.sleep(500)                             # ms
```

### thread

```dgm
imprt thread
let h = thread.spawn(|| {
    writ("Running in background")
})
thread.join(h)
```

### xml

```dgm
imprt xml
let doc = xml.parse("<root><item>hello</item></root>")
let str = xml.stringify(doc)
let val = xml.query(doc, "root.item")
```

### security

```dgm
imprt security

# Configure runtime security policy
security.configure({
    "allow_fs":       tru,
    "allow_exec":     fals,
    "allow_net":      tru,
    "sandbox_root":   "/app/data",
    "allowed_hosts":  ["api.example.com", "cdn.trusted.io"]
})

# Check current config
let status = security.status()
writ(status.allow_exec)     # fals
writ(status.sandbox_root)   # /app/data
```

---

## Security Model

DGM provides a **thread-local security configuration** with no global mutex overhead.

### Controls

| Setting | Type | Default | Effect |
|---|---|---|---|
| `allow_fs` | bool | `true` | Gates all `fs.*` operations |
| `allow_exec` | bool | `true` | Gates `os.exec`, `os.spawn`, `os.run`, `os.run_timeout` |
| `allow_net` | bool | `true` | Gates `net.*` operations |
| `sandbox_root` | str \| null | `null` | Restricts `fs.*` to a directory subtree |
| `allowed_hosts` | list \| null | `null` | Restricts `net.*` to specific hosts |

### Sandbox path resolution

Path normalization is **lexical** — no syscalls, no `canonicalize()`:

```
Input path:  /sandbox/../../etc/passwd
             → normalize: /etc/passwd
             → /etc/passwd.starts_with(/sandbox) = false
             → BLOCKED: RuntimeError
```

This prevents path traversal attacks even when the target path does not exist on disk.

---

## Architecture

```
Source (.dgm)
    │
    ▼
Lexer (lexer.rs)          — hand-written, O(n) single pass
    │  Vec<Token>
    ▼
Parser (parser.rs)        — recursive descent, pratt-style precedence
    │  Vec<Stmt> (AST)
    ▼
Interpreter (interpreter.rs)
    │  ├── exec_stmt() — statement evaluation
    │  ├── eval_expr() — expression evaluation
    │  ├── call_function() — closure calls
    │  └── do_import() — lazy module loading
    │
    ├── Environment (environment.rs)
    │    └── Rc<RefCell<>> scope chain
    │
    └── stdlib/
         ├── mod.rs         — module dispatch
         ├── security.rs    — thread_local SecurityConfig
         ├── fs_mod.rs      — sandboxed FS
         ├── os_mod.rs      — process control
         ├── json_mod.rs    — byte-level JSON
         ├── http_mod.rs    — HTTP client/server
         └── ...13 modules total
```

---

## Project Structure

```
dgm-source/
├── workflow.flyde          # Visual flow graph (Flyde format)
├── WORKFLOW.md             # Detailed workflow + architecture documentation
├── README.md               # This file
│
└── dgm/
    ├── Cargo.toml          # Crate manifest
    ├── Cargo.lock          # Locked dependencies
    │
    ├── src/
    │   ├── main.rs         # CLI + REPL entry point
    │   ├── lexer.rs        # Tokenizer
    │   ├── token.rs        # Token type definitions
    │   ├── parser.rs       # Recursive descent parser
    │   ├── ast.rs          # AST node types (Stmt, Expr)
    │   ├── interpreter.rs  # Tree-walk interpreter + DgmValue
    │   ├── environment.rs  # Lexical scope chain
    │   ├── error.rs        # DgmError enum
    │   └── stdlib/
    │       ├── mod.rs
    │       ├── math.rs
    │       ├── io_mod.rs
    │       ├── fs_mod.rs
    │       ├── os_mod.rs
    │       ├── json_mod.rs
    │       ├── http_mod.rs
    │       ├── crypto_mod.rs
    │       ├── regex_mod.rs
    │       ├── net_mod.rs
    │       ├── time_mod.rs
    │       ├── thread_mod.rs
    │       ├── xml_mod.rs
    │       ├── security.rs
    │       └── tests.rs     # Integration tests
    │
    └── target/              # Auto-generated — By Cargo
        ├── debug/           # Debug build artifacts
        └── release/         # Production build artifacts
```

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `serde` | 1 | Serialization framework |
| `serde_json` | 1 | JSON (used by `json.pretty` only) |
| `itoa` | 1 | Zero-alloc integer formatting |
| `ryu` | 1 | Zero-alloc float formatting |
| `ureq` | 2 | HTTP client (sync, no tokio) |
| `tiny_http` | 0.12 | HTTP server (minimal, no async) |
| `regex` | 1 | Regular expressions |
| `sha2` | 0.10 | SHA-256 hashing |
| `md-5` | 0.10 | MD5 hashing |
| `base64` | 0.22 | Base64 encoding/decoding |
| `chrono` | 0.4 | Date/time formatting |
| `rand` | 0.8 | Random number generation |
| `rustyline` | 14 | REPL readline with history |
| `quick-xml` | 0.36 | XML parsing/serialization |

---

## Performance

Measured under soak testing on Linux (stable, no warm-up):

| Metric | Result |
|---|---|
| HTTP throughput | ~660 req/s |
| RSS memory | ~32 MB (stable) |
| Memory leaks | None detected |
| JSON encode (per request) | ~0 heap allocations |
| HTTP body copy | 0 (direct `&[u8]` write) |
| Sandbox path check | 0 syscalls (lexical only) |

---

## Testing

```bash
# Run all tests (serial — required due to thread-local security state)
cargo test -- --test-threads=1

# Run specific test
cargo test test_fs_sandbox_violation -- --test-threads=1

# Check compilation without running
cargo check
```

**Test coverage (17 tests):**

| Test | Description |
|---|---|
| `test_normalize_path` | Path normalization correctness |
| `test_sandbox_blocks_escape` | `..` traversal blocked by sandbox |
| `test_fs_denied` | `allow_fs=false` blocks all FS ops |
| `test_exec_denied` | `allow_exec=false` blocks exec |
| `test_net_denied` | `allow_net=false` blocks net |
| `test_host_whitelist` | Host whitelist allows/blocks |
| `test_sandbox_path_inside` | Paths inside sandbox allowed |
| `test_sandbox_path_escape_blocked` | Paths outside sandbox blocked |
| `test_fs_blocked_when_disabled` | FS gate enforcement |
| `test_exec_blocked_when_disabled` | Exec gate enforcement |
| `test_fs_write_read_delete_cycle` | Full write → read → exists → delete |
| `test_fs_append` | Append to existing file |
| `test_fs_list` | Directory listing |
| `test_fs_sandbox_violation` | Sandbox escape via absolute path |
| `test_os_exec_enabled` | `os.exec("echo hello")` captures stdout |
| `test_os_exec_blocked` | Exec blocked when policy disallows |
| `test_os_cwd_chdir` | cwd/chdir roundtrip |

---

## License

```
Copyright 2026 Đặng Gia Minh

Licensed under the Apache License, Version Alpha_Major_1 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

This project is licensed under the **Apache License 2.0**.
See the full license text at: https://www.apache.org/licenses/LICENSE-2.0

---

<div align="center">

Built with in Rust · Created by **Đặng Gia Minh** · Apache 2.0

</div>
