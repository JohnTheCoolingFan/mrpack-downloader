# Panduan Release Otomatis

## 📋 Ringkasan

Release sekarang sudah **sepenuhnya otomatis**! Tidak perlu manual lagi.

## 🚀 Cara Membuat Release

### Metode 1: GitHub Actions Web UI (TERMUDAH)

1. **Buka tab Actions**
   - Pergi ke: https://github.com/bayusegara27/mrpack-downloader/actions
   
2. **Pilih workflow "Release with GUI"**
   - Klik workflow di sidebar kiri
   
3. **Jalankan workflow**
   - Klik tombol **"Run workflow"** (dropdown di kanan atas)
   - Masukkan tag release: `v0.4.2-gui` (atau versi lain)
   - Klik tombol hijau **"Run workflow"**
   
4. **Tunggu selesai**
   - Workflow akan berjalan sekitar 10-15 menit
   - Build untuk Windows dan Linux
   - Create release otomatis
   
5. **Cek hasilnya**
   - Pergi ke: https://github.com/bayusegara27/mrpack-downloader/releases
   - Release baru akan muncul dengan semua files

### Metode 2: Menggunakan Script

Jika punya akses ke repository lokal:

```bash
./create-release.sh v0.4.2-gui
```

Script akan:
- Coba trigger GitHub Actions workflow
- Atau buat release langsung dengan GitHub CLI
- Upload semua artifacts

### Metode 3: Push Git Tag

```bash
git tag v0.4.2-gui
git push origin v0.4.2-gui
```

Workflow akan otomatis ter-trigger saat tag dipush.

## 📦 Apa yang Dibuat Otomatis

Workflow akan membuat:

1. **Windows Build (x64)**
   - Binary: `mrpack-downloader.exe`
   - Archive: `mrpack-downloader-windows-x64.zip`
   - Checksum: `mrpack-downloader-windows-x64.sha256`

2. **Linux Build (x64)**
   - Binary: `mrpack-downloader`
   - Archive: `mrpack-downloader-linux-x64.tar.gz`
   - Checksum: `mrpack-downloader-linux-x64.sha256`

3. **GitHub Release**
   - Otomatis publish dengan semua files
   - Release notes dari RELEASE_NOTES.md
   - Siap untuk di-download publik

## ⏱️ Timeline

| Tahap | Waktu |
|-------|-------|
| Build Linux | ~4-5 menit |
| Build Windows | ~4-5 menit |
| Package & Upload | ~1 menit |
| **Total** | **~10-15 menit** |

## ✅ Checklist

- [ ] Merge PR ini ke branch main
- [ ] Pergi ke Actions tab
- [ ] Run workflow "Release with GUI"
- [ ] Masukkan tag version
- [ ] Tunggu workflow selesai
- [ ] Cek release di https://github.com/bayusegara27/mrpack-downloader/releases
- [ ] Test download untuk Windows dan Linux

## 🔧 Troubleshooting

### Workflow gagal?

1. Cek logs di Actions tab
2. Pastikan tag belum ada (jika error "tag already exists")
3. Delete tag dan coba lagi:
   ```bash
   git tag -d v0.4.2-gui
   git push --delete origin v0.4.2-gui
   ```

### Butuh versi baru?

Ganti tag number:
- `v0.4.3-gui`
- `v0.5.0-gui`
- `v1.0.0`

## 📝 Catatan

- Workflow file: `.github/workflows/release-gui.yml`
- Script: `create-release.sh`
- Workflow bisa di-trigger manual atau otomatis via tag
- Tidak ada step manual untuk build atau upload
- Semua otomatis dari source code di repository

## 🎉 Kesimpulan

**Sekarang release sepenuhnya otomatis!**
Cukup klik "Run workflow" di GitHub, tunggu 10 menit, dan release siap!

Tidak perlu:
- ❌ Build manual
- ❌ Upload files manual
- ❌ Create release manual
- ❌ Generate checksums manual

Semua otomatis! ✅
