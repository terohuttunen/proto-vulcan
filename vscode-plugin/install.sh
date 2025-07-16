#!/bin/bash

# Proto-Vulcan VSCode Extension Installer
# This script installs the Proto-Vulcan syntax highlighting extension

set -e

echo "Installing Proto-Vulcan VSCode Extension..."

# Get the VSCode extensions directory
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    EXTENSIONS_DIR="$USERPROFILE/.vscode/extensions"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    EXTENSIONS_DIR="$HOME/.vscode/extensions"
else
    # Linux
    EXTENSIONS_DIR="$HOME/.vscode/extensions"
fi

# Create extensions directory if it doesn't exist
mkdir -p "$EXTENSIONS_DIR"

# Define the target directory
TARGET_DIR="$EXTENSIONS_DIR/proto-vulcan"

# Remove existing installation if it exists
if [ -d "$TARGET_DIR" ]; then
    echo "Removing existing installation..."
    rm -rf "$TARGET_DIR"
fi

# Create the target directory
mkdir -p "$TARGET_DIR"

# Copy extension files
echo "Copying extension files..."
cp package.json "$TARGET_DIR/"
cp language-configuration.json "$TARGET_DIR/"
cp README.md "$TARGET_DIR/"
cp -r syntaxes "$TARGET_DIR/"

echo "Proto-Vulcan VSCode extension installed successfully!"
echo "Please restart VSCode to activate the extension."
echo ""
echo "Test the extension by opening any .pv file."
echo "You can try the included example.pv file to see syntax highlighting in action." 