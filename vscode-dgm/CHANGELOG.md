# Changelog

All notable changes to the DGM VSCode Extension are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-04-05

### Added
- ✅ Syntax highlighting for DGM language
- ✅ Language configuration (auto-closing pairs, indentation rules)
- ✅ File association for `.dgm` extension
- ✅ 25+ code snippets for common patterns
- ✅ Run DGM File command (`dgm.run`)
- ✅ Show DGM Version command (`dgm.version`)
- ✅ Support for all keywords (frozen @ v0.2.0)
- ✅ Support for f-string interpolation
- ✅ Support for numbers, strings, booleans, null
- ✅ Support for operators and comments

### Status
- **Language**: Stable (v0.2.0)
- **Extension**: MVP Release
- **Keywords**: Frozen (no additions without major bump)
- **Modules**: All 13 stdlib modules documented

---

## Standards

### File Format
- All DGM scripts must use `.dgm` extension
- UTF-8 encoding recommended

### Supported Versions
- VSCode: 1.85.0 or later
- DGM: 0.2.0 or later

### Known Limitations
- No real-time type checking (language is dynamically typed)
- No built-in debugger (use print statements)
- HTTP module requires network access
- Security sandbox available for filesystem operations
