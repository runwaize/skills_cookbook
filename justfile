# Skill Cookbook Relay - Justfile
# Run commands with: just <command>

# Default recipe (show all available commands)
default:
    @just --list

# Development commands
# ====================

# Run development server (uses live skills.runwaize.com)
dev:
    cargo tauri dev

# Run development server against local web app (localhost:5175)
dev-local:
    TAURI_DEV_WEB=http://localhost:5175 cargo tauri dev

# Check code for errors (fast)
check:
    cargo check --all-targets

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run full test cycle (format, lint, test) for skills delivery pipeline
test-delivery:
    just fmt-check
    just lint
    just test
    @echo "✅ Skills Cookbook relay tests passed!"

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Run tests in watch mode (requires cargo-watch)
test-watch:
    cargo watch -x test

# Build commands
# ==============

# Build debug version
build-debug:
    cargo build

# Build release version (optimized)
build-release:
    cargo build --release

# Build Tauri app bundle for current platform
build-app:
    cargo tauri build

# Build Tauri app in debug mode
build-app-debug:
    cargo tauri build --debug

# Code quality
# ============

# Format code
fmt:
    cargo fmt --all

# Check formatting without making changes
fmt-check:
    cargo fmt --all -- --check

# Run clippy linter
lint:
    cargo clippy --all-targets -- -D warnings

# Fix auto-fixable lint issues
fix:
    cargo fix --allow-dirty --allow-staged
    cargo clippy --fix --allow-dirty --allow-staged

# Run all quality checks (format + lint + test)
quality: fmt-check lint test

# Clean commands
# ==============

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Clean and rebuild
rebuild: clean build-release

# Documentation
# =============

# Generate and open documentation
docs:
    cargo doc --open --no-deps

# Generate documentation for all dependencies
docs-all:
    cargo doc --open

# Install commands
# ================

# Install development dependencies
install-dev:
    cargo install cargo-watch
    cargo install cargo-edit
    cargo install cargo-outdated

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Run commands
# ============

# Run the binary directly (without Tauri)
run:
    cargo run

# Run with specific log level
run-debug:
    RUST_LOG=debug cargo run

# Run with trace logging
run-trace:
    RUST_LOG=trace cargo run

# Utility commands
# ================

# Show dependency tree
deps:
    cargo tree

# Check binary size
size:
    @echo "Debug binary size:"
    @ls -lh target/debug/skill-cookbook-relay 2>/dev/null || echo "Not built yet"
    @echo "\nRelease binary size:"
    @ls -lh target/release/skill-cookbook-relay 2>/dev/null || echo "Not built yet"

# Benchmark build times
bench-build:
    cargo clean
    time cargo build --release

# CI/CD commands
# ==============

# Run full CI pipeline locally
ci: fmt-check lint test build-release
    @echo "✅ All CI checks passed!"

# Quick validation (fast checks before commit)
pre-commit: fmt check test
    @echo "✅ Ready to commit!"

# Release preparation
# ====================

# Prepare for release (bump version, build, test)
prepare-release VERSION:
    @echo "Preparing release {{VERSION}}..."
    cargo set-version {{VERSION}}
    just ci
    just build-app
    @echo "✅ Release {{VERSION}} prepared!"

# Tag release
tag-release VERSION MESSAGE:
    git tag -a v{{VERSION}} -m "{{MESSAGE}}"
    git push origin v{{VERSION}}

# Platform-specific
# =================

# macOS: Create DMG installer
[macos]
dmg: build-app
    @echo "DMG created in target/release/bundle/dmg/"

# macOS: Notarize app
[macos]
notarize:
    @echo "TODO: Add notarization script"

# Linux: Create AppImage
[linux]
appimage: build-app
    @echo "AppImage created in target/release/bundle/appimage/"

# Windows: Create MSI installer
[windows]
msi: build-app
    @echo "MSI created in target/release/bundle/msi/"

# Debugging
# =========

# Show Cargo configuration
show-config:
    cargo config get

# Show build target info
show-target:
    rustc --version --verbose

# Check for common issues
doctor:
    @echo "Checking Rust installation..."
    rustc --version
    @echo "\nChecking Cargo..."
    cargo --version
    @echo "\nChecking Tauri CLI..."
    cargo tauri --version 2>/dev/null || echo "⚠️  Tauri CLI not installed"
    @echo "\nChecking dependencies..."
    cargo tree --depth 1
    @echo "\n✅ Doctor check complete!"

# Maintenance
# ===========

# Audit dependencies for security vulnerabilities
audit:
    cargo audit

# Install cargo-audit if not present
install-audit:
    cargo install cargo-audit

# Generate changelog
changelog:
    git log --oneline --decorate --graph

# Full setup for new developers
setup: install-dev
    @echo "Setting up development environment..."
    rustup component add clippy rustfmt
    @echo "✅ Setup complete! Run 'just dev' to start developing."


