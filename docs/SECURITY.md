# Security and Vulnerability Scanning

This document describes the security and vulnerability scanning setup for the httpc project.

## Current Security Tools

### 1. cargo-audit

- **Purpose**: Scans for known security vulnerabilities in Rust dependencies
- **Database**: RustSec Advisory Database
- **Coverage**: Excellent for Rust/Cargo projects
- **Status**: ✅ Active in CI pipeline

### 2. cargo-deny

- **Purpose**: Supply chain security, license compliance, and dependency management
- **Features**:
  - Security advisory checks
  - License validation
  - Duplicate dependency detection
  - Source validation
- **Status**: ✅ Active in CI pipeline
- **Configuration**: `deny.toml`

## Future Security Integrations

### 3. Snyk (Planned)

- **Purpose**: Additional vulnerability scanning and SaaS integration
- **Status**: 🔄 Prepared but not active (limited Rust support)
- **Setup**: Ready for activation when Rust support improves

## Snyk API Token Setup

To integrate Snyk when their Rust support improves:

### Step 1: Get Snyk API Token

1. Log in to [Snyk.io](https://snyk.io/)
2. Go to **Account Settings** → **API Token**
3. Copy your API token

### Step 2: Add to GitHub Secrets

1. Go to GitHub repository: `https://github.com/samwisely75/httpc`
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **"New repository secret"**
4. Add:
   - **Name**: `SNYK_TOKEN`
   - **Value**: `your-snyk-api-token`

### Step 3: Enable in CI Pipeline

Uncomment the Snyk steps in `.github/workflows/ci.yml`:

```yaml
- name: Run Snyk to check for vulnerabilities
  uses: snyk/actions/node@master
  env:
    SNYK_TOKEN: ${{ secrets.SNYK_TOKEN }}
  with:
    args: --severity-threshold=high --file=Cargo.toml
```

## Security Scanning Results

All current scans pass with no security vulnerabilities:

- ✅ **cargo-audit**: No known vulnerabilities
- ✅ **cargo-deny**: No license or security issues
- ✅ **All dependencies**: From trusted sources

## Maintenance

Security tools are automatically updated and run on every:

- Push to `develop` branch
- Pull request to `develop` branch

Manual checks can be run locally:

```bash
# Run all security checks
make security-check

# Individual tools
cargo audit
cargo deny check
```

## Notes

- The `atty` crate security issues were resolved by migrating to `std::io::IsTerminal`
- All licenses are properly validated and approved
- Duplicate dependencies are monitored but don't fail CI (normal for cross-platform Rust)
