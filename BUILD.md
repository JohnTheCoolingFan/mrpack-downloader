# Build Guide for mrpack-downloader

This guide explains how to build the mrpack-downloader from source with various configurations.

## Prerequisites

- Rust 1.91.1 or later
- Cargo (comes with Rust)
- Git (for cloning the repository)

### Platform-Specific Requirements

#### Linux
```bash
# Debian/Ubuntu
sudo apt-get install libssl-dev pkg-config libgtk-3-dev

# Fedora
sudo dnf install openssl-devel gtk3-devel

# Arch
sudo pacman -S openssl gtk3
```

#### macOS
```bash
# Using Homebrew
brew install openssl
```

#### Windows
No additional dependencies required. The build will work with MSVC or GNU toolchains.

## Building from Source

### 1. Clone the Repository
```bash
git clone https://github.com/bayusegara27/mrpack-downloader.git
cd mrpack-downloader
```

### 2. Build in Debug Mode (for development)
```bash
cargo build
```
The binary will be at `target/debug/mrpack-downloader`.

### 3. Build in Release Mode (optimized)
```bash
cargo build --release
```
The optimized binary will be at `target/release/mrpack-downloader`.

## Release Build Configuration

The project uses the following release optimizations in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-Time Optimization for smaller binary
codegen-units = 1    # Better optimization at cost of compile time
strip = true         # Remove debug symbols for smaller size
```

These settings provide:
- **Smaller binary size** (~14MB vs ~24MB unoptimized)
- **Faster execution** through aggressive optimization
- **Removed debug symbols** for production deployment

## Running Tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

## Cross-Compilation

### For Windows (from Linux)
```bash
# Install target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### For Linux (from macOS)
```bash
# Install target
rustup target add x86_64-unknown-linux-gnu

# Build (may require cross-compilation tools)
cargo build --release --target x86_64-unknown-linux-gnu
```

## Distribution

After building in release mode:

1. The binary is located at `target/release/mrpack-downloader`
2. Strip additional symbols if needed: `strip target/release/mrpack-downloader`
3. Compress for distribution: `tar -czvf mrpack-downloader-linux-x64.tar.gz -C target/release mrpack-downloader`

## Troubleshooting

### Build Fails with OpenSSL Errors
Make sure OpenSSL development libraries are installed:
```bash
# Linux
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
```

### GUI Doesn't Launch
Ensure GTK3 libraries are installed on Linux:
```bash
sudo apt-get install libgtk-3-dev
```

### Slow Compilation
The release build with LTO is intentionally slow for maximum optimization. Use `cargo build` for faster development builds.

## Development Tips

1. **Faster Builds**: Use `cargo build` during development
2. **Check Code**: Run `cargo clippy` for linting
3. **Format Code**: Run `cargo fmt` to format code
4. **Update Dependencies**: Run `cargo update` to update dependencies

## Creating Release Packages

### Linux
```bash
cargo build --release
strip target/release/mrpack-downloader
tar -czvf mrpack-downloader-v$(cargo pkgid | cut -d# -f2)-linux-x64.tar.gz \
  -C target/release mrpack-downloader README.md LICENSE
```

### Windows
```bash
cargo build --release --target x86_64-pc-windows-gnu
zip -j mrpack-downloader-windows-x64.zip \
  target/x86_64-pc-windows-gnu/release/mrpack-downloader.exe \
  README.md LICENSE
```

### macOS
```bash
cargo build --release
strip target/release/mrpack-downloader
tar -czvf mrpack-downloader-macos-x64.tar.gz \
  -C target/release mrpack-downloader README.md LICENSE
```
