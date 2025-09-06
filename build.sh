#!/bin/bash

echo "Building zellij-todo plugin..."

# Check if wasm32-wasip1 target is installed
if ! rustup target list --installed | grep -q wasm32-wasip1; then
    echo "Installing wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

# Build the plugin
cargo build "$@"

if [ $? -eq 0 ]; then
    echo "✓ Build successful!"
    echo "Plugin location: target/wasm32-wasip1/debug/zellij-todo.wasm"
    echo ""
    echo "To run the plugin:"
    echo "  zellij plugin -f -- file:$(pwd)/target/wasm32-wasip1/debug/zellij-todo.wasm"
    echo ""
    echo "To use the development environment:"
    echo "  zellij -l zellij.kdl"
else
    echo "✗ Build failed!"
    exit 1
fi
