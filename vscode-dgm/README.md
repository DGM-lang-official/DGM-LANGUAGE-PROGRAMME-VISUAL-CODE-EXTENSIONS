# DGM Language Support for VSCode

Official VSCode extension for the **DGM Programming Language** — a dynamically typed, interpreted language written in Rust.

## Features

**Syntax Highlighting**
- Keywords, operators, strings, numbers, comments
- F-string interpolation support
- Function definitions and calls

**Editor Navigation**
- Rust-backed LSP hover for builtins, local symbols, and imported module members
- Cross-file go-to-definition for imported module exports
- Find references, rename symbol, document symbols, and document formatting

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
- `Validate DGM File` (`dgm.validate`) — Validate syntax and semantics and surface diagnostics
- `Show DGM Version` (`dgm.version`) — Display installed DGM version
- `Restart DGM Language Server` (`dgm.restartLanguageServer`) — Restart the DGM LSP client and server
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
npm install
npx @vscode/vsce package
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
   - Opening or editing a `.dgm` file refreshes LSP diagnostics automatically
   - Or terminal: `dgm validate hello.dgm`

4. Navigate the file:
   - Hover on builtins like `map`, `json.parse`, or imported members like `helper.answer`
   - Use `Go to Definition`, `Find References`, and `Rename Symbol` across module boundaries
   - Format the current document through the DGM formatter

## Syntax Example

```dgm
# Comments start with #

# Variables
let x = 42
let s = "hello"
let flag = true  # legacy alias: tru

# Functions
fn add(a, b) {
  return a + b
}

# if/else
if (x > 0) {
  writ("positive")
} else {
  writ("non-positive")
}

# Loops
for i in [1, 2, 3] {
  writ(i)
}

# Import modules
import "http"
import "json"

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

### Canonical Keywords
`let`, `fn`, `class`, `new`, `this`, `in`, `import`, `writ`, `if`, `elseif`, `else`, `for`, `while`, `break`, `continue`, `return`, `try`, `catch`, `finally`, `throw`, `match`, `and`, `or`, `not`, `true`, `false`, `null`, `extends`, `lam`

Legacy aliases such as `def`, `imprt`, `retrun`, `iff`, `els`, `fr`, `whl`, `brk`, `cont`, `cls`, `ths`, `tru`, `fals`, and `nul` are still accepted.

### Operators
Arithmetic: `+`, `-`, `*`, `/`, `%`, `**`  
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`  
Logical: `and`, `or`, `not`  
Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`  
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`

### Data Types
- **Boolean**: `true`, `false` with legacy `tru`, `fals` aliases
- **Null**: `null` with legacy `nul` alias
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

If the binary lives outside `PATH`, set `dgm.binaryPath` in VS Code settings.

If not installed, build from source:
```bash
cd /path/to/dgm-source/dgm
cargo build --release
export PATH="./target/release:$PATH"
```

### Syntax highlighting not working
1. Make sure file has `.dgm` extension
2. Reload VSCode window: `Ctrl+Shift+P` → "Developer: Reload Window"

### LSP features are not starting
1. Confirm `dgm lsp` runs from the terminal
2. Check the `DGM Language Server` output panel in VS Code
3. Verify `dgm.binaryPath` points to the DGM executable you built or installed

### Snippets not showing
1. Start typing a snippet prefix (e.g., "fn", "def", "if", "lam")
2. Snippets appear in autocomplete menu
3. Press Tab to expand

## README / Packaging Workflow

- This README is partially generated from [`../docs/manifest.json`](/home/danggiaminh/Downloads/dgm-source/docs/manifest.json) by [`../scripts/build_docs.js`](/home/danggiaminh/Downloads/dgm-source/scripts/build_docs.js).
- After changing command lists or module summaries, run `node ../scripts/build_docs.js`.
- CI checks generated docs with `node scripts/build_docs.js --check`.
- Package with `npm install && npx @vscode/vsce package` so the LSP client dependency is bundled into the VSIX.

## Documentation

- [Language Specification](../LANGUAGE_SPEC.md) — Canonical syntax and compatibility aliases
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

**Current Version**: 0.3.0 (Alpha)  
**Language Status**: Stable ✓  
**Last Updated**: 2026-04-08
