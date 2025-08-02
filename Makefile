# ControlRake Development Makefile 🦀

# Default Rust environment
CARGO := cargo
RUST_VERSION := 1.87

# Default targets
.PHONY: help build test clean run check fmt clippy doc

# Default target
help: ## Show this help message
	@echo "ControlRake 🚀 - Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# Build commands
build: ## Build the project
	$(CARGO) build

build-release: ## Build optimized release version
	$(CARGO) build --release

# Testing
test: ## Run all tests
	$(CARGO) test

test-verbose: ## Run tests with verbose output  
	$(CARGO) test -- --nocapture

# Quick test commands
test-unit: ## Run unit tests only
	$(CARGO) test --lib

test-integration: ## Run integration tests only  
	$(CARGO) test --test '*'

# Development workflow
check-todos: ## 🚨 Enforce TODO rule - only warn!() allowed
	@echo "🔍 Checking for comment-style TODOs..."
	@if grep -r "// TODO\|# TODO\|/\* TODO" src/ --include="*.rs" 2>/dev/null; then \
		echo "❌ RULE VIOLATION: Found comment-style TODOs. Use warn!(\"TODO: ...\") instead!"; \
		exit 1; \
	fi
	@if grep -r "TODO" src/ --include="*.rs" 2>/dev/null | grep -v "warn.*TODO"; then \
		echo "❌ RULE VIOLATION: Found non-warn TODOs. Use warn!(\"TODO: ...\") instead!"; \
		exit 1; \
	fi
	@echo "✅ All TODOs follow the warn!() pattern"

dev: check-todos ## Run checks and tests with TODO enforcement
	$(CARGO) fmt --check
	$(CARGO) clippy -- -D warnings
	$(CARGO) test

check: check-todos ## Run cargo check with TODO enforcement
	$(CARGO) check

# Watch for changes  
watch: ## Watch and run tests on change
	cargo watch -x test

# Development tools

fmt: ## Format code with rustfmt
	$(CARGO) fmt

clippy: ## Run clippy linter
	$(CARGO) clippy -- -D warnings

# Documentation
doc: ## Generate and open documentation
	$(CARGO) doc --open

# Cleaning
clean: ## Clean build artifacts
	$(CARGO) clean

# Running (Note: User should run manually as requested)
run-dev: ## Show how to run in development (don't actually run)
	@echo "To run the server manually:"
	@echo "  cargo run"
	@echo ""
	@echo "Make sure you have:"
	@echo "  - config.toml in project root"
	@echo "  - static/ folder with index.html"

# Development setup
setup: ## Setup development environment
	@echo "Setting up ControlRake development environment..."
	@mkdir -p static
	@echo "<!DOCTYPE html><html><head><title>ControlRake</title></head><body><h1>🚀 ControlRake</h1><p>System running!</p></body></html>" > static/index.html
	@echo "Creating default config.toml..."
	@echo "[system]" > config.toml
	@echo "shutdown_timeout = 30" >> config.toml
	@echo "log_level = \"info\"" >> config.toml
	@echo "" >> config.toml
	@echo "[http]" >> config.toml
	@echo "port = 8888" >> config.toml
	@echo "auth_key = \"dev-secret-key\"" >> config.toml
	@echo "static_folder = \"./static\"" >> config.toml
	@echo ""
	@echo "✅ Setup complete!"
	@echo "  - Created static/index.html"
	@echo "  - Created config.toml"
	@echo ""
	@echo "Next steps:"
	@echo "  make build    # Build the project"
	@echo "  make test     # Run tests"
	@echo "  cargo run     # Run manually when ready"

# Git helpers
commit: ## Add all and commit with message (usage: make commit MSG="your message")
ifndef MSG
	@echo "Usage: make commit MSG=\"your commit message\""
	@exit 1
endif
	git add .
	git commit -m "$(MSG)"

init-git: ## Initialize git repository
	git init
	@echo "# ControlRake 🚀" > README.md
	@echo "" >> README.md
	@echo "Component messaging system with HTTP/WebSocket gateway." >> README.md
	@echo "" >> README.md
	@echo "## Quick Start" >> README.md
	@echo "" >> README.md
	@echo "1. \`make setup\` - Create config and static files" >> README.md
	@echo "2. \`make build\` - Build the project" >> README.md  
	@echo "3. \`cargo run\` - Start the server" >> README.md
	@echo "4. Open http://localhost:8888" >> README.md
	@echo "" >> README.md
	@echo "## Development" >> README.md
	@echo "" >> README.md
	@echo "- \`make test\` - Run tests" >> README.md
	@echo "- \`make clippy\` - Run linter" >> README.md
	@echo "- \`make fmt\` - Format code" >> README.md
	git add .
	git commit -m "🚀 Initial ControlRake project setup"

# Environment info
env: ## Show development environment info
	@echo "🦀 Rust Environment:"
	@rustc --version
	@cargo --version
	@echo ""
	@echo "📦 Project Info:"
	@echo "  Name: controlrake"
	@echo "  Location: $(PWD)"
	@echo ""
	@echo "⚙️ Configuration:"
	@if [ -f config.toml ]; then echo "  ✅ config.toml exists"; else echo "  ❌ config.toml missing (run 'make setup')"; fi
	@if [ -d static ]; then echo "  ✅ static/ folder exists"; else echo "  ❌ static/ folder missing (run 'make setup')"; fi
