.PHONY: build test install clean uninstall run-signal-cli help

# Default target
all: build

# Build the project
build:
	@echo "Building claudifier..."
	cargo build --release

# Run tests
test:
	@echo "Running tests..."
	cargo test

# Install to ~/.cargo/bin
install: build
	@echo "Installing claudifier to ~/.cargo/bin..."
	cargo install --path .
	@echo "Installation complete! Run 'claudifier --help' to get started."

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Uninstall from ~/.cargo/bin
uninstall:
	@echo "Uninstalling claudifier..."
	cargo uninstall claudifier
	@echo "Uninstall complete."

# Run signal-cli for testing
run-signal-cli:
	@echo "Running signal-cli daemon mode..."
	@if command -v signal-cli >/dev/null 2>&1; then \
		echo "Starting signal-cli in daemon mode..."; \
		signal-cli daemon; \
	else \
		echo "Error: signal-cli not found in PATH"; \
		echo "Install it from: https://github.com/AsamK/signal-cli"; \
		exit 1; \
	fi

# Display help
help:
	@echo "Claudifier - Universal notification receiver for Claude Code"
	@echo ""
	@echo "Available targets:"
	@echo "  make build           - Build the project in release mode"
	@echo "  make test            - Run all tests"
	@echo "  make install         - Build and install to ~/.cargo/bin"
	@echo "  make clean           - Remove build artifacts"
	@echo "  make uninstall       - Remove from ~/.cargo/bin"
	@echo "  make run-signal-cli  - Run signal-cli in daemon mode"
	@echo "  make help            - Show this help message"
