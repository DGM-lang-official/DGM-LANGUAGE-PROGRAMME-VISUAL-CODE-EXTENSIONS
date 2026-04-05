# VSCode Extension Directory Structure

```
vscode-dgm/
├── package.json                    # Extension manifest
├── language-configuration.json      # Language config (brackets, comments, etc.)
├── extension.js                     # Main extension entry point
├── README.md                        # User documentation
├── CHANGELOG.md                     # Version history
├── .vscodeignore                    # Files to ignore when packaging
│
├── syntaxes/
│   └── dgm.tmLanguage.json         # TextMate grammar for syntax highlighting
│
├── snippets/
│   └── dgm.json                     # Code snippets (25+ templates)
│
└── LICENSE                          # GPL-3.0 license file
```

## Files Reference

| File | Purpose | Size |
|------|---------|------|
| `package.json` | Extension metadata, commands, language registration | ~1KB |
| `language-configuration.json` | Editor behavior (folding, indentation, pairs) | ~800B |
| `extension.js` | Command handlers and activation logic | ~1.5KB |
| `syntaxes/dgm.tmLanguage.json` | Syntax highlighting rules (TextMate format) | ~6.5KB |
| `snippets/dgm.json` | 25+ predefined code templates | ~4.5KB |
| `README.md` | User documentation | ~8KB |
| `CHANGELOG.md` | Version history | ~1KB |

## Publishing Steps

1. Install dependencies:
   ```bash
   npm install -g vsce
   ```

2. Package extension:
   ```bash
   cd vscode-dgm
   vsce package
   ```

3. Creates: `dgm-0.2.0.vsix`

4. Publish to marketplace (requires Microsoft account):
   ```bash
   vsce publish
   ```

5. Or share directly: Users can install `.vsix` file via:
   - Extensions → "..." → Install from VSIX

## Development

### Testing locally
1. Open extension folder in VSCode
2. Press F5 to open Extension Development Host
3. Create test file: `test.dgm`
4. Test syntax highlighting and commands

### Modifying syntax
- Edit `syntaxes/dgm.tmLanguage.json`
- Reload extension (F5 in dev host)

### Adding snippets
- Edit `snippets/dgm.json`
- Add new entry with prefix and body
- Reload to test

### Changing commands
- Edit `package.json` (contributes section)
- Edit `extension.js` (handlers)
- Test with F5

## Constraints

- **VSCode API Version**: 1.85.0 minimum
- **Node.js**: Required for extension runtime
- **Language**: JavaScript (no TypeScript modules)
- **External Runtime**: Requires DGM binary in PATH

## Extension API Usage

- `vscode.commands.registerCommand()` — Register commands
- `vscode.window.activeTextEditor` — Get active editor
- `vscode.window.createTerminal()` — Create terminal for dgm run
- `vscode.window.showErrorMessage()` — Show UI errors
- `vscode.window.showInformationMessage()` — Show UI info

See VSCode Extension API docs: https://code.visualstudio.com/api
