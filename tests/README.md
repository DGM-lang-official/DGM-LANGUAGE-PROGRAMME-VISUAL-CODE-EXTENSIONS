# Test Fixtures

This directory is the language behavior source of truth.

## Layout

`conformance/`
- `lexer/` token snapshots from `input.dgm`
- `parser/` AST snapshots from `input.dgm`
- `runtime/` stdout/stderr snapshots from `input.dgm`
- `errors/` failure-format snapshots from `input.dgm`

`golden/`
- End-to-end runtime snapshots for core language scenarios and module interactions

`examples/`
- Executable documentation examples reused by README and CI-facing tests
- Generated docs are built from `dgm/README.template.md` via `scripts/build_docs.js`

## Fixture Format

Each fixture lives in its own folder and includes:

- `input.dgm`
- `expected.json`
- Optional `command.txt` when a fixture should run with `validate` instead of the default `run`

`expected.json` stores the behavior snapshot for that fixture:

```json
{
  "version": 1,
  "input": "...",
  "tokens": null,
  "ast": null,
  "status": 0,
  "stdout": "",
  "stderr": "",
  "error": null
}
```

Conformance fixtures fill either `tokens`, `ast`, or runtime/error fields depending on the suite.

This structure keeps runtime, docs, tooling, and future diagnostics aligned to the same executable behavior.
