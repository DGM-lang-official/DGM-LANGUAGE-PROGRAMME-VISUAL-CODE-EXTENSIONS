# DGM Programming Language — Rust Interpreter

<div align="center">

**A dynamically typed, interpreted programming language — implemented in pure Rust.**

This is the **interpreter implementation** within the [dgm-source repository](../)

Named after **Đặng Gia Minh** · Built from scratch · Zero parser generators · Zero external parser combinators

[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](../LICENSE)
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-Alpha__Major__1-green.svg)](Cargo.toml)
[![Tests](https://img.shields.io/badge/tests-contract--driven-brightgreen.svg)](#testing)

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

> **Part of dgm-source** — This folder contains the **Rust interpreter implementation** for DGM. The parent repository also includes documentation (LANGUAGE_SPEC.md, STDLIB_SPEC.md), a VS Code extension (vscode-dgm/), and additional project resources.

DGM is a **tree-walk interpreted** programming language with:

- A **hand-written lexer and recursive descent parser** — no yacc, no pest, no nom
- A **single-pass tree-walk interpreter** with lexical scoping via `Rc<RefCell<>>`
- A **semantic analyzer and Rust LSP server** powering diagnostics, navigation, completion, rename, and formatting
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
| Classes & inheritance | `class`, `new`, `this`, single-parent inheritance |
| Exception handling | `try / catch / finally / throw` |
| Pattern matching | `match` with `=>` arms |
| String interpolation | `f"Hello, {name}!"` |
| Module system | `import <module>` with optional `as` alias — lazy loading |
| REPL | Interactive shell with history (`rustyline`) |
| HTTP server | Built-in TCP server via `tiny_http` |
| Security sandboxing | Filesystem sandbox, exec gate, network whitelist |
| Tooling | Semantic `validate`, built-in formatter, Rust LSP server |

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

<!-- GENERATED:CLI_USAGE:START -->
```bash
# Run a DGM script
dgm run script.dgm
```
<!-- GENERATED:CLI_USAGE:END -->

## Executable Examples

These snippets are generated from executable fixtures in `../tests/examples` so the README stays locked to real runtime behavior.

### Hello World

{{include: ../tests/examples/hello_world/input.dgm}}

### Control Flow

{{include: ../tests/examples/control_flow/input.dgm}}

### XML Query

{{include: ../tests/examples/xml_query/input.dgm}}

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
let active = true
let empty = null

# Lists
let nums = [1, 2, 3, 4, 5]
let mixed = [1, "two", true, null]

# Maps (dictionaries)
let person = {"name": "Alice", "age": 30}

# String interpolation
let greeting = f"Hello, {name}! You have {len(nums)} items."
```

Canonical literals are `true`, `false`, `null`. Legacy aliases `tru`, `fals`, and `nul` are still accepted by the lexer.

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
let eq  = (a == b)    # false
let neq = (a != b)    # true
let lt  = (a < 20)    # true

# Logical
let both = true and false    # false
let either = true or false   # true
let inv = not true           # false

# Membership
let inList = 3 in [1, 2, 3, 4]    # true
let inMap  = "name" in person       # true

# Compound assignment
x += 5
x -= 2
x *= 3
x /= 2
```

### Control Flow

```dgm
# if / else if / else
if x > 0 {
    writ("positive")
} elseif x == 0 {
    writ("zero")
} else {
    writ("negative")
}

# Ternary
let label = x > 0 ? "pos" : "non-pos"

# for loop
for i in range(10) {
    writ(i)
}

for item in ["a", "b", "c"] {
    writ(item)
}

# while loop
let n = 5
while n > 0 {
    writ(n)
    n -= 1
}

# break / continue
for i in range(100) {
    if i == 5 { break }
    if i % 2 == 0 { continue }
    writ(i)
}
```

### Functions

```dgm
# Function definition
fn add(a, b) {
    return a + b
}

# Default usage
let result = add(3, 4)    # 7

# Lambda / anonymous function
let square = lam(x) => x * x
writ(square(5))    # 25

# Higher-order functions
let nums = [1, 2, 3, 4, 5]
let doubled = map(nums, lam(x) => x * 2)
let evens   = filter(nums, lam(x) => x % 2 == 0)
let total   = reduce(nums, 0, lam(acc, x) => acc + x)

# Closures
fn make_counter() {
    let count = 0
    return lam() => {
        count += 1
        return count
    }
}
let counter = make_counter()
writ(counter())    # 1
writ(counter())    # 2
```

### Classes

```dgm
class Animal {
    fn init(name, sound) {
        this.name = name
        this.sound = sound
    }

    fn speak() {
        writ(f"{this.name} says {this.sound}!")
    }

    fn to_str() {
        return f"Animal({this.name})"
    }
}

# Inheritance
class Dog extends Animal {
    fn init(name) {
        this.name = name
        this.sound = "Woof"
    }

    fn fetch(item) {
        writ(f"{this.name} fetches the {item}!")
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
} catch (err) {
    writ(f"Error caught: {err}")
} finally {
    writ("Always runs")
}

# Throw an error
fn divide(a, b) {
    if b == 0 {
        throw "Division by zero"
    }
    return a / b
}
```

### Pattern Matching

```dgm
let status = 404

match status {
    200 => { writ("OK") }
    404 => { writ("Not Found") }
    500 => { writ("Server Error") }
    _ => { writ(f"Unknown: {status}") }
}
```

---

## Standard Library

Use `import <module>` to load a module:

```dgm
import json
let data = json.parse("{\"key\": \"value\"}")
```

### math

```dgm
import math
import math as m
math.sin(math.PI / 2)       # 1.0
math.sqrt(16)               # 4.0
m.sqrt(9)                   # 3.0
math.pow(2, 10)             # 1024
math.random()               # 0.0..1.0
math.floor(3.7)             # 3
math.ceil(3.2)              # 4
math.log(math.E)            # 1.0
```

### io

```dgm
import io
let content = io.read_file("data.txt")
io.write_file("output.txt", "Hello!")
io.append_file("log.txt", "new line\n")
let lines = io.read_lines("data.txt")
let files = io.list_dir("./src")
io.mkdir("new_folder")
io.rename("old.txt", "new.txt")
io.copy("src.txt", "dst.txt")
io.delete("file.txt")
let exists = io.exists("file.txt")    # true / false
let size   = io.file_size("file.txt") # bytes
let path   = io.abs_path("./relative")
let cwd    = io.cwd()
```

### fs — Sandboxed Filesystem

```dgm
import fs
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
import os
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
import json
let parsed = json.parse("{\"name\":\"Alice\",\"age\":30}")
let str    = json.stringify(parsed)         # fast byte-level encode
let pretty = json.pretty(parsed)            # indented
let resp   = json.raw_parts("users", data) # {"ok":true,"users":<data>}
```

### http

```dgm
import http

# HTTP client
let res = http.get("https://api.example.com/users")
writ(res.status)     # 200
writ(res.body)       # response body string
writ(res.ok)         # true

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
import crypto
let hash     = crypto.sha256("hello world")
let md5hash  = crypto.md5("hello world")
let encoded  = crypto.base64_encode("binary data")
let decoded  = crypto.base64_decode(encoded)
let hmac     = crypto.hmac_sha256("secret", "message")
let randbytes = crypto.random_bytes(32)    # list[int]
```

### regex

```dgm
import regex
let match  = regex.test("[0-9]+", "abc123def")     # true
let found  = regex.find("[0-9]+", "abc123def")     # "123"
let all    = regex.find_all("[0-9]+", "a1b2c3")    # ["1","2","3"]
let groups = regex.match("(\\w+)@(\\w+)", "user@host") # list
let rep    = regex.replace("[aeiou]", "hello", "*")    # "h*ll*"
```

### net

```dgm
import net
let sock = net.connect("127.0.0.1", 9000)
net.send(sock, "Hello, server!")
let data = net.recv(sock, 4096)
net.close(sock)

let server = net.listen("0.0.0.0", 9000)
```

### time

```dgm
import time
let ts_sec = time.now()                              # unix seconds
let ts_ms  = time.now_ms()                           # unix milliseconds
let fmt    = time.format(ts_sec, "%Y-%m-%d %H:%M:%S")
let parsed = time.parse("2026-04-05 19:00:00", "%Y-%m-%d %H:%M:%S")
let delta  = time.elapsed(ts_ms)
```

### thread

```dgm
import thread
let cpus = thread.available_cpus()
thread.sleep(500)
writ(cpus)
```

### xml

```dgm
import xml
let doc = xml.parse("<root><item>hello</item></root>")
let str = xml.stringify(doc)
let val = xml.query(doc, "root.item")
writ(val.text)
```

### security

```dgm
import security

# Configure runtime security policy
security.configure({
    "allow_fs":       true,
    "allow_exec":     false,
    "allow_net":      false,
    "sandbox_root":   "/app/data",
    "allowed_hosts":  ["api.example.com", "cdn.trusted.io"],
    "allowed_programs": ["git", "node"],
    "max_http_body_bytes": 1048576
})

# Check current config
let status = security.status()
writ(status.allow_exec)     # false
writ(status.sandbox_root)   # /app/data
```

---

## Security Model

DGM provides a **thread-local security configuration** with no global mutex overhead.

### Controls

| Setting | Type | Default | Effect |
|---|---|---|---|
| `allow_fs` | bool | `true` | Gates all `fs.*` operations |
| `allow_exec` | bool | `false` | Gates `os.exec`, `os.spawn`, `os.run`, `os.run_timeout` |
| `allow_net` | bool | `false` | Gates `net.*` and `http.*` operations |
| `sandbox_root` | str \| null | `null` | Restricts `fs.*` to a directory subtree |
| `allowed_hosts` | list \| null | `null` | Restricts `net.*` to specific hosts |
| `allowed_programs` | list \| null | `null` | Restricts `os.run*` to an allowlist and disables shell exec APIs |
| `max_http_body_bytes` | int | `1048576` | Caps HTTP response bodies before parsing |

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
    │  ├── call_callable() — unified callable dispatch
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

# Rebuild generated docs from executable examples
node ../scripts/build_docs.js

# Check docs are in sync (same command CI uses)
node ../scripts/build_docs.js --check
```

Test suites are organized around language contracts instead of ad-hoc unit checks:

- `src/*` unit tests cover lexer aliases, security policy, stdlib behavior, and XML traversal.
- [`../tests/conformance`](/home/danggiaminh/Downloads/dgm-source/tests/conformance) locks lexer, parser, runtime, import, and error behavior behind versioned snapshots.
- [`../tests/golden`](/home/danggiaminh/Downloads/dgm-source/tests/golden) captures end-to-end scenarios for syntax, control flow, OOP, modules, and runtime failures.
- [`../tests/examples`](/home/danggiaminh/Downloads/dgm-source/tests/examples) are executable documentation examples and fail CI when docs drift from the runtime.

Documentation workflow:
- [`README.template.md`](/home/danggiaminh/Downloads/dgm-source/dgm/README.template.md) is the editable source for this README.
- [`../docs/manifest.json`](/home/danggiaminh/Downloads/dgm-source/docs/manifest.json) drives generated CLI/module sections across docs.
- [`../scripts/build_docs.js`](/home/danggiaminh/Downloads/dgm-source/scripts/build_docs.js) regenerates checked-in README content.
- `.github/workflows/ci.yml` rejects stale generated docs with `node scripts/build_docs.js --check`.

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**.

Copyright 2026 Đặng Gia Minh

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

See the full license text at: ../LICENSE

---

<div align="center">

Interpreter: Rust · Created by **Đặng Gia Minh** · GPL-3.0

</div>
