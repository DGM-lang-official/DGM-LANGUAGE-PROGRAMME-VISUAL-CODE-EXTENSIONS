# DGM Language Support for VSCode

Official VSCode extension for the **DGM Programming Language** — a dynamically typed, interpreted language written in Rust.

## Features

**Syntax Highlighting**
- Keywords, operators, strings, numbers, comments
- F-string interpolation support
- Function definitions and calls

**Editor Navigation**
- Static hover for common builtins, module calls, and DGM error codes
- Same-file go-to-definition for `def`, `cls`, and `let` bindings
- Document symbols for functions and classes

**Language Configuration**
- Auto-closing brackets and quotes
- Comment formatting
- Indentation rules
- Multi-line support

**Code Snippets**
- Common language constructs (if, while, for, functions)
- HTTP requests and server setup
- JSON parsing and serialization
- Error handling patterns

**File Association**
- Automatically associates `.dgm` files with DGM language

**Commands**
<!-- GENERATED:VSCODE_COMMANDS:START -->
- `Run DGM File` (`dgm.run`) — Execute current `.dgm` file
- `Validate DGM File` (`dgm.validate`) — Validate syntax and surface diagnostics
- `Show DGM Version` (`dgm.version`) — Display installed DGM version
<!-- GENERATED:VSCODE_COMMANDS:END -->

## Installation

### From VSCode Extensions Marketplace
1. Open VSCode
2. Go to Extensions (`Ctrl+Shift+X` / `Cmd+Shift+X`)
3. Search for "DGM" or "dgm-lang"
4. Click Install

### Manual Installation
```bash
git clone https://github.com/danggiaminh/dgm-source.git
cd vscode-dgm
npx @vscode/vsce package --no-dependencies
# Install the .vsix file in VSCode
```

## Prerequisites

- VSCode 1.85.0 or later
- DGM runtime installed (for `Run DGM File` command)
  ```bash
  cargo install dgm
  # or build from source
  cd ../dgm
  cargo build --release
  ```

## Quick Start

1. Create a new file: `hello.dgm`
   ```dgm
   let name = "World"
   writ(f"Hello, {name}!")
   ```

2. Run the file:
   - Use command palette: `DGM: Run DGM File`
   - Or terminal: `dgm run hello.dgm`

3. Validate the file:
   - Use command palette: `DGM: Validate DGM File`
   - Opening a `.dgm` file triggers validation automatically
   - Editing a `.dgm` file refreshes inline diagnostics with a short debounce
   - Saving a `.dgm` file also forces a fresh validation pass
   - Or terminal: `dgm validate hello.dgm`

4. Navigate the file:
   - Hover on builtins like `map`, `json.parse`, or `E100`
   - Use `Go to Definition` on same-file `def`, `cls`, and `let` bindings
   - Open `Outline` / `Go to Symbol in Editor` for function and class symbols

## Syntax Example

```dgm
# Comments start with #

# Variables
let x = 42
let s = "hello"
let flag = tru  # alias: true

# Functions
def add(a, b) {
  retrun a + b
}

# if/else
iff (x > 0) {
  writ("positive")
} els {
  writ("non-positive")
}

# Loops
fr i in [1, 2, 3] {
  writ(i)
}

# Import modules
imprt "http"
imprt "json"

# HTTP request
let response = http.get("https://example.com")

# F-strings
writ(f"x = {x}, s = {s}")

# Error handling
try {
  let data = json.parse(bad_json)
} catch (err) {
  writ(f"Error: {err}")
}
```

## Available Modules

<!-- GENERATED:MODULE_BULLETS:START -->
- **math** — sqrt, sin, cos, tan, random, ceil, floor, abs, min, max, pow, log
- **io** — read_file, write_file, append_file, list_dir, mkdir, rename, copy
- **fs** — read, write, append, delete, list, exists (sandboxed)
- **os** — exec, spawn, run, run_timeout, env, platform, cwd, chdir
- **json** — parse, stringify
- **http** — get, post, request, serve
- **crypto** — sha256, md5, base64_encode, base64_decode, random_bytes
- **regex** — match, split, replace, find_all
- **net** — tcp_connect, tcp_listen
- **time** — now, now_ms, format, parse, elapsed
- **thread** — sleep, available_cpus
- **xml** — parse, stringify, query
<!-- GENERATED:MODULE_BULLETS:END -->

## Language Features

### Keywords (Stable @ v0.2.0)
`let`, `def`, `cls`, `new`, `ths`, `in`, `imprt`, `writ`, `iff`, `elseif`, `els`, `fr`, `whl`, `brk`, `cont`, `retrun`, `try`, `catch`, `finally`, `throw`, `match`, `and`, `or`, `not`, `tru`, `fals`, `nul`, `extends`, `lam`

Canonical literals remain `tru`, `fals`, `nul`. Compatibility aliases `true`, `false`, `null` are also accepted.

### Operators
Arithmetic: `+`, `-`, `*`, `/`, `%`, `**`  
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`  
Logical: `and`, `or`, `not`  
Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`  
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`

### Data Types
- **Boolean**: `tru`, `fals` with `true`, `false` aliases
- **Null**: `nul` with `null` alias
- **Number**: `42`, `3.14`
- **String**: `"hello"`
- **Array**: `[1, 2, 3]`
- **Map**: `{"key": value}`

## Troubleshooting

### "DGM not found" when running files
Ensure DGM is installed and in your PATH:
```bash
dgm version
```

If not installed, build from source:
```bash
cd /path/to/dgm-source/dgm
cargo build --release
export PATH="./target/release:$PATH"
```

### Syntax highlighting not working
1. Make sure file has `.dgm` extension
2. Reload VSCode window: `Ctrl+Shift+P` → "Developer: Reload Window"

### Go-to-definition did not jump
Navigation is intentionally lightweight in v0.2.0:
1. Definitions are same-file only
2. Regex scanning covers `def`, `cls`, and `let`
3. Cross-file symbol resolution requires future LSP work

### Snippets not showing
1. Start typing a snippet prefix (e.g., "def", "iff", "lam")
2. Snippets appear in autocomplete menu
3. Press Tab to expand

## Documentation

- [Language Specification](../LANGUAGE_SPEC.md) — Frozen syntax & keywords
- [Standard Library](../STDLIB_SPEC.md) — Module reference
- [Project README](../dgm/README.md) — Full project documentation
- [DGM Homepage](https://github.com/danggiaminh/dgm-source)

## Contributing

Issues and pull requests welcome:
- [GitHub Issues](https://github.com/danggiaminh/dgm-source/issues)
- [GitHub Repository](https://github.com/danggiaminh/dgm-source)

## License

GNU General Public License v3.0 (GPL-3.0) — See [LICENSE](./LICENSE)

## Credits

- **Language Author**: Đặng Gia Minh
- **Implementation**: Rust (hand-written lexer, parser, tree-walk interpreter)
- **VSCode Extension**: DGM Lang Contributors

---

**Current Version**: 0.2.0 (Alpha)  
**Language Status**: Stable ✓  
**Last Updated**: 2026-04-05
