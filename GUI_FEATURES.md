# GUI Features Documentation

This document provides detailed information about the GUI features of mrpack-downloader.

## Overview

The mrpack-downloader GUI provides a modern, user-friendly interface for downloading and installing Modrinth modpacks. Built with the egui framework, it offers a responsive and attractive experience.

## Main Features

### 1. File Selection

**Modpack File Browser**
- Click "Browse..." to select a `.mrpack` file
- Supported file picker with filter for `.mrpack` files
- Visual feedback showing selected file path with icon

**Output Directory Browser**
- Click "Browse..." to select installation directory
- Folder picker dialog
- Path display with folder icon

### 2. Settings Panel

The settings panel is collapsible and provides the following options:

**Server Mode** 💻
- Enable to download server version of the modpack
- Includes server-specific overrides
- Excludes client-only mods

**Include Optional Mods** 📦
- When enabled, all optional mods are downloaded automatically
- When disabled, optional mods are skipped
- Useful for minimal installations

**Skip Hash Verification** ⚠️
- Disables SHA-1 and SHA-512 hash checking
- Speeds up installation but reduces security
- Not recommended for production use

**Skip Host Checking** 🔓
- Bypasses download host whitelist verification
- Allows downloads from non-standard sources
- Use with caution

**Concurrent Downloads**
- Slider to adjust parallel download count (1-20)
- Default: 5 concurrent downloads
- Higher values = faster but more network usage

### 3. Modpack Information Display

When a modpack is loaded, the GUI shows:

**Basic Information**
- Modpack name (highlighted)
- Version identifier
- Summary/description (if available)

**Dependencies**
- List of required loaders (Forge, Fabric, etc.)
- Minecraft version
- Other dependencies with versions

**Download Statistics**
- Total number of files to download
- Total download size (formatted in GB/MB/KB)

### 4. Progress Tracking

During download, the GUI displays:

**File Progress**
- Current file number / Total files
- Percentage complete
- Progress bar visualization

**Byte Progress**
- Downloaded bytes / Total bytes
- Formatted sizes (GB/MB/KB/B)
- Secondary progress bar

**Current File**
- Name of file being downloaded
- Real-time updates

### 5. Status Indicators

The application shows different states:

**Idle** - Ready to load a modpack
- "Load Modpack" button visible
- Requires both input file and output directory

**Loading Index** ⏳
- Parsing .mrpack file
- Reading modpack metadata

**Ready to Download** ⬇️
- Shows modpack information
- "Start Download" button available

**Downloading** ⏬
- Progress bars active
- Real-time status updates

**Completed** ✅
- Success message
- "Reset" button to start over

**Error** ❌
- Error message display
- "Reset" button to retry

## User Interface Design

### Color Scheme

**Background**: Dark theme (RGB: 30, 30, 35)
- Easy on the eyes
- Professional appearance

**Accent Colors**:
- Primary green: RGB(30, 180, 100) - for headers
- Info blue: RGB(100, 200, 255) - for file paths
- Success green: RGB(0, 255, 0) - for completion
- Error red: RGB(255, 100, 100) - for errors

**Text**: Light gray (RGB: 220, 220, 220)
- High contrast for readability

### Layout

**Header Section**
- Application title with emoji 🎮
- Subtitle with description
- Separator line

**Main Content** (Scrollable)
- File selection area
- Settings panel
- Modpack information (when loaded)
- Progress indicators (when downloading)

**Action Buttons**
- Context-sensitive button display
- Large, clearly labeled buttons
- Disabled states for invalid actions

### Icons and Emojis

The GUI uses emojis for visual enhancement:
- 🎮 Application title
- 📁 File selection section
- 📦 Modpack file
- 📂 Output directory
- ⚙️ Settings
- 💻 Server mode
- 📋 Modpack information
- ⏬ Download progress
- ✅ Success
- ❌ Error

## Keyboard and Mouse Support

**Mouse**:
- Click buttons to perform actions
- Drag sliders to adjust values
- Scroll to view content

**Keyboard**:
- Tab to navigate between elements
- Space/Enter to activate buttons
- Arrow keys for slider adjustment

## Error Handling

The GUI provides clear error messages for:
- Invalid .mrpack files
- Missing index files
- Download failures
- File system errors
- Network issues

Each error includes:
- Clear description of the problem
- Reset button to retry
- Persistent display until dismissed

## Performance

**Memory Usage**: ~50-100MB during operation
**CPU Usage**: Minimal during idle, active during downloads
**Network**: Configurable concurrent downloads (1-20)
**Disk I/O**: Efficient streaming to disk

## Accessibility

- High contrast text and backgrounds
- Clear visual hierarchy
- Large, clickable buttons
- Descriptive labels and tooltips
- Scrollable content areas

## Platform Support

The GUI works on:
- **Linux**: With GTK3 installed
- **Windows**: Native Windows API
- **macOS**: Native Cocoa support

## Tips for Best Experience

1. **Monitor Progress**: Keep the window open during downloads
2. **Adjust Concurrent Downloads**: Lower for slow connections, higher for fast
3. **Use Settings Panel**: Customize installation to your needs
4. **Check Modpack Info**: Verify dependencies before downloading
5. **Enable Hash Verification**: Keep enabled for security (default)

## Future Enhancements

Potential features for future versions:
- Download pause/resume
- Queue multiple modpacks
- History of installed modpacks
- Update checking
- Custom themes
- More detailed progress information
- Download speed indicator
