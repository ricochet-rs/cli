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
#   just build-static linux-x86_64
#   just build-static linux-aarch64
#   just build-static linux-riscv64
#   just build-static windows-x86_64
#   just build-static windows-arm64
#   just build-static macos-arm64
#   just build-static macos-x86_64
#
# Prerequisites on Debian/Ubuntu:
#   apt install gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu gcc-mingw-w64 pkg-config libssl-dev build-essential
#   rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu riscv64gc-unknown-linux-gnu x86_64-pc-windows-gnu aarch64-pc-windows-msvc
# Prerequisites on macOS:
#   rustup target add x86_64-apple-darwin aarch64-apple-darwin
build-static target="all":
    #!/usr/bin/env bash
    set -euo pipefail

    # Set up Rust environment (macOS only)
    # Detect cargo/rustup location - handles both regular users and root
    if [ "$(uname)" = "Darwin" ]; then
      if [ -d "$HOME/.cargo" ]; then
          CARGO_HOME_DIR="$HOME/.cargo"
          RUSTUP_HOME_DIR="$HOME/.rustup"
      elif [ -d "/var/root/.cargo" ]; then
          CARGO_HOME_DIR="/var/root/.cargo"
          RUSTUP_HOME_DIR="/var/root/.rustup"
      elif [ -d "/root/.cargo" ]; then
          CARGO_HOME_DIR="/root/.cargo"
          RUSTUP_HOME_DIR="/root/.rustup"
      else
          echo "Error: Cannot find cargo installation"
          exit 1
      fi

      export PATH="${CARGO_HOME_DIR}/bin:$PATH"
      export CARGO_HOME="${CARGO_HOME:-${CARGO_HOME_DIR}}"
      export RUSTUP_HOME="${RUSTUP_HOME:-${RUSTUP_HOME_DIR}}"
      export RUSTC_WRAPPER=sccache
    fi

    # Set up sccache directory (macOS only)
    if [ "$(uname)" = "Darwin" ]; then
        if [ -z "${SCCACHE_DIR:-}" ]; then
            if [ -d "/var/root" ] && [ "$HOME" = "/var/root" ]; then
                export SCCACHE_DIR="/var/root/Library/Caches/Mozilla.sccache"
            else
                export SCCACHE_DIR="${HOME}/Library/Caches/Mozilla.sccache"
            fi
        fi
    fi

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

    # Setup cross-compilation for Windows ARM64 with clang
    export CC_aarch64_pc_windows_msvc=clang
    export CXX_aarch64_pc_windows_msvc=clang++
    export AR_aarch64_pc_windows_msvc=llvm-ar

    # Determine version string
    if [ -n "${CI_COMMIT_TAG:-}" ]; then
      VERSION="${CI_COMMIT_TAG#v}"
    else
      # Use commit SHA if available, otherwise use date only
      if [ -n "${CI_COMMIT_SHA:-}" ]; then
        COMMIT_SHORT=$(echo "${CI_COMMIT_SHA}" | cut -c1-8)
        VERSION="$(date +%Y%m%d).$COMMIT_SHORT"
      else
        VERSION="$(date +%Y%m%d)"
      fi
    fi

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
        # Linux targets
        "linux-x86_64"|"x86_64")
            echo "Building static binary for x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-x86_64
            cleanup_assets
            echo "✓ Built static x86_64-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-linux-x86_64"
            ;;
        "linux-aarch64"|"aarch64")
            echo "Building static binary for aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-aarch64
            cleanup_assets
            echo "✓ Built static aarch64-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-linux-aarch64"
            ;;
        "linux-riscv64"|"riscv64"|"riscv")
            echo "Building static binary for riscv64gc-unknown-linux-gnu..."
            rustup target add riscv64gc-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target riscv64gc-unknown-linux-gnu
            cp target/riscv64gc-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-riscv64
            cleanup_assets
            echo "✓ Built static riscv64gc-unknown-linux-gnu successfully"
            echo "Binary location: target/binaries/ricochet-linux-riscv64"
            ;;
        # Windows targets
        "windows-x86_64"|"windows"|"win")
            echo "Building static binary for x86_64-pc-windows-gnu..."
            rustup target add x86_64-pc-windows-gnu || true
            # Windows builds use different RUSTFLAGS
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-pc-windows-gnu
            cp target/x86_64-pc-windows-gnu/$PROFILE/ricochet.exe target/binaries/ricochet-windows-x86_64.exe
            cleanup_assets
            echo "✓ Built static x86_64-pc-windows-gnu successfully"
            echo "Binary location: target/binaries/ricochet-windows-x86_64.exe"
            ;;
        "windows-arm64")
            echo "Building static binary for aarch64-pc-windows-msvc..."
            rustup target add aarch64-pc-windows-msvc || true
            # Windows ARM64 builds use MSVC target with static CRT
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-pc-windows-msvc
            cp target/aarch64-pc-windows-msvc/$PROFILE/ricochet.exe target/binaries/ricochet-windows-arm64.exe
            cleanup_assets
            echo "✓ Built static aarch64-pc-windows-msvc successfully"
            echo "Binary location: target/binaries/ricochet-windows-arm64.exe"
            ;;
        # macOS targets
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
        "macos-x86_64"|"macos-x64"|"darwin-x86")
            echo "Building binary for x86_64-apple-darwin..."
            rustup target add x86_64-apple-darwin || true
            # macOS uses different RUSTFLAGS (no static linking flags)
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-apple-darwin
            cp target/x86_64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-x86_64
            cleanup_assets
            echo "✓ Built x86_64-apple-darwin successfully"
            echo "Binary location: target/binaries/ricochet-macos-x86_64"
            ;;
        "all")
            echo "Building binaries for all targets..."

            echo "Building static x86_64-unknown-linux-gnu..."
            rustup target add x86_64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-unknown-linux-gnu
            cp target/x86_64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-x86_64

            echo "Building static aarch64-unknown-linux-gnu..."
            rustup target add aarch64-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-unknown-linux-gnu
            cp target/aarch64-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-aarch64

            echo "Building static riscv64gc-unknown-linux-gnu..."
            rustup target add riscv64gc-unknown-linux-gnu || true
            cargo build --profile "$PROFILE" --locked --bin ricochet --target riscv64gc-unknown-linux-gnu
            cp target/riscv64gc-unknown-linux-gnu/$PROFILE/ricochet target/binaries/ricochet-linux-riscv64

            echo "Building static x86_64-pc-windows-gnu..."
            rustup target add x86_64-pc-windows-gnu || true
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-pc-windows-gnu
            cp target/x86_64-pc-windows-gnu/$PROFILE/ricochet.exe target/binaries/ricochet-windows-x86_64.exe

            echo "Building static aarch64-pc-windows-msvc..."
            rustup target add aarch64-pc-windows-msvc || true
            RUSTFLAGS="-C target-feature=+crt-static" cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-pc-windows-msvc
            cp target/aarch64-pc-windows-msvc/$PROFILE/ricochet.exe target/binaries/ricochet-windows-arm64.exe

            echo "Building aarch64-apple-darwin..."
            rustup target add aarch64-apple-darwin || true
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target aarch64-apple-darwin
            cp target/aarch64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-arm64

            echo "Building x86_64-apple-darwin..."
            rustup target add x86_64-apple-darwin || true
            RUSTFLAGS="" cargo build --profile "$PROFILE" --locked --bin ricochet --target x86_64-apple-darwin
            cp target/x86_64-apple-darwin/$PROFILE/ricochet target/binaries/ricochet-macos-x86_64

            cleanup_assets

            echo "All binaries built successfully!"
            echo "Binaries location:"
            echo "  - Linux x86_64:      target/binaries/ricochet-linux-x86_64"
            echo "  - Linux aarch64:     target/binaries/ricochet-linux-aarch64"
            echo "  - Linux riscv64:     target/binaries/ricochet-linux-riscv64"
            echo "  - Windows x86_64:    target/binaries/ricochet-windows-x86_64.exe"
            echo "  - Windows ARM64:     target/binaries/ricochet-windows-arm64.exe"
            echo "  - macOS ARM64:       target/binaries/ricochet-macos-arm64"
            echo "  - macOS x86_64:      target/binaries/ricochet-macos-x86_64"
            ;;
        *)
            echo "Unknown target: {{target}}"
            echo "Available targets:"
            echo "  - linux-x86_64       Linux x86_64"
            echo "  - linux-aarch64      Linux ARM64"
            echo "  - linux-riscv64      Linux RISC-V 64"
            echo "  - windows-x86_64     Windows x86_64"
            echo "  - windows-arm64      Windows ARM64"
            echo "  - macos-arm64        macOS ARM64"
            echo "  - macos-x86_64       macOS x86_64"
            echo "  - all                All targets"
            echo ""
            echo "Legacy aliases (deprecated):"
            echo "  - x86_64             -> linux-x86_64"
            echo "  - aarch64            -> linux-aarch64"
            echo "  - riscv64 (riscv)    -> linux-riscv64"
            echo "  - windows (win)      -> windows-x86_64"
            exit 1
            ;;
    esac


# Install debug version locally
install:
	cargo install --path . --debug

build-local PROFILE="release":
    cargo build --profile {{PROFILE}} --bin ricochet
	
# Move built binary to local bin
move-cli-local PROFILE="release": (build-local PROFILE)
    sudo cp target/{{PROFILE}}/ricochet ~/.local/bin/ricochet-dev

# generate CLI documentation
docs:
    @echo "Generating CLI documentation..."
    @mkdir -p docs
    @cargo run --quiet -- generate-docs > docs/cli-commands.md 2>/dev/null
    @echo "✓ Documentation generated: docs/cli-commands.md"

# check if CLI documentation is up-to-date
docs-check:
    @echo "Checking if CLI documentation is up-to-date..."
    @mkdir -p docs
    @cargo run --quiet -- generate-docs > /tmp/cli-commands-generated.md 2>/dev/null
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
