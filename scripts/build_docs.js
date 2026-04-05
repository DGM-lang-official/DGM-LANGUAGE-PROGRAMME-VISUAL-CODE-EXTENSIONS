#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const manifest = JSON.parse(fs.readFileSync(path.join(repoRoot, 'docs', 'manifest.json'), 'utf8'));
const docs = [
    {
        source: path.join(repoRoot, 'dgm', 'README.template.md'),
        output: path.join(repoRoot, 'dgm', 'README.md')
    },
    {
        source: path.join(repoRoot, 'vscode-dgm', 'README.md'),
        output: path.join(repoRoot, 'vscode-dgm', 'README.md')
    }
];

const includePattern = /\{\{include:\s*([^}]+)\s*\}\}/g;
const markerPattern = /<!-- GENERATED:([A-Z0-9_]+):START -->([\s\S]*?)<!-- GENERATED:\1:END -->/g;
const checkOnly = process.argv.includes('--check');

const generators = {
    CLI_USAGE() {
        const body = manifest.cliCommands
            .map(({ description, command }) => `# ${description}\n${command}`)
            .join('\n\n');
        return `\`\`\`bash\n${body}\n\`\`\``;
    },
    VSCODE_COMMANDS() {
        return manifest.vscodeCommands
            .map(({ title, id, description }) => `- \`${title}\` (\`${id}\`) — ${description}`)
            .join('\n');
    },
    MODULE_BULLETS() {
        return manifest.modules
            .map(({ name, summary }) => `- **${name}** — ${summary}`)
            .join('\n');
    }
};

function fenceFor(filePath) {
    const ext = path.extname(filePath);
    if (ext === '.dgm') {
        return 'dgm';
    }
    if (ext === '.json') {
        return 'json';
    }
    if (ext === '.sh') {
        return 'bash';
    }
    return '';
}

function renderTemplate(templatePath) {
    const templateDir = path.dirname(templatePath);
    const source = fs.readFileSync(templatePath, 'utf8');

    const withIncludes = source.replace(includePattern, (_, includeTarget) => {
        const resolved = path.resolve(templateDir, includeTarget.trim());
        const content = fs.readFileSync(resolved, 'utf8').replace(/\s+$/, '');
        const fence = fenceFor(resolved);
        return `\`\`\`${fence}\n${content}\n\`\`\``;
    });

    return withIncludes.replace(markerPattern, (match, key) => {
        const generator = generators[key];
        if (!generator) {
            return match;
        }
        return `<!-- GENERATED:${key}:START -->\n${generator()}\n<!-- GENERATED:${key}:END -->`;
    });
}

let hasDiff = false;

for (const doc of docs) {
    const rendered = renderTemplate(doc.source);

    if (checkOnly) {
        const current = fs.readFileSync(doc.output, 'utf8');
        if (current !== rendered) {
            hasDiff = true;
            console.error(`out of date: ${path.relative(repoRoot, doc.output)}`);
        }
        continue;
    }

    fs.writeFileSync(doc.output, rendered);
    console.log(`updated ${path.relative(repoRoot, doc.output)}`);
}

if (checkOnly && hasDiff) {
    process.exit(1);
}
