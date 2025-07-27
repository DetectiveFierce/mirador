#!/bin/bash

# Mirador Release Script
# Usage: ./scripts/create_release.sh <version> <platform>

set -e

VERSION=$1
PLATFORM=$2

if [ -z "$VERSION" ] || [ -z "$PLATFORM" ]; then
    echo "Usage: $0 <version> <platform>"
    echo "Example: $0 v0.0.1a Linux"
    exit 1
fi

# Create releases directory and version subfolder if they don't exist
mkdir -p releases
mkdir -p "releases/$VERSION"

# Build the project
echo "Building Mirador..."
cargo build --release

# Copy binary with proper naming
BINARY_NAME="Mirador-${VERSION}-${PLATFORM}"
cp target/release/mirador "releases/${VERSION}/${BINARY_NAME}"

# Make executable
chmod +x "releases/${VERSION}/${BINARY_NAME}"

echo "Release created: releases/${VERSION}/${BINARY_NAME}"
echo "Binary size: $(du -h releases/${VERSION}/${BINARY_NAME} | cut -f1)"

# Create checksum
sha256sum "releases/${VERSION}/${BINARY_NAME}" > "releases/${VERSION}/${BINARY_NAME}.sha256"

echo "Checksum created: releases/${VERSION}/${BINARY_NAME}.sha256" 