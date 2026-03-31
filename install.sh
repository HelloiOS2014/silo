#!/usr/bin/env bash
set -e

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Install silo binary from GitHub Releases.

OPTIONS:
    -p, --prefix PATH    Installation prefix (default: ~/.local)
                        Binary will be installed to PATH/bin/silo
    -h, --help          Show this help message

EXAMPLES:
    $0                                      # Install to ~/.local/bin/silo
    $0 --prefix /usr/local                  # Install to /usr/local/bin/silo
    PREFIX=/usr/local $0                    # Alternative way to set prefix

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--prefix)
            PREFIX="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64)
        ARCH="silo-macos-arm64"
        ;;
    x86_64)
        ARCH="silo-macos-x86_64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

# Get latest version
RELEASES_URL="https://api.github.com/repos/HelloiOS2014/silo/releases/latest"
VERSION="$(curl -fsSL "$RELEASES_URL" | grep -o '"tag_name":[^,]*' | cut -d'"' -f4)"
if [[ -z "$VERSION" ]]; then
    echo "Failed to fetch latest release version" >&2
    exit 1
fi

DOWNLOAD_URL="https://github.com/HelloiOS2014/silo/releases/download/$VERSION/$ARCH"
BIN_PATH="$BIN_DIR/silo"

echo "Installing silo $VERSION to $BIN_PATH..."

# Create bin directory if needed
mkdir -p "$BIN_DIR"

# Download binary
curl -fsSL "$DOWNLOAD_URL" -o "$BIN_PATH"

# Make executable
chmod +x "$BIN_PATH"

echo "Done! Add $BIN_DIR to your PATH if needed."
