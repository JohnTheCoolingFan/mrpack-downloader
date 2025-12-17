#!/bin/bash
# Release Build Script for mrpack-downloader
# This script builds release binaries for multiple platforms

set -e

VERSION=${1:-"dev"}
RELEASE_DIR="releases"

echo "Building mrpack-downloader release v${VERSION}"
echo "============================================"

# Clean previous releases
rm -rf $RELEASE_DIR
mkdir -p $RELEASE_DIR

# Build for Linux
echo ""
echo "Building for Linux (x86_64)..."
cargo build --release
cp target/release/mrpack-downloader $RELEASE_DIR/mrpack-downloader-linux-x64
strip $RELEASE_DIR/mrpack-downloader-linux-x64 2>/dev/null || true

# Build for Windows
echo ""
echo "Building for Windows (x86_64)..."
if rustup target list | grep -q "x86_64-pc-windows-gnu (installed)"; then
    echo "Windows target already installed"
else
    echo "Installing Windows target..."
    rustup target add x86_64-pc-windows-gnu
fi

cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/mrpack-downloader.exe $RELEASE_DIR/mrpack-downloader-windows-x64.exe

# Generate checksums
echo ""
echo "Generating checksums..."
cd $RELEASE_DIR
sha256sum mrpack-downloader-linux-x64 > mrpack-downloader-linux-x64.sha256
sha256sum mrpack-downloader-windows-x64.exe > mrpack-downloader-windows-x64.exe.sha256

# Create compressed archives
echo ""
echo "Creating archives..."
tar -czf mrpack-downloader-${VERSION}-linux-x64.tar.gz mrpack-downloader-linux-x64 mrpack-downloader-linux-x64.sha256
zip -q mrpack-downloader-${VERSION}-windows-x64.zip mrpack-downloader-windows-x64.exe mrpack-downloader-windows-x64.exe.sha256

cd ..

# Display results
echo ""
echo "Release build complete!"
echo "======================"
echo ""
echo "Binaries:"
ls -lh $RELEASE_DIR/mrpack-downloader-*
echo ""
echo "Checksums:"
cat $RELEASE_DIR/*.sha256
echo ""
echo "Archives:"
ls -lh $RELEASE_DIR/*.tar.gz $RELEASE_DIR/*.zip
echo ""
echo "Release artifacts are in: $RELEASE_DIR/"
