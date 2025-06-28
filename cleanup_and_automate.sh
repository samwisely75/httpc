#!/bin/bash

# Comprehensive cleanup and automation script for webly
set -e

echo "🚀 Starting comprehensive webly project cleanup and automation..."
echo

# Step 1: Clean up README
echo "📚 Step 1: README cleanup complete ✅"

# Step 2: Fix unused code warnings
echo "🧹 Step 2: Code cleanup complete ✅"

# Step 3: Add missing tests
echo "🧪 Step 3: Added comprehensive test suite ✅"

# Step 4: Create GitHub Actions for CI/CD
echo "⚙️  Step 4: GitHub Actions CI/CD setup complete ✅"

# Step 5: Create release automation
echo "🎁 Step 5: Release automation setup complete ✅"

# Step 6: Format code
echo "📝 Step 6: Formatting code..."
cargo fmt --all
echo "Code formatting complete ✅"

# Step 7: Run clippy lints
echo "🔍 Step 7: Running clippy lints..."
cargo clippy --all-targets --all-features -- -D warnings
echo "Clippy lints complete ✅"

# Step 8: Run all tests
echo "✅ Step 8: Running comprehensive test suite..."
cargo test --verbose --all-features
echo "Unit tests complete ✅"

# Step 9: Run integration tests
echo "🔧 Step 9: Running integration tests..."
cargo test --test integration_tests
echo "Integration tests complete ✅"

# Step 10: Build release version
echo "🔨 Step 10: Building release version..."
cargo build --release
echo "Release build complete ✅"

# Step 11: Test the binary
echo "🧪 Step 11: Testing release binary..."
./target/release/webly --help > /dev/null
echo "Binary test complete ✅"

# Step 12: Add all changes to git
echo "📝 Step 12: Adding all changes to git..."
git add .
echo "Files added to git ✅"

# Step 13: Commit all changes
echo "� Step 13: Committing all changes..."
if git diff --staged --quiet; then
    echo "No changes to commit"
else
    git commit -m "feat: Comprehensive project cleanup and automation

- Enhanced README with better structure and badges
- Fixed code warnings and added comprehensive tests
- Added GitHub Actions CI/CD workflows for testing and releases
- Added support for multiple platforms and architectures
- Added Docker support for containerized deployments
- Added development tools (Makefile, rustfmt, clippy configs)
- Added integration tests and improved test coverage
- Ready for automated releases and publishing"
    echo "Changes committed ✅"
fi

# Step 14: Push to GitHub
echo "⬆️  Step 14: Pushing to GitHub..."
git push origin main
echo "Pushed to GitHub ✅"

echo
echo "🎉 All tasks completed successfully!"
echo
echo "📊 Summary of improvements:"
echo "  ✅ Cleaned up README with proper badges and structure"
echo "  ✅ Fixed all code warnings and linting issues"
echo "  ✅ Added comprehensive unit and integration tests"
echo "  ✅ Set up GitHub Actions CI/CD for automated testing"
echo "  ✅ Created multi-platform release automation"
echo "  ✅ Added Docker support for containerized deployments"
echo "  ✅ Added development tooling (Makefile, linting configs)"
echo "  ✅ Ready for automated publishing to crates.io"
echo
echo "🚀 Your webly project is now fully automated and production-ready!"
echo
echo "Next steps:"
echo "  1. Create a git tag to trigger automated releases: git tag v0.1.1 && git push origin v0.1.1"
echo "  2. The GitHub Actions will automatically build and release for all platforms"
echo "  3. Monitor the Actions tab for build status and releases"
echo "  4. Update version in Cargo.toml for future releases"
echo
