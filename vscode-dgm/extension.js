const vscode = require('vscode');
const { execFile } = require('child_process');
const fs = require('fs/promises');
const os = require('os');
const path = require('path');
const { LanguageClient } = require('vscode-languageclient/node');

let client;
let outputChannel;

function getBinaryPath() {
    return vscode.workspace.getConfiguration('dgm').get('binaryPath', 'dgm');
}

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

function parseValidationSummary(stderr) {
    const firstLine = (stderr || '').trim().split(/\r?\n/, 1)[0];
    if (!firstLine) {
        return 'Validation failed';
    }
    return firstLine;
}

async function validateDocument(document, options = {}) {
    const { showSuccess = false, showFailurePopup = true } = options;

    if (!document || document.languageId !== 'dgm') {
        return;
    }

    const binaryPath = getBinaryPath();
    const cwd = document.isUntitled ? process.cwd() : path.dirname(document.fileName);
    const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'dgm-validate-'));
    const baseName = document.fileName.endsWith('.dgm') ? path.basename(document.fileName) : 'untitled.dgm';
    const tempPath = path.join(tempDir, baseName);

    try {
        await fs.writeFile(tempPath, document.getText(), 'utf8');
        const result = await execFileCapture(binaryPath, ['validate', tempPath], { cwd });

        if (result.error && result.error.code === 'ENOENT') {
            if (showFailurePopup) {
                vscode.window.showErrorMessage(`DGM binary not found: ${binaryPath}`);
            }
            return;
        }

        if (result.error) {
            if (showFailurePopup) {
                vscode.window.showErrorMessage(parseValidationSummary(result.stderr));
            }
            return;
        }

        if (showSuccess) {
            vscode.window.showInformationMessage('DGM validation passed');
        }
    } finally {
        await fs.rm(tempDir, { recursive: true, force: true });
    }
}

async function stopClient() {
    if (client) {
        const active = client;
        client = undefined;
        await active.stop();
    }
}

async function startClient(context) {
    await stopClient();

    const binaryPath = getBinaryPath();
    const serverOptions = {
        command: binaryPath,
        args: ['lsp'],
        options: {
            env: process.env
        }
    };

    client = new LanguageClient('dgmLanguageServer', 'DGM Language Server', serverOptions, {
        documentSelector: [
            { scheme: 'file', language: 'dgm' },
            { scheme: 'untitled', language: 'dgm' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.dgm')
        },
        outputChannel
    });

    context.subscriptions.push(client.start());

    try {
        await client.onReady();
    } catch (error) {
        vscode.window.showErrorMessage(`Failed to start DGM language server: ${error.message}`);
    }
}

function activate(context) {
    outputChannel = vscode.window.createOutputChannel('DGM Language Server');
    context.subscriptions.push(outputChannel);

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

        const terminal = vscode.window.createTerminal('DGM');
        terminal.sendText(`${getBinaryPath()} run "${filePath}"`);
        terminal.show();
    });

    const validateCommand = vscode.commands.registerCommand('dgm.validate', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        await validateDocument(editor.document, {
            showSuccess: true,
            showFailurePopup: true
        });
    });

    const versionCommand = vscode.commands.registerCommand('dgm.version', async () => {
        const result = await execFileCapture(getBinaryPath(), ['version']);
        if (result.error) {
            vscode.window.showErrorMessage(`DGM binary not found: ${getBinaryPath()}`);
            return;
        }
        vscode.window.showInformationMessage(result.stdout.trim());
    });

    const restartCommand = vscode.commands.registerCommand('dgm.restartLanguageServer', async () => {
        await startClient(context);
        vscode.window.showInformationMessage('DGM language server restarted');
    });

    const configWatcher = vscode.workspace.onDidChangeConfiguration(async (event) => {
        if (event.affectsConfiguration('dgm.binaryPath')) {
            await startClient(context);
        }
    });

    context.subscriptions.push(runCommand);
    context.subscriptions.push(validateCommand);
    context.subscriptions.push(versionCommand);
    context.subscriptions.push(restartCommand);
    context.subscriptions.push(configWatcher);

    void startClient(context);
}

async function deactivate() {
    await stopClient();
}

module.exports = {
    activate,
    deactivate
};
