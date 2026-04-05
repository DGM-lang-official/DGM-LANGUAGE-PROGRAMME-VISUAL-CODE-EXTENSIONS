const vscode = require('vscode');
const { execSync, execFile } = require('child_process');
const fs = require('fs/promises');
const os = require('os');
const path = require('path');
const VALIDATION_DEBOUNCE_MS = 400;
const DGM_SELECTOR = { language: 'dgm' };
const SYMBOL_CHARS = /[A-Za-z0-9_.]/;
const ERROR_CODE_DOCS = new Map([
    ['E001', 'Tokenization failure while scanning source text.'],
    ['E100', 'Undefined variable access at runtime.'],
    ['E101', 'Invalid callable usage or wrong call target.'],
    ['E102', 'Division or modulo by zero.'],
    ['E103', 'Invalid index or missing keyed access.'],
    ['E199', 'Generic runtime error raised by the interpreter or stdlib.'],
    ['E200', 'Module import or loading failure.'],
    ['E201', 'Circular import detected while loading modules.'],
    ['E300', 'General parser failure.'],
    ['E301', 'Unexpected token while parsing.'],
    ['E302', 'Expected token missing while parsing.'],
    ['E400', 'User-thrown value that escaped to the top level.']
]);
const STATIC_HOVERS = new Map([
    ['writ', { signature: 'writ(value)', description: 'Print a value to stdout.' }],
    ['len', { signature: 'len(value)', description: 'Return the length of a list, string, or map.' }],
    ['type', { signature: 'type(value)', description: 'Return the runtime type name for a value.' }],
    ['str', { signature: 'str(value)', description: 'Convert a value to its string form.' }],
    ['int', { signature: 'int(value)', description: 'Convert a value to an integer.' }],
    ['float', { signature: 'float(value)', description: 'Convert a value to a float.' }],
    ['range', { signature: 'range(end) | range(start, end)', description: 'Create a list of integers in sequence.' }],
    ['push', { signature: 'push(list, value)', description: 'Append a value to a list.' }],
    ['pop', { signature: 'pop(list)', description: 'Remove and return the last list item.' }],
    ['map', { signature: 'map(list, fn)', description: 'Apply a callback to each list item and return a new list.' }],
    ['filter', { signature: 'filter(list, fn)', description: 'Keep items whose callback returns `tru`.' }],
    ['reduce', { signature: 'reduce(list, init, fn)', description: 'Fold a list into a single value.' }],
    ['each', { signature: 'each(list, fn)', description: 'Run a callback for each list item.' }],
    ['find', { signature: 'find(list, fn)', description: 'Return the first item whose callback returns `tru`.' }],
    ['any', { signature: 'any(list, fn?)', description: 'Return `tru` if any item is truthy or matches the callback.' }],
    ['all', { signature: 'all(list, fn?)', description: 'Return `tru` if every item is truthy or matches the callback.' }],
    ['json.parse', { signature: 'json.parse(string)', description: 'Parse a JSON string into DGM values.' }],
    ['json.stringify', { signature: 'json.stringify(value)', description: 'Serialize a DGM value as JSON.' }],
    ['http.get', { signature: 'http.get(url)', description: 'Perform an HTTP GET request.' }],
    ['http.post', { signature: 'http.post(url, data)', description: 'Perform an HTTP POST request.' }],
    ['http.serve', { signature: 'http.serve(port, handler)', description: 'Start a simple HTTP server.' }],
    ['math.sqrt', { signature: 'math.sqrt(n)', description: 'Return the square root of a number.' }],
    ['math.pow', { signature: 'math.pow(base, exp)', description: 'Raise a number to a power.' }],
    ['regex.match', { signature: 'regex.match(pattern, text)', description: 'Check whether a regex matches text.' }],
    ['regex.find_all', { signature: 'regex.find_all(pattern, text)', description: 'Return all matches for a regex pattern.' }],
    ['xml.parse', { signature: 'xml.parse(string)', description: 'Parse XML into the DGM XML node map shape.' }],
    ['xml.query', { signature: 'xml.query(node, path)', description: 'Find a nested XML child node by dotted path.' }]
]);

/**
 * DGM VSCode Extension Entry Point
 * Provides language support, snippets, and run commands
 */

function execFileCapture(file, args, options = {}) {
    return new Promise((resolve) => {
        execFile(file, args, { ...options, encoding: 'utf8' }, (error, stdout, stderr) => {
            resolve({
                error,
                stdout: stdout || '',
                stderr: stderr || ''
            });
        });
    });
}

function parseErrorReport(stderr) {
    const lines = (stderr || '').trimEnd().split(/\r?\n/);
    const header = lines[0] || '';
    const headerMatch = header.match(/^\[(E\d{3})\] (.+)$/);

    if (!headerMatch) {
        return null;
    }

    const spanLine = lines.find((line) => /^\s*--> /.test(line)) || '';
    const spanMatch = spanLine.match(/^\s*--> .+:(\d+):(\d+)$/);

    return {
        code: headerMatch[1],
        message: headerMatch[2],
        span: spanMatch
            ? {
                  line: Number(spanMatch[1]),
                  col: Number(spanMatch[2])
              }
            : null
    };
}

function buildDiagnostic(document, stderr) {
    const parsed = parseErrorReport(stderr);
    const message = parsed ? `[${parsed.code}] ${parsed.message}` : ((stderr || '').trim().split(/\r?\n/, 1)[0] || 'Validation failed');

    let range = new vscode.Range(0, 0, 0, 0);
    if (parsed?.span) {
        const lineIndex = Math.max(parsed.span.line - 1, 0);
        const safeLine = Math.min(lineIndex, Math.max(document.lineCount - 1, 0));
        const lineText = document.lineAt(safeLine).text;
        const columnIndex = Math.max(parsed.span.col - 1, 0);
        const safeColumn = Math.min(columnIndex, lineText.length);
        const endColumn = safeColumn < lineText.length ? safeColumn + 1 : lineText.length;
        range = new vscode.Range(safeLine, safeColumn, safeLine, endColumn);
    }

    const diagnostic = new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
    if (parsed?.code) {
        diagnostic.code = parsed.code;
    }
    diagnostic.source = 'dgm';
    return diagnostic;
}

async function validateDocument(document, diagnostics, options = {}) {
    const { showSuccess = false, showFailurePopup = true } = options;

    if (!document || document.languageId !== 'dgm') {
        return;
    }

    if (!document.isUntitled && !document.fileName.endsWith('.dgm')) {
        if (showFailurePopup) {
            vscode.window.showErrorMessage('File must have .dgm extension');
        }
        return;
    }

    const cwd = document.isUntitled ? process.cwd() : path.dirname(document.fileName);
    const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'dgm-validate-'));
    const baseName = document.fileName.endsWith('.dgm') ? path.basename(document.fileName) : 'untitled.dgm';
    const tempPath = path.join(tempDir, baseName);

    try {
        await fs.writeFile(tempPath, document.getText(), 'utf8');
        const result = await execFileCapture('dgm', ['validate', tempPath], { cwd });

        if (result.error && result.error.code === 'ENOENT') {
            if (showFailurePopup) {
                vscode.window.showErrorMessage('DGM not installed or not in PATH');
            }
            return;
        }

        if (result.error) {
            const diagnostic = buildDiagnostic(document, result.stderr);
            diagnostics.set(document.uri, [diagnostic]);
            if (showFailurePopup) {
                vscode.window.showErrorMessage(diagnostic.message);
            }
            return;
        }

        diagnostics.delete(document.uri);
        if (showSuccess) {
            vscode.window.showInformationMessage('DGM validation passed');
        }
    } finally {
        await fs.rm(tempDir, { recursive: true, force: true });
    }
}

function scheduleValidation(document, diagnostics, pendingValidations) {
    if (!document || document.languageId !== 'dgm') {
        return;
    }

    const key = document.uri.toString();
    const existing = pendingValidations.get(key);
    if (existing) {
        clearTimeout(existing);
    }

    const timeout = setTimeout(async () => {
        pendingValidations.delete(key);
        await validateDocument(document, diagnostics, {
            showSuccess: false,
            showFailurePopup: false
        });
    }, VALIDATION_DEBOUNCE_MS);

    pendingValidations.set(key, timeout);
}

function getTokenAtPosition(document, position) {
    const line = document.lineAt(position.line).text;
    if (!line) {
        return null;
    }

    let start = position.character;
    while (start > 0 && SYMBOL_CHARS.test(line[start - 1])) {
        start -= 1;
    }

    let end = position.character;
    while (end < line.length && SYMBOL_CHARS.test(line[end])) {
        end += 1;
    }

    if (start === end) {
        return null;
    }

    return {
        text: line.slice(start, end),
        range: new vscode.Range(position.line, start, position.line, end)
    };
}

function addDefinition(index, definition) {
    const bucket = index.get(definition.name) || [];
    bucket.push(definition);
    index.set(definition.name, bucket);
}

function scanDocument(document) {
    const definitions = new Map();
    const symbols = [];

    for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber += 1) {
        const text = document.lineAt(lineNumber).text;

        const functionMatch = text.match(/^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)/);
        if (functionMatch) {
            const [, name, params] = functionMatch;
            const start = text.indexOf(name);
            const selectionRange = new vscode.Range(lineNumber, start, lineNumber, start + name.length);
            addDefinition(definitions, {
                name,
                kind: 'function',
                signature: `def ${name}(${params})`,
                selectionRange
            });
            symbols.push(
                new vscode.DocumentSymbol(
                    name,
                    params ? `(${params})` : '()',
                    vscode.SymbolKind.Function,
                    new vscode.Range(lineNumber, 0, lineNumber, text.length),
                    selectionRange
                )
            );
        }

        const classMatch = text.match(/^\s*cls\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+extends\s+([A-Za-z_][A-Za-z0-9_]*))?/);
        if (classMatch) {
            const [, name, parent] = classMatch;
            const start = text.indexOf(name);
            const selectionRange = new vscode.Range(lineNumber, start, lineNumber, start + name.length);
            addDefinition(definitions, {
                name,
                kind: 'class',
                signature: parent ? `cls ${name} extends ${parent}` : `cls ${name}`,
                selectionRange
            });
            symbols.push(
                new vscode.DocumentSymbol(
                    name,
                    parent ? `extends ${parent}` : '',
                    vscode.SymbolKind.Class,
                    new vscode.Range(lineNumber, 0, lineNumber, text.length),
                    selectionRange
                )
            );
        }

        const letMatch = text.match(/^\s*let\s+([A-Za-z_][A-Za-z0-9_]*)\s*=/);
        if (letMatch) {
            const [, name] = letMatch;
            const start = text.indexOf(name);
            addDefinition(definitions, {
                name,
                kind: 'variable',
                signature: `let ${name} = ...`,
                selectionRange: new vscode.Range(lineNumber, start, lineNumber, start + name.length)
            });
        }
    }

    return { definitions, symbols };
}

function createHover(range, { signature, description }) {
    const markdown = new vscode.MarkdownString();
    if (signature) {
        markdown.appendCodeblock(signature, 'dgm');
    }
    if (description) {
        markdown.appendMarkdown(description);
    }
    markdown.isTrusted = false;
    return new vscode.Hover(markdown, range);
}

function activate(context) {
    const diagnostics = vscode.languages.createDiagnosticCollection('dgm');
    const pendingValidations = new Map();

    // Command: Run DGM File
    const runCommand = vscode.commands.registerCommand('dgm.run', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const filePath = editor.document.fileName;
        if (!filePath.endsWith('.dgm')) {
            vscode.window.showErrorMessage('File must have .dgm extension');
            return;
        }

        try {
            const terminal = vscode.window.createTerminal('DGM');
            terminal.sendText(`dgm run "${filePath}"`);
            terminal.show();
        } catch (err) {
            vscode.window.showErrorMessage(`Error running DGM: ${err.message}`);
        }
    });

    const validateCommand = vscode.commands.registerCommand('dgm.validate', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        await validateDocument(editor.document, diagnostics, {
            showSuccess: true,
            showFailurePopup: true
        });
    });

    // Command: Show DGM Version
    const versionCommand = vscode.commands.registerCommand('dgm.version', () => {
        try {
            const version = execSync('dgm version').toString().trim();
            vscode.window.showInformationMessage(version);
        } catch (err) {
            vscode.window.showErrorMessage('DGM not installed or not in PATH');
        }
    });

    const closeDocument = vscode.workspace.onDidCloseTextDocument((document) => {
        const key = document.uri.toString();
        const existing = pendingValidations.get(key);
        if (existing) {
            clearTimeout(existing);
            pendingValidations.delete(key);
        }
        diagnostics.delete(document.uri);
    });

    const openDocument = vscode.workspace.onDidOpenTextDocument((document) => {
        scheduleValidation(document, diagnostics, pendingValidations);
    });

    const changeDocument = vscode.workspace.onDidChangeTextDocument((event) => {
        scheduleValidation(event.document, diagnostics, pendingValidations);
    });

    const saveDocument = vscode.workspace.onDidSaveTextDocument(async (document) => {
        await validateDocument(document, diagnostics, {
            showSuccess: false,
            showFailurePopup: false
        });
    });

    const hoverProvider = vscode.languages.registerHoverProvider(DGM_SELECTOR, {
        provideHover(document, position) {
            const token = getTokenAtPosition(document, position);
            if (!token) {
                return null;
            }

            if (ERROR_CODE_DOCS.has(token.text)) {
                return createHover(token.range, {
                    signature: token.text,
                    description: ERROR_CODE_DOCS.get(token.text)
                });
            }

            if (STATIC_HOVERS.has(token.text)) {
                return createHover(token.range, STATIC_HOVERS.get(token.text));
            }

            const symbolName = token.text.split('.').pop();
            const definition = scanDocument(document).definitions.get(symbolName)?.[0];
            if (!definition) {
                return null;
            }

            return createHover(token.range, {
                signature: definition.signature,
                description:
                    definition.kind === 'class'
                        ? 'Same-file class definition.'
                        : definition.kind === 'function'
                          ? 'Same-file function definition.'
                          : 'Same-file variable definition.'
            });
        }
    });

    const definitionProvider = vscode.languages.registerDefinitionProvider(DGM_SELECTOR, {
        provideDefinition(document, position) {
            const token = getTokenAtPosition(document, position);
            if (!token) {
                return null;
            }

            const symbolName = token.text.split('.').pop();
            const matches = scanDocument(document).definitions.get(symbolName) || [];
            const target = matches.find((definition) => !definition.selectionRange.contains(position));
            return target ? new vscode.Location(document.uri, target.selectionRange) : null;
        }
    });

    const symbolProvider = vscode.languages.registerDocumentSymbolProvider(DGM_SELECTOR, {
        provideDocumentSymbols(document) {
            return scanDocument(document).symbols;
        }
    });

    context.subscriptions.push({
        dispose() {
            for (const timeout of pendingValidations.values()) {
                clearTimeout(timeout);
            }
            pendingValidations.clear();
        }
    });

    context.subscriptions.push(diagnostics);
    context.subscriptions.push(runCommand);
    context.subscriptions.push(validateCommand);
    context.subscriptions.push(versionCommand);
    context.subscriptions.push(closeDocument);
    context.subscriptions.push(openDocument);
    context.subscriptions.push(changeDocument);
    context.subscriptions.push(saveDocument);
    context.subscriptions.push(hoverProvider);
    context.subscriptions.push(definitionProvider);
    context.subscriptions.push(symbolProvider);
}

function deactivate() {}

module.exports = {
    activate,
    deactivate
};
