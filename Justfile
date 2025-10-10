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
    just docs-check

# run cargo clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

lint-fix:
    cargo clippy --fix --all-targets --all-features -- -D warnings

# Test commands
test:
    cargo test --all-features --workspace

# Build the CLI for current platform
cli-build PROFILE="release":
    cargo build --profile {{PROFILE}} --bin ricochet

# Build CLI for Linux x86_64
cli-build-linux-x64 PROFILE="release":
    @echo "Building CLI for Linux x86_64 with profile: {{PROFILE}}"
    rustup target add x86_64-unknown-linux-gnu
    @mkdir -p target/binaries
    cargo build --profile {{PROFILE}} --bin ricochet --target x86_64-unknown-linux-gnu
    @cp target/x86_64-unknown-linux-gnu/{{PROFILE}}/ricochet target/binaries/ricochet-linux-x64

# Build CLI for Linux ARM64
cli-build-linux-aarch64 PROFILE="release":
    @echo "Building CLI for Linux aarch64 with profile: {{PROFILE}}"
    rustup target add aarch64-unknown-linux-gnu
    @mkdir -p target/binaries
    cargo build --profile {{PROFILE}} --bin ricochet --target aarch64-unknown-linux-gnu
    @cp target/aarch64-unknown-linux-gnu/{{PROFILE}}/ricochet target/binaries/ricochet-linux-aarch64

# Build CLI for macOS x86_64
cli-build-macos-x64 PROFILE="release":
    @echo "Building CLI for macOS x86_64 with profile: {{PROFILE}}"
    rustup target add x86_64-apple-darwin
    @mkdir -p target/binaries
    cargo build --profile {{PROFILE}} --bin ricochet --target x86_64-apple-darwin
    @cp target/x86_64-apple-darwin/{{PROFILE}}/ricochet target/binaries/ricochet-macos-x64

# Build CLI for macOS ARM64 (Apple Silicon)
cli-build-macos-arm64 PROFILE="release":
    @echo "Building CLI for macOS ARM64 with profile: {{PROFILE}}"
    rustup target add aarch64-apple-darwin
    @mkdir -p target/binaries
    cargo build --profile {{PROFILE}} --bin ricochet --target aarch64-apple-darwin
    @cp target/aarch64-apple-darwin/{{PROFILE}}/ricochet target/binaries/ricochet-macos-arm64

# Build CLI for Windows
cli-build-windows-x64 PROFILE="release":
    @echo "Building CLI for Windows with profile: {{PROFILE}}"
    rustup target add x86_64-pc-windows-gnu
    @mkdir -p target/binaries
    cargo build --profile {{PROFILE}} --bin ricochet --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/{{PROFILE}}/ricochet.exe target/binaries/ricochet-windows.exe

# Move built binary to local bin
move-cli-local PROFILE="release": (cli-build PROFILE)
    sudo cp target/{{PROFILE}}/ricochet ~/.local/bin/ricochet-dev

# generate CLI documentation
docs:
    @echo "Generating CLI documentation..."
    @mkdir -p docs
    @cargo run --quiet --manifest-path docs-generator/Cargo.toml > docs/cli-commands.md 2>/dev/null
    @echo "✓ Documentation generated: docs/cli-commands.md"

# check if CLI documentation is up-to-date
docs-check:
    @echo "Checking if CLI documentation is up-to-date..."
    @mkdir -p docs
    @cargo run --quiet --manifest-path docs-generator/Cargo.toml > /tmp/cli-commands-generated.md 2>/dev/null
    @if ! diff -q docs/cli-commands.md /tmp/cli-commands-generated.md > /dev/null 2>&1; then \
        echo "❌ ERROR: CLI documentation is out of date!"; \
        echo ""; \
        echo "Run 'just docs' to update the documentation."; \
        echo ""; \
        echo "Differences:"; \
        diff -u docs/cli-commands.md /tmp/cli-commands-generated.md || true; \
        rm -f /tmp/cli-commands-generated.md; \
        exit 1; \
    else \
        echo "✓ CLI documentation is up-to-date"; \
        rm -f /tmp/cli-commands-generated.md; \
    fi
