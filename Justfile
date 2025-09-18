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

# Build the CLI for current platform
cli-build:
    cargo build --release --bin ricochet

# Build CLI for multiple platforms using cross-compilation
cli-build-all: cli-install-targets cli-build-linux-x64 cli-build-linux-aarch64 cli-build-macos-x64 cli-build-macos-arm64 cli-build-windows-x64
    @echo "✓ All CLI builds complete!"
    @ls -lh target/releases/

# Build CLI for Linux x86_64
cli-build-linux-x64:
    @echo "Building CLI for Linux x86_64..."
    rustup target add x86_64-unknown-linux-gnu
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-unknown-linux-gnu
    @cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64

# Build CLI for Linux ARM64
cli-build-linux-aarch64:
    @echo "Building CLI for Linux aarch64..."
    rustup target add aarch64-unknown-linux-gnu
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target aarch64-unknown-linux-gnu
    @cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-aarch64

# Build CLI for macOS x86_64
cli-build-macos-x64:
    @echo "Building CLI for macOS x86_64..."
    rustup target add x86_64-apple-darwin
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-apple-darwin
    @cp target/x86_64-apple-darwin/release/ricochet target/releases/ricochet-macos-x64

# Build CLI for macOS ARM64 (Apple Silicon)
cli-build-macos-arm64:
    @echo "Building CLI for macOS ARM64..."
    rustup target add aarch64-apple-darwin
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target aarch64-apple-darwin
    @cp target/aarch64-apple-darwin/release/ricochet target/releases/ricochet-macos-arm64

# Build CLI for Windows
cli-build-windows-x64:
    @echo "Building CLI for Windows..."
    rustup target add x86_64-pc-windows-gnu
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/ricochet.exe target/releases/ricochet-windows.exe

# Install all required rustup targets for cross-compilation
cli-install-targets:
    @echo "Installing all rustup targets for cross-compilation..."
    rustup target add x86_64-unknown-linux-gnu
    rustup target add aarch64-unknown-linux-gnu
    rustup target add x86_64-apple-darwin
    rustup target add aarch64-apple-darwin
    rustup target add x86_64-pc-windows-gnu
    @echo "✓ All rustup targets installed"

# Install cross-compilation tools
cli-install-cross-tools:
    @echo "Installing cross-compilation tools..."
    cargo install cross
    @echo "✓ Cross tools installed"

# Build CLI using cross for better cross-platform support
cli-cross-build target:
    @echo "Building CLI for {{target}} using cross..."
    @mkdir -p target/releases
    cross build --release --bin ricochet --target {{target}}
    @echo "✓ Build complete for {{target}}"

# Build all CLI targets using cross (more reliable cross-compilation)
cli-cross-build-all:
    @echo "Building CLI for all platforms using cross..."
    @mkdir -p target/releases
    just cli-cross-build x86_64-unknown-linux-gnu
    @cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64
    just cli-cross-build aarch64-unknown-linux-gnu
    @cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-arm64
    just cli-cross-build x86_64-apple-darwin
    @cp target/x86_64-apple-darwin/release/ricochet target/releases/ricochet-macos-x64
    just cli-cross-build aarch64-apple-darwin
    @cp target/aarch64-apple-darwin/release/ricochet target/releases/ricochet-macos-arm64
    just cli-cross-build x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/ricochet.exe target/releases/ricochet-windows.exe
    @echo "✓ All cross-compilation builds complete!"
    @ls -lh target/releases/


move-cli-local:
    sudo cp target/release/ricochet ~/.local/bin/ricochet-dev

# generate CLI documentation
docs:
    @echo "Generating CLI documentation..."
    @mkdir -p docs
    @cargo run --quiet -- generate-docs > docs/cli-commands.md 2>/dev/null
    @echo "✓ Documentation generated: docs/cli-commands.md"
