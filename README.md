# Modpack Downloader
Download Modrinth (.mrpack) and CurseForge (.zip) modpacks

## Features

- 🎮 **Modern GUI** - Beautiful and intuitive graphical user interface
- 💻 **CLI Support** - Full command-line interface for automation
- ⚡ **Fast Downloads** - Concurrent downloads with progress tracking
- 🔒 **Secure** - Hash verification for downloaded files (Modrinth)
- 🎯 **Smart Filtering** - Optional mods selection and environment-based filtering
- 📦 **Complete** - Handles overrides and server/client specific files
- 🟢 **Modrinth Support** - Full support for .mrpack modpacks
- 🟧 **CurseForge Support** - Full support for CurseForge .zip modpacks with automatic mod loader download

## Installation

### Pre-built Binaries (Recommended)

Download pre-built binaries from the [releases page](https://github.com/bayusegara27/mrpack-downloader/releases).

**Windows:**
1. Download `mrpack-downloader-v0.5.0-windows-x64.zip`
2. Extract the ZIP file
3. Run `mrpack-downloader-windows-x64.exe`

**Linux:**
1. Download `mrpack-downloader-v0.5.0-linux-x64.tar.gz`
2. Extract: `tar -xzf mrpack-downloader-v0.5.0-linux-x64.tar.gz`
3. Run: `./mrpack-downloader-linux-x64`

### From Source
```bash
cargo build --release
```

The binary will be available at `target/release/mrpack-downloader`.

## Usage

### GUI Mode (Default)

Simply run the executable without arguments to launch the graphical interface:

```bash
mrpack-downloader
```

Or explicitly:

```bash
mrpack-downloader --gui
```

The GUI provides:
- **File Selection**: Easy browsing for .mrpack and .zip files and output directories
- **Format Detection**: Automatic detection of Modrinth or CurseForge modpack format
- **Modpack Information**: Detailed view of modpack name, version, dependencies
- **Settings Panel**: Configure server mode, optional mods, hash checking, and concurrent downloads
- **Progress Tracking**: Real-time download progress with file counts and sizes
- **Visual Feedback**: Beautiful, modern interface with helpful icons and colors

### CLI Mode

For command-line usage and automation:

```bash
# Modrinth modpacks
mrpack-downloader <input-file.mrpack> <output-directory>

# CurseForge modpacks
mrpack-downloader <input-file.zip> <output-directory>
```

#### CLI Options

- `-s, --server` - Download as server version
- `-i, --ignore-hashes` - Skip hash verification (Modrinth only)
- `--skip-host-check` - Skip download host checking (Modrinth only)
- `-j, --jobs <NUM>` - Set concurrent downloads (default: 5)
- `-u, --unattended` - Skip all confirmation prompts
- `-h, --help` - Show help information
- `-V, --version` - Show version

#### CLI Examples

```bash
# Basic usage with Modrinth modpack
mrpack-downloader modpack.mrpack ./minecraft

# CurseForge modpack
mrpack-downloader curseforge-modpack.zip ./minecraft

# Server installation with 10 concurrent downloads
mrpack-downloader modpack.mrpack ./server --server -j 10

# Unattended installation
mrpack-downloader modpack.mrpack ./minecraft --unattended
```

## Supported Modpack Formats

### Modrinth (.mrpack)
- Full support for the Modrinth modpack format
- SHA1/SHA512 hash verification for downloaded files
- Support for overrides (client and server specific)
- Environment-based mod filtering

### CurseForge (.zip)
- Support for CurseForge modpack format (manifest.json)
- Automatic download of mods from CurseForge API
- Automatic Forge/Fabric mod loader installer download
- Support for overrides
- Handles resource packs and shader packs

## Screenshots

The GUI features a modern dark theme with:
- Gradient header with application title
- File browser dialogs for easy selection
- Format detection indicator (Modrinth 🟢 / CurseForge 🟧)
- Collapsible settings panel
- Detailed modpack information display
- Real-time progress bars
- Status indicators with emojis

## Building for Release

```bash
cargo build --release
```

The optimized binary will be created at `target/release/mrpack-downloader`.

## Requirements

- Rust 1.91.1 or later (for building from source)
- Internet connection for downloading mods

## License

MIT License - see LICENSE file for details
