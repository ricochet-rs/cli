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

# Build statically linked binaries for Linux, macOS, and Windows
# Usage: just build-static [target]
# Builds fully static Linux binaries that work on any Linux system, macOS binaries, and Windows executables
# Examples:
#   just build-static                    # builds all targets
#   just build-static x86_64
#   just build-static aarch64
#   just build-static riscv64
#   just build-static windows
#   just build-static macos-arm64
#   just build-static macos-x86
#
# Prerequisites on Debian/Ubuntu:
#   apt install gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu gcc-mingw-w64 pkg-config libssl-dev build-essential
#   rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu riscv64gc-unknown-linux-gnu x86_64-pc-windows-gnu
# Prerequisites on macOS:
#   rustup target add x86_64-apple-darwin aarch64-apple-darwin
build-static target="all":
    #!/usr/bin/env bash
    set -euo pipefail

    export RUSTC_WRAPPER=sccache

    rustup default stable

    # Use SQLx offline mode to avoid needing database during build
    export SQLX_OFFLINE=true

    # Determine which profile to use
    # Use "release" if CI_COMMIT_TAG is set (tagged release), otherwise use "testing"
    if [ -n "${CI_COMMIT_TAG:-}" ]; then
        PROFILE="release"
        echo "Building with 'release' profile (CI_COMMIT_TAG is set)"
    else
        PROFILE="testing"
        echo "Building with 'testing' profile (CI_COMMIT_TAG is not set)"
    fi

    # Set flags for static linking with glibc
    # Note: We avoid PIE (Position Independent Executable) for static builds
    export RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static -C link-arg=-no-pie"

    # Setup cross-compilation for glibc targets
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
    export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar

    export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
    export CC_riscv64gc_unknown_linux_gnu=riscv64-linux-gnu-gcc
    export AR_riscv64gc_unknown_linux_gnu=riscv64-linux-gnu-ar

    # Create output directory
    mkdir -p target/binaries

    # Function to clean up assets after build
    cleanup_assets() {
        if [ -n "${CI:-}" ]; then
            echo "Cleaning up UI assets and cache (CI environment detected)..."
            rm -rf ./target/site
            # Clean incremental compilation cache
            rm -rf ./target/release/incremental
            rm -rf ./target/testing/incremental
            # Clean cargo cache for the profile that was used
            if [ "$PROFILE" = "release" ]; then
                cargo clean --release 2>/dev/null || true
            else
                cargo clean --profile testing 2>/dev/null || true
            fi
            echo "✓ Cleanup complete"
        fi
    }

    case "{{target}}" in
        "x86_64")
            echo "Building static binary for x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-x86_64
            cleanup_assets
            echo "✓ Built static x86_64-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-x86_64"
            ;;
        "aarch64")
            echo "Building static binary for aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-aarch64
            cleanup_assets
            echo "✓ Built static aarch64-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-aarch64"
            ;;
        "riscv64"|"riscv")
            echo "Building static binary for riscv64gc-unknown-linux-gnu..."
            rustup target add riscv64gc-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target riscv64gc-unknown-linux-gnu
            cp target/riscv64gc-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-riscv64
            cleanup_assets
            echo "✓ Built static riscv64gc-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-riscv64"
            ;;
        "windows"|"win")
            echo "Building static binary for x86_64-pc-windows-gnu..."
            rustup target add x86_64-pc-windows-gnu || true
            # Windows builds use different RUSTFLAGS
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-pc-windows-gnu
            cp target/x86_64-pc-windows-gnu/$PROFILE/ricochet.exe target/binaries/ricochet-windows.exe
            cleanup_assets
            echo "✓ Built static x86_64-pc-windows-gnu successfully"
            echo "Binary location: target/binaries/ricochet-windows.exe"
            ;;
        "macos-arm64"|"macos-arm"|"darwin-arm64")
            echo "Building binary for aarch64-apple-darwin..."
            rustup target add aarch64-apple-darwin || true
            # macOS uses different RUSTFLAGS (no static linking flags)
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-apple-darwin
            cp target/aarch64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-arm64
            cleanup_assets
            echo "✓ Built aarch64-apple-darwin successfully"
            echo "Binary location: target/binaries/ricochet-macos-arm64"
            ;;
        "macos-x86"|"macos-x64"|"darwin-x86")
            echo "Building binary for x86_64-apple-darwin..."
            rustup target add x86_64-apple-darwin || true
            # macOS uses different RUSTFLAGS (no static linking flags)
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-apple-darwin
            cp target/x86_64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-x86
            cleanup_assets
            echo "✓ Built x86_64-apple-darwin successfully"
            echo "Binary location: target/binaries/ricochet-macos-x86"
            ;;
        "all")
            echo "Building binaries for all targets..."

            echo "Building static x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-x86_64

            echo "Building static aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-aarch64

            echo "Building static riscv64gc-unknown-linux-gnu..."
            rustup target add riscv64gc-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target riscv64gc-unknown-linux-gnu
            cp target/riscv64gc-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-riscv64

            echo "Building static x86_64-pc-windows-gnu..."
            rustup target add x86_64-pc-windows-gnu || true
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-pc-windows-gnu
            cp target/x86_64-pc-windows-gnu/$PROFILE/ricochet.exe target/binaries/ricochet-windows.exe

            echo "Building aarch64-apple-darwin..."
            rustup target add aarch64-apple-darwin || true
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-apple-darwin
            cp target/aarch64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-arm64

            echo "Building x86_64-apple-darwin..."
            rustup target add x86_64-apple-darwin || true
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-apple-darwin
            cp target/x86_64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-x86

            cleanup_assets

            echo "All binaries built successfully!"
            echo "Binaries location:"
            echo "  - Linux x86_64:    target/binaries/ricochet-x86_64"
            echo "  - Linux aarch64:   target/binaries/ricochet-aarch64"
            echo "  - Linux riscv64:   target/binaries/ricochet-riscv64"
            echo "  - Windows x86_64:  target/binaries/ricochet-windows.exe"
            echo "  - macOS ARM64:     target/binaries/ricochet-macos-arm64"
            echo "  - macOS x86_64:    target/binaries/ricochet-macos-x86"
            ;;
        *)
            echo "Unknown target: {{target}}"
            echo "Available targets:"
            echo "  - x86_64             Linux x86_64"
            echo "  - aarch64            Linux ARM64"
            echo "  - riscv64 (riscv)    Linux RISC-V 64"
            echo "  - windows (win)      Windows x86_64"
            echo "  - macos-arm64        macOS ARM64"
            echo "  - macos-x86          macOS x86_64"
            echo "  - all                All targets"
            exit 1
            ;;
    esac


build-local PROFILE="release":
    cargo build --profile {{PROFILE}} --bin ricochet

# Move built binary to local bin
move-cli-local PROFILE="release": (build-local PROFILE)
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
