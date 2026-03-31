#!/usr/bin/env bash
set -e

# Remove old cargo-installed binary
if [[ -f ~/.cargo/bin/silo ]]; then
    echo "Removing ~/.cargo/bin/silo..."
    rm ~/.cargo/bin/silo
fi

# Build and install to ~/.local/bin
echo "Building and installing silo to ~/.local/bin..."
cargo build --release
cp target/release/silo ~/.local/bin/silo

# Ensure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo ""
    echo "NOTE: Add ~/.local/bin to your PATH if not already present:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

echo "Done! Run 'silo --help' to verify."
