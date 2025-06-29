#!/bin/bash
# Development script to automatically fix common linting issues

set -e

echo "🔧 Running automatic fixes..."

echo "📝 Formatting code..."
cargo fmt

echo "🔍 Running clippy fixes..."
cargo clippy --fix --allow-dirty --all-targets --all-features

echo "✨ Verifying fixes..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🧪 Running tests to ensure nothing broke..."
cargo test

echo "✅ All fixes applied successfully!"
echo ""
echo "💡 You can now commit your changes:"
echo "   git add -A"
echo "   git commit -m 'Apply automatic linting fixes'"
