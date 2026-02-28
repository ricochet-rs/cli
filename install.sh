#!/bin/sh
# Ricochet CLI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh

set -e

VERSION="${RICOCHET_VERSION:-0.4.0}"
GITHUB_RELEASES_BASE="https://github.com/ricochet-rs/cli/releases/download/v${VERSION}"
S3_BASE_URL="https://hel1.your-objectstorage.com/ricochet-cli/v${VERSION}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

# Detect Windows (Git Bash, MSYS2, MinGW, Cygwin)
case "${OS}" in
    MINGW*|MSYS*|CYGWIN*)
        IS_WINDOWS=1
        OS="Windows"
        ;;
    *)
        IS_WINDOWS=0
        ;;
esac

# Determine default install directory
if [ -n "${RICOCHET_INSTALL_DIR:-}" ]; then
    INSTALL_DIR="${RICOCHET_INSTALL_DIR}"
elif [ "$(id -u)" = "0" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ "${IS_WINDOWS}" = "1" ]; then
    INSTALL_DIR="$HOME/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

case "${OS}" in
    Darwin*)
        case "${ARCH}" in
            arm64|aarch64)
                TARBALL="ricochet-${VERSION}-darwin-arm64.tar.gz"
                BINARY_NAME="ricochet"
                BASE_URL="${S3_BASE_URL}"
                ;;
            x86_64)
                TARBALL="ricochet-${VERSION}-darwin-x86_64.tar.gz"
                BINARY_NAME="ricochet"
                BASE_URL="${S3_BASE_URL}"
                ;;
            *)
                echo "Unsupported macOS architecture: ${ARCH}"
                exit 1
                ;;
        esac
        ;;
    Linux*)
        case "${ARCH}" in
            x86_64)
                TARBALL="ricochet-${VERSION}-linux-x86_64.tar.gz"
                BINARY_NAME="ricochet-${VERSION}-linux-x86_64"
                BASE_URL="${GITHUB_RELEASES_BASE}"
                ;;
            aarch64|arm64)
                TARBALL="ricochet-${VERSION}-linux-aarch64.tar.gz"
                BINARY_NAME="ricochet-${VERSION}-linux-aarch64"
                BASE_URL="${GITHUB_RELEASES_BASE}"
                ;;
            *)
                echo "Unsupported Linux architecture: ${ARCH}"
                exit 1
                ;;
        esac
        ;;
    Windows)
        case "${ARCH}" in
            x86_64|x86-64)
                TARBALL="ricochet-${VERSION}-windows-x86_64.exe.tar.gz"
                BINARY_NAME="ricochet-${VERSION}-windows-x86_64.exe"
                BASE_URL="${GITHUB_RELEASES_BASE}"
                ;;
            *)
                echo "Unsupported Windows architecture: ${ARCH}"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unsupported operating system: ${OS}"
        exit 1
        ;;
esac

URL="${BASE_URL}/${TARBALL}"

echo "Installing Ricochet CLI v${VERSION} (${OS} ${ARCH})..."

# Create temporary directory
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

# Download and extract
echo "Downloading from ${URL}..."
if command -v curl > /dev/null 2>&1; then
    curl -fsSL "${URL}" -o "${TMP_DIR}/${TARBALL}"
elif command -v wget > /dev/null 2>&1; then
    wget -q "${URL}" -O "${TMP_DIR}/${TARBALL}"
else
    echo "Error: curl or wget is required"
    exit 1
fi

echo "Extracting..."
tar -xzf "${TMP_DIR}/${TARBALL}" -C "${TMP_DIR}"

# Determine final binary name
FINAL_NAME="ricochet"

# Ensure install directory exists
mkdir -p "${INSTALL_DIR}" 2>/dev/null || true

# Move the binary
if [ -w "${INSTALL_DIR}" ]; then
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${FINAL_NAME}"
else
    echo "Error: Cannot write to ${INSTALL_DIR}"
    echo "Set RICOCHET_INSTALL_DIR to a writable location, or run as root."
    exit 1
fi

chmod +x "${INSTALL_DIR}/${FINAL_NAME}"

echo "✓ Ricochet CLI installed successfully!"
echo "Binary installed to: ${INSTALL_DIR}/${FINAL_NAME}"
echo ""

# Check if directory is in PATH
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        echo "Run 'ricochet --help' to get started."
        ;;
    *)
        echo "⚠️  Note: ${INSTALL_DIR} is not in your PATH."
        echo ""
        echo "To add it, run:"
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        echo "To make it permanent, add that line to your shell profile (~/.bashrc, ~/.zshrc, etc.)."
        echo ""
        echo "For now, you can run: ${INSTALL_DIR}/${FINAL_NAME} --help"
        ;;
esac
