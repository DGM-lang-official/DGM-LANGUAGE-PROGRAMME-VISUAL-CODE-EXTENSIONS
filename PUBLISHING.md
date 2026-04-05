# VSCode Extension Publishing Guide

## Prerequisites

1. **Node.js** (v14 or later)
   ```bash
   node --version  # should be v14+
   npm --version   # should be v6+
   ```

2. **VSCode Extension Build Tool**
   ```bash
   npm install -g @vscode/vsce
   vsce --version
   ```

3. **Git** (for repository operations)
   ```bash
   git --version
   ```

---

## Local Packaging

### Step 1: Install Dependencies
```bash
cd vscode-dgm
npm install
```

### Step 2: Package Extension
```bash
vsce package
```

Output: `dgm-0.2.0.vsix` (installable package)

### Step 3: Test Locally
1. Open VSCode
2. Go to Extensions → "..." → "Install from VSIX..."
3. Select `dgm-0.2.0.vsix`
4. Reload VSCode
5. Create test file: `test.dgm`
6. Verify:
   - Syntax highlighting works
   - Snippets appear in IntelliSense
   - Commands are accessible

---

## Publishing to VSCode Marketplace

### Prerequisites
- Microsoft account
- Personal Access Token (PAT) from Azure DevOps

### Step 1: Create Publisher Account

Visit: https://marketplace.visualstudio.com/manage/publishers

1. Sign in with Microsoft account
2. Click "Create Publisher"
3. Enter publisher ID (e.g., `dgm-lang`)
4. Complete profile

### Step 2: Create Personal Access Token (PAT)

1. Go to: https://dev.azure.com/
2. User Settings → Personal Access Tokens
3. New Token:
   - Name: "VSCode Extension Publish"
   - Organization: All accessible organizations
   - Scopes: Marketplace (Manage)
   - Expiration: 1 year
4. Copy token immediately (won't be shown again)

### Step 3: Login to Publisher
```bash
vsce login <publisher-id>
# Paste PAT when prompted
```

### Step 4: Publish Extension
```bash
cd vscode-dgm
vsce publish
```

This will:
- Package the extension
- Upload to marketplace
- Update publicly visible version

### Step 5: Verify Publication
- Visit: https://marketplace.visualstudio.com/items?itemName=<publisher-id>.dgm
- Should be visible within 5-10 minutes
- Install button available immediately

---

## Direct Distribution

### Option 1: GitHub Releases
1. Create GitHub release
2. Attach `.vsix` file
3. Users download and install via "Install from VSIX"

### Option 2: Direct Link
```bash
# Upload to web server or cloud storage
# Users download directly and install
```

### Option 3: Email/Chat
Send `.vsix` file directly to users.

---

## Version Updates

### For Next Release (e.g., 0.2.1)

1. **Update version in package.json**
   ```json
   "version": "0.2.1"
   ```

2. **Update CHANGELOG.md**
   ```markdown
   ## [0.2.1] - YYYY-MM-DD
   
   ### Fixed
   - Bug fix description
   
   ### Changed
   - Change description
   ```

3. **Commit changes**
   ```bash
   git add package.json CHANGELOG.md
   git commit -m "Version bump: 0.2.1"
   git tag -a v0.2.1 -m "VSCode Extension 0.2.1"
   git push --tags
   ```

4. **Republish**
   ```bash
   vsce publish
   # or
   vsce publish patch  # auto-increments patch version
   ```

---

## Troubleshooting

### "vsce not found"
```bash
npm install -g @vscode/vsce
```

### "Unable to find manifest"
Ensure `package.json` exists and is valid JSON:
```bash
cat package.json | jq .
```

### "Icon not found" warnings
Icons are optional. Ignore if not using custom icons.

### "Missing license" warnings
Create LICENSE symlink:
```bash
ln -s ../LICENSE LICENSE
```

### "Authentication failed"
- Verify PAT is still valid
- Try logging in again: `vsce login <publisher-id>`
- Create new PAT if expired

### Extension won't activate
- Ensure VSCode version ≥ 1.85.0
- Check `activationEvents` in package.json
- Reload VSCode window

### Syntax highlighting not working in test
- VSCode might cache old extension
- Uninstall completely and reinstall `.vsix`
- Or run in Extension Development Host (F5)

---

## File Checklist for Publishing

Before publishing, verify all files present:

```bash
cd vscode-dgm

# Check structure
tree -L 2

# Verify required files
ls -la package.json
ls -la language-configuration.json
ls -la extension.js
ls -la README.md
ls -la CHANGELOG.md
ls -la syntaxes/dgm.tmLanguage.json
ls -la snippets/dgm.json

# Validate JSON
jq . package.json > /dev/null && echo "✓ package.json"
jq . language-configuration.json > /dev/null && echo "✓ language-configuration.json"
jq . syntaxes/dgm.tmLanguage.json > /dev/null && echo "✓ dgm.tmLanguage.json"
jq . snippets/dgm.json > /dev/null && echo "✓ snippets/dgm.json"
```

---

## Publishing Checklist

- [ ] All files present and valid JSON
- [ ] Version bumped in `package.json`
- [ ] CHANGELOG.md updated
- [ ] Tested locally (.vsix install works)
- [ ] Syntax highlighting verified
- [ ] Snippets accessible
- [ ] Commands functional
- [ ] README clear and complete
- [ ] No sensitive data in files
- [ ] No large binary files (keep < 2MB)
- [ ] License file included
- [ ] .vscodeignore excludes unnecessary files

---

## Continuous Integration (Optional)

Create `.github/workflows/publish.yml`:

```yaml
name: Publish Extension

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions/setup-node@v2
        with:
          node-version: '18'
      - run: npm install -g @vscode/vsce
      - run: cd vscode-dgm && vsce package
      - uses: actions/upload-artifact@v2
        with:
          name: vsix
          path: vscode-dgm/*.vsix
      - run: cd vscode-dgm && vsce publish -p ${{ secrets.VSCE_TOKEN }}
```

Then set `VSCE_TOKEN` secret in GitHub → Settings → Secrets.

---

## Support

- **VSCode Docs**: https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- **VSCE GitHub**: https://github.com/microsoft/vscode-vsce
- **Marketplace**: https://marketplace.visualstudio.com
- **Issues**: Report bugs on DGM repository

---

**Version**: 0.2.0  
**Last Updated**: 2026-04-05
