# DGM Source

Monorepo for the **DGM programming language**, its Rust runtime, tests, documentation, and VS Code extension.

## What Is Here

- [`dgm/`](/home/danggiaminh/Downloads/dgm-source/dgm) — Rust interpreter, semantic analyzer, formatter, and built-in LSP server
- [`vscode-dgm/`](/home/danggiaminh/Downloads/dgm-source/vscode-dgm) — VS Code extension backed by `dgm lsp`
- [`tests/`](/home/danggiaminh/Downloads/dgm-source/tests) — conformance, golden, and executable example fixtures
- [`docs/manifest.json`](/home/danggiaminh/Downloads/dgm-source/docs/manifest.json) — shared metadata for generated docs
- [`scripts/build_docs.js`](/home/danggiaminh/Downloads/dgm-source/scripts/build_docs.js) — README generation script used locally and in CI

## Current Highlights

- Canonical public syntax with backward-compatible legacy keyword aliases
- Tree-walk runtime in Rust
- Semantic `validate` pass
- Rust LSP server with diagnostics, hover, completion, definition, references, rename, and formatting
- Sandboxed filesystem, exec, HTTP, TCP, XML, and security configuration support

## CLI

<!-- GENERATED:CLI_USAGE:START -->
```bash
# Run a DGM script
dgm run script.dgm

# Validate syntax and semantics without executing
dgm validate script.dgm

# Start the language server
dgm lsp

# Start interactive REPL
dgm repl

# Show version
dgm version

# Show help
dgm help
```
<!-- GENERATED:CLI_USAGE:END -->

## VS Code Commands

<!-- GENERATED:VSCODE_COMMANDS:START -->
- `Run DGM File` (`dgm.run`) — Execute current `.dgm` file
- `Validate DGM File` (`dgm.validate`) — Validate syntax and semantics and surface diagnostics
- `Show DGM Version` (`dgm.version`) — Display installed DGM version
- `Restart DGM Language Server` (`dgm.restartLanguageServer`) — Restart the DGM LSP client and server
<!-- GENERATED:VSCODE_COMMANDS:END -->

## Standard Library Snapshot

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

## Quick Start

### Runtime

```bash
cd dgm
cargo build --release
./target/release/dgm version
```

### Extension

```bash
cd vscode-dgm
npm install
npx @vscode/vsce package
```

## Documentation Workflow

- Root [`README.md`](/home/danggiaminh/Downloads/dgm-source/README.md), [`dgm/README.md`](/home/danggiaminh/Downloads/dgm-source/dgm/README.md), and generated command/module sections are derived from checked-in templates and [`docs/manifest.json`](/home/danggiaminh/Downloads/dgm-source/docs/manifest.json).
- Regenerate docs with:

```bash
node scripts/build_docs.js
```

- Check docs are current with the same command CI uses:

```bash
node scripts/build_docs.js --check
```

## CI / Release Workflow

- CI runs Rust tests from [`dgm/`](/home/danggiaminh/Downloads/dgm-source/dgm), checks generated docs, installs extension dependencies, validates extension metadata, and packages the VS Code extension.
- Release packaging for the extension must keep runtime dependencies bundled, so use `npm install` and package without `--no-dependencies`.

## Main References

- [`dgm/README.md`](/home/danggiaminh/Downloads/dgm-source/dgm/README.md)
- [`vscode-dgm/README.md`](/home/danggiaminh/Downloads/dgm-source/vscode-dgm/README.md)
- [`LANGUAGE_SPEC.md`](/home/danggiaminh/Downloads/dgm-source/LANGUAGE_SPEC.md)
- [`STDLIB_SPEC.md`](/home/danggiaminh/Downloads/dgm-source/STDLIB_SPEC.md)
