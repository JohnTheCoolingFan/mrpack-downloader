# mrpack-downloader
Download Modrinth modpacks from mrpack files

## Features

- 🎮 **Modern GUI** - Beautiful and intuitive graphical user interface
- 💻 **CLI Support** - Full command-line interface for automation
- ⚡ **Fast Downloads** - Concurrent downloads with progress tracking
- 🔒 **Secure** - Hash verification for downloaded files
- 🎯 **Smart Filtering** - Optional mods selection and environment-based filtering
- 📦 **Complete** - Handles overrides and server/client specific files

## Installation

### From Release
Download the latest release binary from the [releases page](https://github.com/bayusegara27/mrpack-downloader/releases).

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
- **File Selection**: Easy browsing for .mrpack files and output directories
- **Modpack Information**: Detailed view of modpack name, version, dependencies
- **Settings Panel**: Configure server mode, optional mods, hash checking, and concurrent downloads
- **Progress Tracking**: Real-time download progress with file counts and sizes
- **Visual Feedback**: Beautiful, modern interface with helpful icons and colors

### CLI Mode

For command-line usage and automation:

```bash
mrpack-downloader <input-file.mrpack> <output-directory>
```

#### CLI Options

- `-s, --server` - Download as server version
- `-i, --ignore-hashes` - Skip hash verification
- `--skip-host-check` - Skip download host checking
- `-j, --jobs <NUM>` - Set concurrent downloads (default: 5)
- `-u, --unattended` - Skip all confirmation prompts
- `-h, --help` - Show help information
- `-V, --version` - Show version

#### CLI Examples

```bash
# Basic usage
mrpack-downloader modpack.mrpack ./minecraft

# Server installation with 10 concurrent downloads
mrpack-downloader modpack.mrpack ./server --server -j 10

# Unattended installation
mrpack-downloader modpack.mrpack ./minecraft --unattended
```

## Screenshots

The GUI features a modern dark theme with:
- Gradient header with application title
- File browser dialogs for easy selection
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
