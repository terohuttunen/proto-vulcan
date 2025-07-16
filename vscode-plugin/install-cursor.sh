#!/bin/bash

# Proto-Vulcan Cursor IDE Extension Installer
# This script installs the Proto-Vulcan syntax highlighting extension for Cursor IDE

set -e

echo "Installing Proto-Vulcan Extension for Cursor IDE..."

# Get the Cursor extensions directory
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    EXTENSIONS_DIR="$USERPROFILE/.cursor/extensions"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    EXTENSIONS_DIR="$HOME/.cursor/extensions"
else
    # Linux
    EXTENSIONS_DIR="$HOME/.cursor/extensions"
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

echo "Proto-Vulcan extension installed successfully for Cursor IDE!"
echo "Please restart Cursor to activate the extension."
echo ""
echo "Test the extension by opening any .pv file in Cursor."
echo "The extension will provide syntax highlighting for Proto-Vulcan files." 