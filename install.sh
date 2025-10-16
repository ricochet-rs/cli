#!/bin/sh
# Ricochet CLI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh

set -e

VERSION="${RICOCHET_VERSION:-0.1.0}"
INSTALL_DIR="${RICOCHET_INSTALL_DIR:-/usr/local/bin}"
BASE_URL="https://hel1.your-objectstorage.com/ricochet-cli/v${VERSION}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
    Darwin*)
        case "${ARCH}" in
            arm64|aarch64)
                TARBALL="ricochet-${VERSION}-darwin-arm64.tar.gz"
                ;;
            x86_64)
                TARBALL="ricochet-${VERSION}-darwin-x86_64.tar.gz"
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
                TARBALL="ricochet-${VERSION}.x86_64-unknown-linux-gnu.tar.gz"
                ;;
            aarch64|arm64)
                TARBALL="ricochet-${VERSION}.aarch64-unknown-linux-gnu.tar.gz"
                ;;
            *)
                echo "Unsupported Linux architecture: ${ARCH}"
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

# Install binary
echo "Installing to ${INSTALL_DIR}..."
if [ -w "${INSTALL_DIR}" ]; then
    mv "${TMP_DIR}/ricochet" "${INSTALL_DIR}/ricochet"
else
    sudo mv "${TMP_DIR}/ricochet" "${INSTALL_DIR}/ricochet"
fi

chmod +x "${INSTALL_DIR}/ricochet"

echo "✓ Ricochet CLI installed successfully!"
echo ""
echo "Run 'ricochet --help' to get started."
