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
<!-- GENERATED:CLI_USAGE:END -->

## VS Code Commands

<!-- GENERATED:VSCODE_COMMANDS:START -->
<!-- GENERATED:VSCODE_COMMANDS:END -->

## Standard Library Snapshot

<!-- GENERATED:MODULE_BULLETS:START -->
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
