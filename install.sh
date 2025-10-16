#!/bin/sh
# Ricochet CLI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh

set -e

VERSION="${RICOCHET_VERSION:-0.1.0}"
INSTALL_DIR="${RICOCHET_INSTALL_DIR:-/usr/local/bin}"
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
                # On Windows, use AppData\Local\Programs if not specified
                if [ -z "${RICOCHET_INSTALL_DIR:-}" ]; then
                    # C:\Users\<username>\AppData\Local\Programs is typically in PATH
                    INSTALL_DIR="$HOME/AppData/Local/Programs"
                    mkdir -p "$INSTALL_DIR" 2>/dev/null || true
                    
                    # If that doesn't exist or isn't writable, try other locations
                    if [ ! -w "$INSTALL_DIR" ]; then
                        if [ -d "$HOME/.local/bin" ] && [ -w "$HOME/.local/bin" ]; then
                            INSTALL_DIR="$HOME/.local/bin"
                        elif [ -d "$HOME/bin" ] && [ -w "$HOME/bin" ]; then
                            INSTALL_DIR="$HOME/bin"
                        else
                            # Create .local/bin as fallback
                            INSTALL_DIR="$HOME/.local/bin"
                            mkdir -p "$INSTALL_DIR"
                        fi
                    fi
                fi
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
if [ "${IS_WINDOWS}" = "1" ]; then
    FINAL_NAME="ricochet.exe"
else
    FINAL_NAME="ricochet"
fi

# Move the binary
if [ -w "${INSTALL_DIR}" ]; then
    mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${FINAL_NAME}"
else
    if [ "${IS_WINDOWS}" = "1" ]; then
        # On Windows, if we can't write to the dir, just fail (no sudo)
        echo "Error: Cannot write to ${INSTALL_DIR}"
        echo "Please run this script with appropriate permissions or set RICOCHET_INSTALL_DIR to a writable location."
        exit 1
    else
        sudo mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${FINAL_NAME}"
    fi
fi

chmod +x "${INSTALL_DIR}/${FINAL_NAME}"

echo "✓ Ricochet CLI installed successfully!"
echo "Binary installed to: ${INSTALL_DIR}/${FINAL_NAME}"
echo ""

if [ "${IS_WINDOWS}" = "1" ]; then
    # Check if directory is in PATH
    if echo "$PATH" | grep -q "${INSTALL_DIR}"; then
        echo "Run 'ricochet --help' to get started."
    else
        echo "⚠️  Note: ${INSTALL_DIR} is not in your PATH."
        echo ""
        echo "To add it to your PATH, run:"
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        echo "Or add this line to your ~/.bashrc or ~/.bash_profile:"
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
        echo ""
        echo "For now, you can run: ${INSTALL_DIR}/${FINAL_NAME} --help"
    fi
else
    echo "Run 'ricochet --help' to get started."
fi
