# Release Setup Guide

This document describes the setup required for automated releases.

## Required GitHub Secrets

The release workflow requires the following secrets to be configured in the GitHub repository:

### CARGO_REGISTRY_TOKEN

**Purpose**: Enables automated publishing to crates.io

**Setup**:
1. Visit [crates.io](https://crates.io) and log in with your GitHub account
2. Go to **Account Settings** → **API Tokens**
3. Click **New Token**
4. Give it a descriptive name like "webly-github-actions"
5. Select appropriate scopes (publish permissions)
6. Copy the generated token

**GitHub Configuration**:
1. Go to your repository on GitHub
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `CARGO_REGISTRY_TOKEN`
5. Value: Paste the token from crates.io
6. Click **Add secret**

### GITHUB_TOKEN

**Purpose**: Built-in token for GitHub Actions, automatically available
- Used for creating releases
- Used for uploading assets
- Used for updating Homebrew formula
- No manual setup required

## Release Process

Once secrets are configured, releases are fully automated:

1. **Create a release tag**: `git tag v0.1.7 && git push origin v0.1.7`
2. **Automated pipeline runs**:
   - ✅ CI checks (Linux, macOS, Windows)
   - ✅ Security audit
   - ✅ Integration tests
   - 🔨 Build binaries and packages
   - 🧪 Test package installations
   - 📦 Publish to crates.io
   - 🐳 Push Docker image
   - 🍺 Update Homebrew formula
   - 📋 Create GitHub release

## Verification

After a successful release, verify:

- [ ] New version appears on [crates.io](https://crates.io/crates/webly)
- [ ] GitHub release created with all assets
- [ ] Docker image pushed to registry
- [ ] Homebrew formula updated (if applicable)
- [ ] Installation works: `cargo install webly`

## Troubleshooting

**Crates.io publish fails with "version already exists"**:
- The workflow checks for existing versions and skips if already published
- This is normal behavior to prevent duplicate publishes

**Missing CARGO_REGISTRY_TOKEN**:
- Error: "no token found, please run `cargo login`"
- Solution: Add the secret as described above

**Permissions errors**:
- Ensure the crates.io token has publish permissions
- Verify you're an owner/collaborator on the crate
