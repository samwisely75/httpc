# WebLy Release Process

This document describes the release process for WebLy, based on the sophisticated release pipeline from the quot project.

## Release Pipeline Overview

The WebLy release pipeline uses **release branches** instead of tags to trigger releases. This approach provides better control and allows for last-minute fixes before release.

## Release Workflow

### 1. Create Release Branch

```bash
# Create a release branch from develop
git checkout develop
git pull origin develop
git checkout -b release/1.2.3
git push origin release/1.2.3
```

### 2. Automatic Release Process

When you push a `release/*` branch, the pipeline automatically:

1. **Builds cross-platform binaries** for:
   - Windows (x64)
   - macOS (Intel and Apple Silicon)
   - Linux (x64 and ARM64)

2. **Creates distribution packages**:
   - `.deb` packages for Debian/Ubuntu
   - Man pages and documentation
   - Configuration templates

3. **Tests all binaries** across platforms

4. **Creates GitHub release**:
   - Extracts version from branch name
   - Creates and pushes git tag
   - Uploads all artifacts
   - Generates release notes

5. **Merges to main and develop**:
   - Merges release branch to main
   - Syncs changes back to develop
   - Cleans up release branch

## Key Features

### Version Extraction
- Version is extracted from branch name: `release/1.2.3` → `v1.2.3`
- No need to manually create tags

### Cross-Platform Support
- Full matrix builds for all supported platforms
- ARM64 support for both Linux and macOS
- Proper cross-compilation setup

### Package Management
- Professional `.deb` packages with proper metadata
- Postinstall scripts for configuration setup
- Man pages and documentation included

### Configuration Templates
- Includes `profile.example` with common configurations
- Automatically creates `~/.httpc/profile` on first install
- Examples for various API types (REST, GraphQL, XML)

### Testing
- Binaries tested on each platform
- Basic functionality verification
- Configuration file testing

## Usage Examples

### Basic Release
```bash
# Update version in Cargo.toml first
git checkout develop
git add Cargo.toml
git commit -m "Bump version to 1.2.3"
git push origin develop

# Create release branch
git checkout -b release/1.2.3
git push origin release/1.2.3

# Pipeline runs automatically
```

### Emergency Hotfix
```bash
# Create hotfix from main
git checkout main
git pull origin main
git checkout -b release/1.2.4
# Make fixes
git commit -m "Fix critical bug"
git push origin release/1.2.4
```

## Release Assets

Each release includes:
- **Cross-platform binaries**: Windows, macOS (Intel/ARM), Linux (x64/ARM64)
- **Linux packages**: `.deb` packages with proper dependencies
- **Documentation**: Man pages, configuration examples
- **Configuration templates**: Ready-to-use profile examples

## Installation Methods

### Package Managers (Linux)
```bash
# Debian/Ubuntu
sudo dpkg -i httpc_*_amd64.deb

# Creates /etc/httpc/profile.example
# Sets up ~/.httpc/profile if not exists
```

### Direct Binary
```bash
# Download binary from GitHub releases
chmod +x httpc-linux-x64
sudo mv httpc-linux-x64 /usr/local/bin/httpc
```

## Configuration

### Profile Setup
The release pipeline creates comprehensive profile examples:
- Default API configurations
- Local development setups
- Staging and production environments
- Various content types (JSON, XML, GraphQL)
- Authentication examples

### Post-Installation
```bash
# Check installation
httpc --version

# Copy example profile
cp /etc/httpc/profile.example ~/.httpc/profile
# Edit as needed
```

## Pipeline Comparison with quot

### Similarities
- Release branch triggered
- Cross-platform builds
- Comprehensive caching
- Automated tag creation
- Merge to main/develop

### WebLy Enhancements
- **Configuration focus**: Includes profile templates and setup
- **Simplified packaging**: Focused on essential packages
- **API-centric**: Examples tailored for HTTP clients
- **Profile management**: Automatic profile setup

### Missing Features (vs quot)
- **Homebrew publishing**: Not implemented (can be added later)
- **RPM packages**: Removed to focus on DEB (can be re-added)
- **Crates.io publishing**: Disabled by default (ready when needed)

## Future Enhancements

### Planned Additions
1. **Homebrew formula**: Auto-publish to tap
2. **RPM packages**: Add back RPM support
3. **Crates.io publishing**: Enable when ready
4. **Windows installer**: MSI package creation
5. **Docker images**: Multi-arch container images

### Configuration Enhancements
1. **Profile validation**: Verify profile syntax
2. **Profile migration**: Handle config updates
3. **Profile templates**: More specialized examples
4. **Environment detection**: Auto-configure common environments

## Security Considerations

- All package scripts are reviewed
- Postinstall scripts are minimal and safe
- Configuration files use secure defaults
- No sensitive data in examples

## Troubleshooting

### Common Issues
1. **Version extraction fails**: Check branch name format `release/X.Y.Z`
2. **Build fails**: Check target platform compatibility
3. **Package install fails**: Verify dependencies

### Debug Steps
```bash
# Check release branch
git branch -a | grep release

# Verify version format
echo "release/1.2.3" | grep -E "^release/[0-9]+\.[0-9]+\.[0-9]+"

# Test binary
./httpc --version
./httpc --help
```

This release pipeline provides a robust, automated solution for WebLy releases with comprehensive configuration management and cross-platform support.
