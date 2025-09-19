set dotenv-load:=true

# default recipe to list all available commands
default:
	@just --list

fmt:
    cargo fmt --all

check:
    cargo check --all-features --workspace
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# run cargo clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

lint-fix:
    cargo clippy --fix --all-targets --all-features -- -D warnings

# Test commands
test:
    cargo test --all-features --workspace

# Build the CLI for current platform or specified target
# Usage: just build [target]
# Examples:
#   just build
#   just build x86_64-unknown-linux-gnu
#   just build aarch64-unknown-linux-gnu
#   just build x86_64-pc-windows-gnu
#   just build x86_64-apple-darwin
#   just build aarch64-apple-darwin
build target="":
    #!/usr/bin/env bash
    mkdir -p target/releases
    if [ -z "{{target}}" ]; then
        echo "Building for current platform..."
        cargo build --release --bin ricochet
        echo "✓ Build complete: target/release/ricochet"
    else
        echo "Building for {{target}}..."
        rustup target add {{target}} 2>/dev/null || true
        echo "CC: $CC"
        echo "CXX: $CXX"
        export CC_x86_64_apple_darwin=o64-clang
        export CXX_x86_64_apple_darwin=o64-clang++
        export AR_x86_64_apple_darwin=o64-ar
        export CC_aarch64_apple_darwin=o64-clang
        export CXX_aarch64_apple_darwin=o64-clang++
        export AR_aarch64_apple_darwin=o64-ar
        cargo build --release --bin ricochet --target {{target}}
        echo "✓ Build complete: target/{{target}}/release/ricochet"
    fi


# Build all release binaries for distribution
# Creates optimized binaries for all major platforms
build-all-release:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Building release binaries for all platforms..."
    mkdir -p target/releases

    # Install all required targets
    echo "Installing compilation targets..."
    rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
    rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
    rustup target add x86_64-apple-darwin 2>/dev/null || true
    rustup target add aarch64-apple-darwin 2>/dev/null || true
    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

    # Build using cross for better compatibility
    if command -v cross &> /dev/null; then
        echo "Using 'cross' for cross-platform compilation..."

        cross build --release --bin ricochet --target x86_64-unknown-linux-gnu
        cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64

        cross build --release --bin ricochet --target aarch64-unknown-linux-gnu
        cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-aarch64

        cross build --release --bin ricochet --target x86_64-apple-darwin
        cp target/x86_64-apple-darwin/release/ricochet target/releases/ricochet-macos-x64

        cross build --release --bin ricochet --target aarch64-apple-darwin
        cp target/aarch64-apple-darwin/release/ricochet target/releases/ricochet-macos-arm64

        cross build --release --bin ricochet --target x86_64-pc-windows-gnu
        cp target/x86_64-pc-windows-gnu/release/ricochet.exe target/releases/ricochet-windows-x64.exe
    else
        echo "Warning: 'cross' not found. Building only for compatible targets..."
        echo "Install 'cross' with: cargo install cross --git https://github.com/cross-rs/cross"

        # Try to build what we can natively
        cargo build --release --bin ricochet

        # Determine current platform and copy appropriate binary
        ARCH=$(uname -m)
        OS=$(uname -s)

        case "$OS" in
            Linux)
                if [ "$ARCH" = "x86_64" ]; then
                    cp target/release/ricochet target/releases/ricochet-linux-x64
                elif [ "$ARCH" = "aarch64" ]; then
                    cp target/release/ricochet target/releases/ricochet-linux-aarch64
                fi
                ;;
            Darwin)
                if [ "$ARCH" = "x86_64" ]; then
                    cp target/release/ricochet target/releases/ricochet-macos-x64
                elif [ "$ARCH" = "arm64" ]; then
                    cp target/release/ricochet target/releases/ricochet-macos-arm64
                fi
                ;;
        esac
    fi

    echo "✓ All release binaries built!"
    echo "Binaries location: target/releases/"
    ls -lh target/releases/

# Install cross-compilation tool for reliable cross-platform builds
install-cross:
    @echo "Installing cross-compilation tool..."
    cargo install cross --git https://github.com/cross-rs/cross
    @echo "✓ Cross installed successfully"

# Build using cross tool (most reliable for cross-platform)
# Usage: just cross-build <target>
# Example: just cross-build x86_64-unknown-linux-gnu
cross-build target:
    @echo "Building for {{target}} using cross..."
    @mkdir -p target/releases
    cross build --release --bin ricochet --target {{target}}
    @echo "✓ Build complete for {{target}}"

# Build all targets using cross (recommended for CI/CD)
cross-build-all:
    @echo "Building all targets using cross..."
    @mkdir -p target/releases

    cross build --release --bin ricochet --target x86_64-unknown-linux-gnu
    @cp target/x86_64-unknown-linux-gnu/release/ricochet target/binaries/ricochet-linux-x64

    cross build --release --bin ricochet --target aarch64-unknown-linux-gnu
    @cp target/aarch64-unknown-linux-gnu/release/ricochet target/binaries/ricochet-linux-aarch64

    cross build --release --bin ricochet --target x86_64-apple-darwin
    @cp target/x86_64-apple-darwin/release/ricochet target/binaries/ricochet-macos-x64

    cross build --release --bin ricochet --target aarch64-apple-darwin
    @cp target/aarch64-apple-darwin/release/ricochet target/binaries/ricochet-macos-arm64

    cross build --release --bin ricochet --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/ricochet.exe target/binaries/ricochet-windows-x64.exe

    @echo "✓ All cross-compilation builds complete!"
    @ls -lh target/binaries/

# Quick build for development (current platform only)
dev:
    cargo build --bin ricochet
    @echo "Development build complete: target/debug/ricochet"

# Install locally for development
install-local: build
    @echo "Installing to ~/.local/bin/ricochet-dev..."
    @mkdir -p ~/.local/bin
    @cp target/release/ricochet ~/.local/bin/ricochet-dev
    @echo "✓ Installed to ~/.local/bin/ricochet-dev"

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/releases

# Generate CLI documentation
docs:
    @echo "Generating CLI documentation..."
    @mkdir -p docs
    @cargo run --quiet -- generate-docs > docs/cli-commands.md 2>/dev/null || echo "Note: generate-docs command may not be available"
    @echo "✓ Documentation generated: docs/cli-commands.md"

# Show binary information
info binary="target/release/ricochet":
    @echo "Binary information for {{binary}}:"
    @file {{binary}} || echo "Binary not found"
    @ldd {{binary}} 2>/dev/null || echo "Not a dynamic executable or ldd not available"
    @du -h {{binary}} 2>/dev/null || echo "Binary not found"

# List all available build targets
list-targets:
    @echo "Available Rust targets:"
    @rustup target list | grep -E "(installed|default)"
    @echo ""
    @echo "Commonly used targets for this project:"
    @echo "  - x86_64-unknown-linux-gnu    (Linux x86_64)"
    @echo "  - aarch64-unknown-linux-gnu   (Linux ARM64)"
    @echo "  - x86_64-apple-darwin         (macOS Intel)"
    @echo "  - aarch64-apple-darwin        (macOS Apple Silicon)"
    @echo "  - x86_64-pc-windows-gnu       (Windows x86_64)"
    @echo ""
    @echo "Install a target with: rustup target add <target-name>"

# Run clippy with auto-fix and format
fix: lint-fix fmt
    @echo "✓ Code fixed and formatted"
