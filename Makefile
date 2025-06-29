.PHONY: help build test clean lint fmt check install release

# Default target
help: ## Show this help message
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# Development tasks
build: ## Build the project
	cargo build

test: ## Run all tests
	cargo test --all-features

test-integration: ## Run integration tests
	cargo test --test integration_tests

clean: ## Clean build artifacts
	cargo clean

lint: ## Run clippy lints
	cargo clippy --all-targets --all-features -- -D warnings

fix: ## Automatically fix linting and formatting issues
	@echo "🔧 Running automatic fixes..."
	@cargo fmt
	@cargo clippy --fix --allow-dirty --all-targets --all-features
	@echo "✨ Verifying fixes..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ All fixes applied successfully!"

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

check: fmt-check lint test ## Run all checks (format, lint, test)

# Installation and release
install: ## Install the binary locally
	cargo install --path .

release: ## Build release version
	cargo build --release

# Documentation
docs: ## Generate documentation
	cargo doc --open

docs-build: ## Build documentation without opening
	cargo doc --no-deps

# Benchmarks (if any)
bench: ## Run benchmarks
	cargo bench

# Coverage
coverage: ## Generate test coverage report
	cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html

# Security audit
audit: ## Run security audit
	cargo audit

# Update dependencies
update: ## Update dependencies
	cargo update

# Check minimal versions
minimal-versions: ## Check with minimal dependency versions
	cargo minimal-versions check

# All-in-one quality check
ci: fmt-check lint test ## Run all CI checks locally

# Release preparation
prepare-release: clean check docs-build ## Prepare for release
	@echo "Ready for release!"

# Development setup
setup: ## Set up development environment
	rustup component add rustfmt clippy
	cargo install cargo-tarpaulin cargo-audit cargo-minimal-versions

# Quick development cycle
dev: fmt lint test ## Quick development cycle: format, lint, test
