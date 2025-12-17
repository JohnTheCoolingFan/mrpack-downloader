# How to Create a GitHub Release

This guide explains how to create releases for mrpack-downloader that users can download.

## Prerequisites

The Windows and Linux binaries have been built and are available in the `releases/` directory:
- `mrpack-downloader-v0.4.2-windows-x64.zip` (3.2 MB)
- `mrpack-downloader-v0.4.2-linux-x64.tar.gz` (6.0 MB)

## Option 1: Manual GitHub Release (Recommended)

### Steps:

1. **Go to the Releases Page**
   - Navigate to: https://github.com/bayusegara27/mrpack-downloader/releases
   - Click "Draft a new release"

2. **Create a New Tag**
   - Tag version: `v0.4.2-gui`
   - Target: `copilot/add-detailed-gui-features` branch
   - Release title: `v0.4.2 - GUI Edition`

3. **Add Release Notes**
   - Copy the content from `RELEASE_NOTES.md` into the description

4. **Upload Release Assets**
   
   Upload these files from the `releases/` directory:
   - `mrpack-downloader-v0.4.2-windows-x64.zip`
   - `mrpack-downloader-windows-x64.exe.sha256`
   - `mrpack-downloader-v0.4.2-linux-x64.tar.gz`
   - `mrpack-downloader-linux-x64.sha256`

5. **Publish**
   - Click "Publish release"

## Option 2: Using GitHub CLI

If you have `gh` CLI installed:

```bash
# Create release
gh release create v0.4.2-gui \
  --title "v0.4.2 - GUI Edition" \
  --notes-file RELEASE_NOTES.md \
  releases/mrpack-downloader-v0.4.2-windows-x64.zip \
  releases/mrpack-downloader-windows-x64.exe.sha256 \
  releases/mrpack-downloader-v0.4.2-linux-x64.tar.gz \
  releases/mrpack-downloader-linux-x64.sha256
```

## Option 3: Automated Releases (Future)

For future releases, tag your commit and push:

```bash
git tag v0.4.3
git push origin v0.4.3
```

The GitHub Actions workflow in `.github/workflows/cd.yml` will automatically:
1. Build for Windows and Linux
2. Generate checksums
3. Create a GitHub release
4. Upload the artifacts

## Verifying the Release

After creating the release:

1. Go to: https://github.com/bayusegara27/mrpack-downloader/releases
2. Verify both download links work
3. Check that checksums are included
4. Test downloading and running on Windows/Linux

## Release Artifacts Location

All release artifacts are in the `releases/` directory:

```
releases/
├── mrpack-downloader-linux-x64                  # Linux binary (14 MB)
├── mrpack-downloader-linux-x64.sha256           # Linux checksum
├── mrpack-downloader-v0.4.2-linux-x64.tar.gz   # Linux archive (6.0 MB)
├── mrpack-downloader-windows-x64.exe            # Windows binary (6.3 MB)
├── mrpack-downloader-windows-x64.exe.sha256     # Windows checksum
└── mrpack-downloader-v0.4.2-windows-x64.zip    # Windows archive (3.2 MB)
```

## Notes

- The `releases/` directory is in `.gitignore` and won't be committed
- Release artifacts should be uploaded to GitHub Releases, not committed to the repository
- Users will download directly from GitHub Releases page
- Checksums allow users to verify file integrity
