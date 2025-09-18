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
#   just build                           # builds for current platform
#   just build x86_64-unknown-linux-gnu
#   just build x86_64-apple-darwin
#   just build aarch64-apple-darwin
#   just build x86_64-pc-windows-gnu
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
        cargo build --release --bin ricochet --target {{target}}
        echo "✓ Build complete: target/{{target}}/release/ricochet"
    fi

# Build statically linked Linux binaries
# Usage: just build-static [target]
# Builds fully static Linux binaries that work on any Linux system
# Examples:
#   just build-static                    # builds all targets
#   just build-static x86_64
#   just build-static aarch64
# 
# Prerequisites on Debian/Ubuntu:
#   apt install gcc-aarch64-linux-gnu pkg-config libssl-dev build-essential
#   rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
build-static target="all":
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Set flags for static linking
    export RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static"
    
    # Setup cross-compilation for glibc targets
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
    export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar

    # Create output directory
    mkdir -p target/releases
    
    case "{{target}}" in
        "x86_64")
            echo "Building static binary for x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
            cargo build --release --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64-static
            echo "✓ Built static x86_64-unknown-linux-gnu successfully"
            echo "Binary location: target/releases/ricochet-linux-x64-static"
            ;;
        "aarch64")
            echo "Building static binary for aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
            cargo build --release --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-aarch64-static
            echo "✓ Built static aarch64-unknown-linux-gnu successfully"
            echo "Binary location: target/releases/ricochet-linux-aarch64-static"
            ;;
        "all")
            echo "Building static binaries for all Linux targets..."
            
            echo "Building static x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
            cargo build --release --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64-static
            
            echo "Building static aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
            cargo build --release --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-aarch64-static
            
            echo "All static binaries built successfully!"
            echo "Binaries location:"
            echo "  - x86_64:  target/releases/ricochet-linux-x64-static"
            echo "  - aarch64: target/releases/ricochet-linux-aarch64-static"
            ;;
        *)
            echo "Unknown target: {{target}}"
            echo "Available targets: x86_64, aarch64, all"
            exit 1
            ;;
    esac

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