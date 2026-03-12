#!/bin/sh
set -e

REPO="sentrux/sentrux"
VERSION="v0.3.4"
INSTALL_DIR="/usr/local/bin"

# Detect OS and architecture
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}" in
    Darwin)
        case "${ARCH}" in
            arm64|aarch64) ARTIFACT="sentrux-darwin-arm64" ;;
            x86_64)
                echo "Error: macOS Intel (x86_64) binary not available yet."
                echo "Build from source: git clone https://github.com/${REPO} && cd sentrux && cargo build --release"
                exit 1
                ;;
            *) echo "Error: unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    Linux)
        case "${ARCH}" in
            x86_64) ARTIFACT="sentrux-linux-x86_64" ;;
            *) echo "Error: unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    *)
        echo "Error: unsupported OS: ${OS}"
        exit 1
        ;;
esac

URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"

echo "Installing sentrux ${VERSION} (${OS} ${ARCH})..."
echo "Downloading ${URL}"

TMP=$(mktemp)
if command -v curl > /dev/null 2>&1; then
    curl -fsSL "${URL}" -o "${TMP}"
elif command -v wget > /dev/null 2>&1; then
    wget -qO "${TMP}" "${URL}"
else
    echo "Error: curl or wget required"
    exit 1
fi

chmod +x "${TMP}"

if [ -w "${INSTALL_DIR}" ]; then
    mv "${TMP}" "${INSTALL_DIR}/sentrux"
else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${TMP}" "${INSTALL_DIR}/sentrux"
fi

echo "sentrux installed to ${INSTALL_DIR}/sentrux"
echo ""
echo "Run:  sentrux              # GUI mode"
echo "      sentrux --mcp        # MCP server for AI agents"
echo "      sentrux check .      # CLI rules check"
