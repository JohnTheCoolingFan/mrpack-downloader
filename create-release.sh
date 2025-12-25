#!/bin/bash
# Automated Release Script
# This script creates a GitHub release automatically using GitHub CLI

set -e

VERSION=${1:-"v0.5.0"}
REPO="bayusegara27/mrpack-downloader"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║        Automated GitHub Release Creation                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Repository: $REPO"
echo "Version: $VERSION"
echo ""

# Check if gh CLI is available
if ! command -v gh &> /dev/null; then
    echo "❌ Error: GitHub CLI (gh) is not installed."
    echo "Install it from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo "❌ Error: Not authenticated with GitHub CLI."
    echo "Run: gh auth login"
    exit 1
fi

echo "✅ GitHub CLI authenticated"
echo ""

# Check if release notes exist
if [ ! -f "RELEASE_NOTES.md" ]; then
    echo "⚠️  Warning: RELEASE_NOTES.md not found, creating basic release notes"
    cat > /tmp/release_notes.md << 'EOF'
# Release v0.5.0 - CurseForge Support

## 🎉 New Features
- **CurseForge Modpack Support** - Now supports downloading CurseForge modpacks (.zip)
- **Automatic Format Detection** - Automatically detects whether the modpack is Modrinth or CurseForge
- **Mod Loader Download** - Automatically downloads Forge/Fabric installer for CurseForge modpacks
- **Updated GUI** - Shows modpack format indicator (🟢 Modrinth / 🟧 CurseForge)

## Features
- Modern GUI with egui/eframe
- Native file selection dialogs
- Real-time progress tracking
- Cross-platform support (Windows + Linux)
- CLI mode fully preserved
- Support for both Modrinth (.mrpack) and CurseForge (.zip) modpacks

## Downloads
Download the appropriate file for your platform and verify with the `.sha256` checksum.
EOF
    NOTES_FILE="/tmp/release_notes.md"
else
    NOTES_FILE="RELEASE_NOTES.md"
    echo "✅ Using RELEASE_NOTES.md for release description"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Creating release $VERSION..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Option 1: Trigger workflow if available
echo "🔧 Option 1: Triggering GitHub Actions workflow..."
if gh workflow run release-gui.yml -f tag="$VERSION" 2>/dev/null; then
    echo "✅ GitHub Actions workflow triggered successfully!"
    echo ""
    echo "The workflow will:"
    echo "  1. Build for Windows and Linux"
    echo "  2. Create release artifacts"
    echo "  3. Publish the release automatically"
    echo ""
    echo "Monitor progress at:"
    echo "  https://github.com/$REPO/actions"
    echo ""
    echo "Release will be available at:"
    echo "  https://github.com/$REPO/releases/tag/$VERSION"
else
    echo "⚠️  Workflow trigger failed, trying direct release creation..."
    echo ""
    
    # Option 2: Create release directly if artifacts exist
    echo "🔧 Option 2: Creating release directly with existing artifacts..."
    
    if [ ! -d "releases" ]; then
        echo "❌ Error: releases/ directory not found."
        echo "Please run ./build-release.sh first to create release artifacts."
        exit 1
    fi
    
    # Check for required files
    REQUIRED_FILES=(
        "releases/mrpack-downloader-v0.5.0-windows-x64.zip"
        "releases/mrpack-downloader-v0.5.0-linux-x64.tar.gz"
    )
    
    for file in "${REQUIRED_FILES[@]}"; do
        if [ ! -f "$file" ]; then
            echo "❌ Error: Required file not found: $file"
            echo "Please run ./build-release.sh first."
            exit 1
        fi
    done
    
    echo "✅ All required files found"
    echo ""
    
    # Create the release
    echo "Creating GitHub release..."
    gh release create "$VERSION" \
        --title "Release $VERSION" \
        --notes-file "$NOTES_FILE" \
        releases/*.tar.gz \
        releases/*.zip \
        releases/*.sha256 \
        || {
            echo "❌ Error: Failed to create release"
            echo ""
            echo "Possible reasons:"
            echo "  - Tag $VERSION already exists"
            echo "  - Insufficient permissions"
            echo "  - Network issues"
            echo ""
            echo "To delete existing tag: git push --delete origin $VERSION"
            exit 1
        }
    
    echo ""
    echo "✅ Release created successfully!"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║              🎉 Release Published! 🎉                        ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "View release at:"
echo "  https://github.com/$REPO/releases/tag/$VERSION"
echo ""
echo "Users can now download:"
echo "  • Windows: mrpack-downloader-windows-x64.zip"
echo "  • Linux: mrpack-downloader-linux-x64.tar.gz"
echo ""
