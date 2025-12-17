# Release Notes - v0.4.2 GUI Edition

## 🎉 Major Update: Native GUI Added!

This release adds a modern, user-friendly graphical interface while maintaining full CLI compatibility.

## ✨ New Features

### Native GUI
- **Modern Interface**: Beautiful dark theme with gradient effects
- **File Selection**: Native file browser dialogs for .mrpack files and output directories
- **Progress Tracking**: Real-time download progress with visual indicators
- **Settings Panel**: Easy-to-use collapsible settings panel
- **Modpack Info**: Detailed display of modpack metadata, dependencies, and file counts
- **Cross-Platform**: Works on Windows, Linux, and macOS

### GUI Features
- 📁 Native file/folder selection dialogs
- ⚙️ Collapsible settings with all CLI options
- 📊 Real-time progress bars showing files and bytes
- 📋 Modpack information preview before download
- 💻 Server mode toggle
- 🔧 Concurrent downloads slider (1-20)
- ✅ Clear status indicators (Idle, Loading, Downloading, Complete, Error)

### CLI Mode (Preserved)
- All original CLI features remain fully functional
- New `--gui` flag to explicitly launch GUI
- GUI launches by default when no arguments provided
- Perfect for automation and scripting

## 🚀 Performance
- Optimized release build with LTO
- Binary size: 14MB (Linux), 6.3MB (Windows)
- Fully stripped and optimized

## 📦 Downloads

### Windows (x64)
- **File**: `mrpack-downloader-v0.4.2-windows-x64.zip` (3.2 MB)
- **Binary**: `mrpack-downloader-windows-x64.exe` (6.3 MB)
- **SHA256**: See `.sha256` file

### Linux (x64)
- **File**: `mrpack-downloader-v0.4.2-linux-x64.tar.gz` (6.0 MB)
- **Binary**: `mrpack-downloader-linux-x64` (14 MB)
- **SHA256**: See `.sha256` file

## 📖 Usage

### GUI Mode (Default)
```bash
# Windows
mrpack-downloader-windows-x64.exe

# Linux
./mrpack-downloader-linux-x64
```

### CLI Mode
```bash
# Windows
mrpack-downloader-windows-x64.exe modpack.mrpack output-dir --server

# Linux
./mrpack-downloader-linux-x64 modpack.mrpack output-dir --server -j 10
```

## 🔧 Technical Details

### Dependencies Added
- `egui 0.29` - Immediate mode GUI framework
- `eframe 0.29` - Native windowing
- `rfd 0.15` - Native file dialogs

### Architecture
- **Modular Design**: Core logic shared between GUI and CLI
- **State Machine**: Clean state transitions in GUI
- **Async Runtime**: Proper tokio runtime handling for both modes

## 🐛 Bug Fixes
- Fixed spelling errors in code comments
- Improved CLI error messages
- Better async runtime handling

## 📚 Documentation
- Updated README with GUI and CLI usage
- Added BUILD.md with cross-platform build instructions
- Added GUI_FEATURES.md with detailed feature documentation
- Added IMPLEMENTATION_SUMMARY.md

## ⚡ Quick Start

1. Download the appropriate archive for your platform
2. Extract the archive
3. Run the executable
4. GUI will launch automatically - select your .mrpack file and output directory
5. Adjust settings if needed
6. Click "Start Download"

## 🔐 Verification

Verify the integrity of your download:

**Windows:**
```powershell
certutil -hashfile mrpack-downloader-windows-x64.exe SHA256
```

**Linux:**
```bash
sha256sum -c mrpack-downloader-linux-x64.sha256
```

## 🙏 Credits

Original CLI tool by JohnTheCoolingFan
GUI implementation and enhancements by GitHub Copilot

## 📄 License

MIT License - See LICENSE file for details
