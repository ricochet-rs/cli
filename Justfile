set dotenv-load:=true

# default recipe to list all available commands
default:
	@just --list

fmt:
    leptosfmt ricochet-ui/src/**/*.rs && leptosfmt ricochet-ui/src/*.rs && cargo fmt --all

check:
    cargo check --all-features --workspace
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings

# run cargo clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

lint-fix:
    cargo clippy --fix --all-targets --all-features -- -D warnings

# Build the CLI for current platform
cli-build:
    cargo build --release --bin ricochet

# Build CLI for multiple platforms using cross-compilation
cli-build-all: cli-build-linux-x64 cli-build-linux-arm64 cli-build-macos-x64 cli-build-macos-arm64 cli-build-windows
    @echo "✓ All CLI builds complete!"
    @ls -lh target/releases/

# Build CLI for Linux x86_64
cli-build-linux-x64:
    @echo "Building CLI for Linux x86_64..."
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-unknown-linux-gnu
    @cp target/x86_64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-x64

# Build CLI for Linux ARM64
cli-build-linux-arm64:
    @echo "Building CLI for Linux ARM64..."
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target aarch64-unknown-linux-gnu
    @cp target/aarch64-unknown-linux-gnu/release/ricochet target/releases/ricochet-linux-arm64

# Build CLI for macOS x86_64
cli-build-macos-x64:
    @echo "Building CLI for macOS x86_64..."
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-apple-darwin
    @cp target/x86_64-apple-darwin/release/ricochet target/releases/ricochet-macos-x64

# Build CLI for macOS ARM64 (Apple Silicon)
cli-build-macos-arm64:
    @echo "Building CLI for macOS ARM64..."
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target aarch64-apple-darwin
    @cp target/aarch64-apple-darwin/release/ricochet target/releases/ricochet-macos-arm64

# Build CLI for Windows
cli-build-windows:
    @echo "Building CLI for Windows..."
    @mkdir -p target/releases
    cargo build --release --bin ricochet --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/ricochet.exe target/releases/ricochet-windows.exe

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

### build container images
# just container alpine alpine reg.devxy.io ricochet ricochet
# just container ubuntu
container variant tag="latest" registry="ghcr.io" org="ricochet-rs" name="ricochet-dev":
    #!/usr/bin/env bash
    IMAGE_NAME="{{registry}}/{{org}}/{{name}}:{{tag}}"
    echo "Building: $IMAGE_NAME"
    case "{{variant}}" in
        "alpine")
            docker buildx build -f container/Containerfile.alpine --push -t "$IMAGE_NAME" .
            ;;
        "ubuntu")
            docker buildx build -f container/Containerfile.ubuntu --push -t "$IMAGE_NAME" .
            ;;
        *)
            echo "Unknown variant: {{variant}}"
            echo "Available variants: alpine, ubuntu"
            exit 1
            ;;
    esac
    echo "Built: $IMAGE_NAME"

# generate CLI documentation
docs:
    @echo "Generating CLI documentation..."
    @mkdir -p docs
    @cargo run --quiet -- generate-docs > docs/cli-commands.md 2>/dev/null
    @echo "✓ Documentation generated: docs/cli-commands.md"
