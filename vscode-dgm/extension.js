const vscode = require('vscode');
const { execSync } = require('child_process');
const path = require('path');

/**
 * DGM VSCode Extension Entry Point
 * Provides language support, snippets, and run commands
 */

function activate(context) {
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

    // Command: Show DGM Version
    const versionCommand = vscode.commands.registerCommand('dgm.version', () => {
        try {
            const version = execSync('dgm version').toString().trim();
            vscode.window.showInformationMessage(version);
        } catch (err) {
            vscode.window.showErrorMessage('DGM not installed or not in PATH');
        }
    });

    context.subscriptions.push(runCommand);
    context.subscriptions.push(versionCommand);
}

function deactivate() {}

module.exports = {
    activate,
    deactivate
};
