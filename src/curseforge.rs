use std::path::Path;

use async_zip::tokio::read::fs::ZipFileReader;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::Client;
use thiserror::Error;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;

use crate::core::{sanitize_path_check, sanitize_zip_filename};
use crate::schemas::{CurseForgeManifest, CurseForgeProjectInfo};

// Constants
const INFO_URL: &str = "https://api.cfwidget.com/";
const DOWNLOAD_URL_TEMPLATE: &str = "https://www.curseforge.com/api/v1/mods/{project_id}/files/{file_id}/download";
const FORGE_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}/forge-{game_version}-{forge_version}-installer.jar";
const FORGE_URL_OLD: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}-{game_version}/forge-{game_version}-{forge_version}-{game_version}-installer.jar";
const FABRIC_URL: &str = "https://maven.fabricmc.net/net/fabricmc/fabric-installer/1.0.1/fabric-installer-1.0.1.jar";
const FABRIC_FILE_NAME: &str = "fabric-installer-1.0.1.jar";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
const FILE_DOWNLOAD_MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Error)]
pub enum CurseForgeError {
    #[error(transparent)]
    AsyncZip(#[from] async_zip::error::ZipError),
    #[error("manifest.json was not found within the modpack file")]
    ManifestNotFound,
    #[error("Failed to parse manifest.json: {0}")]
    ManifestParseError(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to get project info for project {project_id}: {message}")]
    ProjectInfoError { project_id: u64, message: String },
    #[error("Download failed after {attempts} attempts: {url}")]
    DownloadFailed { url: String, attempts: u32 },
    #[error("HTTP error: {message}")]
    HttpError { message: String },
    #[error("File validation error: {message}")]
    FileValidationError { message: String },
    #[error("Task execution error: {message}")]
    TaskError { message: String },
}

/// Read and parse CurseForge manifest.json from a zip file
pub async fn read_curseforge_manifest(zip: &mut ZipFileReader) -> Result<CurseForgeManifest, CurseForgeError> {
    let mut buf = Vec::new();
    let mut found = false;
    
    for (i, file) in zip.file().entries().iter().enumerate() {
        if file.filename().as_bytes() == "manifest.json".as_bytes() {
            found = true;
            let mut entry = zip.reader_with_entry(i).await?;
            entry.read_to_end_checked(&mut buf).await?;
            break;
        }
    }
    
    if !found {
        return Err(CurseForgeError::ManifestNotFound);
    }
    
    serde_json::from_slice(&buf).map_err(CurseForgeError::ManifestParseError)
}

/// Get project info from CurseForge API
pub async fn get_project_info(client: &Client, project_id: u64) -> Result<CurseForgeProjectInfo, CurseForgeError> {
    let url = format!("{}{}", INFO_URL, project_id);
    
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(CurseForgeError::ProjectInfoError {
            project_id,
            message: format!("HTTP {}", response.status()),
        });
    }
    
    let info: CurseForgeProjectInfo = response.json().await.map_err(|e: reqwest::Error| CurseForgeError::ProjectInfoError {
        project_id,
        message: e.to_string(),
    })?;
    
    Ok(info)
}

/// Get the directory name based on project type
fn get_directory_for_type(project_type: &str) -> &'static str {
    match project_type {
        "Mods" => "mods",
        "Resource Packs" => "resourcepacks",
        "Shaders" => "shaderpacks",
        _ => "mods",
    }
}

/// Progress callback type for CurseForge downloads
pub type CurseForgeProgressCallback = Option<Box<dyn Fn(usize, usize, String, u64, u64) + Send + Sync>>;

/// Download all files from CurseForge manifest
pub async fn download_curseforge_files(
    manifest: &CurseForgeManifest,
    output_dir: &Path,
    jobs: usize,
    progress_callback: CurseForgeProgressCallback,
) -> Result<(), CurseForgeError> {
    let mpb = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());
    let client = Client::new();
    let total_files = manifest.files.len();
    let completed_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let downloaded_bytes = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let progress_callback = std::sync::Arc::new(progress_callback);
    
    // Process files with limited concurrency
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(jobs));
    let mut handles = Vec::new();
    
    for (i, file) in manifest.files.iter().enumerate() {
        let client = client.clone();
        let output_dir = output_dir.to_path_buf();
        let mpb = mpb.clone();
        let semaphore = semaphore.clone();
        let completed_count = completed_count.clone();
        let downloaded_bytes = downloaded_bytes.clone();
        let progress_callback = progress_callback.clone();
        let project_id = file.project_id;
        let file_id = file.file_id;
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            
            // Get project info
            let project_info = match get_project_info(&client, project_id).await {
                Ok(info) => info,
                Err(e) => {
                    eprintln!("[{}/{}] Failed to get project info for {}: {}", i + 1, total_files, project_id, e);
                    return Ok::<_, CurseForgeError>(());
                }
            };
            
            // Find the file in project info
            let file_info = project_info.files.iter().find(|f| f.id == file_id);
            let (file_name, file_size) = match file_info {
                Some(f) => (f.name.clone(), f.filesize),
                None => {
                    eprintln!("[{}/{}] File {} not found in project {}", i + 1, total_files, file_id, project_id);
                    return Ok(());
                }
            };
            
            // Determine target directory
            let target_dir = output_dir.join(get_directory_for_type(&project_info.project_type));
            let target_path = target_dir.join(&file_name);
            
            // Skip if file already exists with correct size
            if target_path.exists() {
                if let Ok(metadata) = tokio::fs::metadata(&target_path).await {
                    if metadata.len() == file_size {
                        println!("[{}/{}] File {} already exists", i + 1, total_files, file_name);
                        let current = completed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        let current_bytes = downloaded_bytes.fetch_add(file_size, std::sync::atomic::Ordering::SeqCst) + file_size;
                        if let Some(ref callback) = *progress_callback {
                            callback(current, total_files, file_name, current_bytes, 0);
                        }
                        return Ok(());
                    }
                }
            }
            
            // Create directory if needed
            if !target_dir.exists() {
                create_dir_all(&target_dir).await?;
            }
            
            // Build download URL
            let download_url = DOWNLOAD_URL_TEMPLATE
                .replace("{project_id}", &project_id.to_string())
                .replace("{file_id}", &file_id.to_string());
            
            // Download with retry
            let pb = mpb.add(
                ProgressBar::with_draw_target(Some(file_size), ProgressDrawTarget::stdout())
                    .with_message(format!("Downloading {}", file_name))
                    .with_style(
                        ProgressStyle::default_bar()
                            .template("{msg}\n{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                            .expect("Incorrect template provided")
                            .progress_chars("#> ")
                    ),
            );
            
            let mut attempts = 0;
            loop {
                attempts += 1;
                match download_file_attempt(&client, &download_url, &target_path, file_size, &pb).await {
                    Ok(()) => {
                        pb.finish_with_message(format!("Downloaded {}", file_name));
                        break;
                    }
                    Err(e) => {
                        eprintln!("[{}/{}] Download attempt {} failed: {}", i + 1, total_files, attempts, e);
                        if attempts >= FILE_DOWNLOAD_MAX_ATTEMPTS {
                            pb.finish_with_message(format!("Failed to download {}", file_name));
                            return Err(CurseForgeError::DownloadFailed {
                                url: download_url,
                                attempts,
                            });
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
            
            // Update progress
            let current = completed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let current_bytes = downloaded_bytes.fetch_add(file_size, std::sync::atomic::Ordering::SeqCst) + file_size;
            if let Some(ref callback) = *progress_callback {
                callback(current, total_files, file_name, current_bytes, 0);
            }
            
            Ok(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all downloads to complete
    for handle in handles {
        handle.await.map_err(|e| CurseForgeError::TaskError {
            message: format!("Task join error: {}", e),
        })??;
    }
    
    Ok(())
}

async fn download_file_attempt(
    client: &Client,
    url: &str,
    path: &Path,
    expected_size: u64,
    pb: &ProgressBar,
) -> Result<(), CurseForgeError> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(CurseForgeError::HttpError {
            message: format!("HTTP {} when downloading {}", response.status(), url),
        });
    }
    
    let mut file = File::create(path).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }
    
    file.flush().await?;
    
    // Verify file size
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.len() != expected_size && expected_size > 0 {
        return Err(CurseForgeError::FileValidationError {
            message: format!("Size mismatch: expected {} bytes, got {} bytes", expected_size, metadata.len()),
        });
    }
    
    Ok(())
}

/// Extract overrides from CurseForge modpack
pub async fn extract_curseforge_overrides(
    zip: &mut ZipFileReader,
    overrides_folder: &str,
    output_dir: &Path,
) {
    for (i, entry) in zip.file().entries().iter().enumerate() {
        let filename = entry.filename().as_str().unwrap();
        if filename.starts_with(&format!("{}/", overrides_folder)) {
            println!("Extracting {}", filename);
            let zip_path = sanitize_zip_filename(
                filename.strip_prefix(&format!("{}/", overrides_folder)).unwrap()
            );
            let zip_path = output_dir.join(zip_path);
            sanitize_path_check(&zip_path, output_dir);
            
            if entry.dir().unwrap() {
                if !zip_path.exists() {
                    let _ = create_dir_all(&zip_path).await;
                }
            } else {
                let parent = zip_path.parent().unwrap();
                if !parent.is_dir() {
                    let _ = create_dir_all(parent).await;
                }
                let mut out_file = match File::create(&zip_path).await {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Failed to create file {}: {}", zip_path.display(), e);
                        continue;
                    }
                };
                let entry_reader = match zip.reader_with_entry(i).await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Failed to read entry {}: {}", filename, e);
                        continue;
                    }
                };
                use tokio_util::compat::FuturesAsyncReadCompatExt;
                let mut compat_reader = entry_reader.compat();
                if let Err(e) = tokio::io::copy(&mut compat_reader, &mut out_file).await {
                    eprintln!("Failed to extract {}: {}", filename, e);
                }
            }
        }
    }
}

/// Download mod loader (Forge or Fabric)
pub async fn download_mod_loader(
    manifest: &CurseForgeManifest,
    output_dir: &Path,
) -> Result<Option<String>, CurseForgeError> {
    if manifest.minecraft.mod_loaders.is_empty() {
        return Ok(None);
    }
    
    let mod_loader = &manifest.minecraft.mod_loaders[0];
    let client = Client::new();
    
    if mod_loader.id.starts_with("forge-") {
        let forge_version = mod_loader.id.strip_prefix("forge-").unwrap();
        let game_version = &manifest.minecraft.version;
        
        // Determine URL based on game version
        let url = if let Some(minor) = game_version.split('.').nth(1) {
            if minor.parse::<u32>().unwrap_or(0) < 8 {
                FORGE_URL_OLD
                    .replace("{game_version}", game_version)
                    .replace("{forge_version}", forge_version)
            } else {
                FORGE_URL
                    .replace("{game_version}", game_version)
                    .replace("{forge_version}", forge_version)
            }
        } else {
            FORGE_URL
                .replace("{game_version}", game_version)
                .replace("{forge_version}", forge_version)
        };
        
        let file_name = url.split('/').last().unwrap_or("forge-installer.jar");
        let dest_path = output_dir.join(file_name);
        
        println!("Downloading Forge from {}", url);
        download_simple(&client, &url, &dest_path).await?;
        
        Ok(Some(format!(
            "Forge {} downloaded. Run: java -jar \"{}\" to install",
            forge_version,
            dest_path.display()
        )))
    } else if mod_loader.id.starts_with("fabric") {
        let dest_path = output_dir.join(FABRIC_FILE_NAME);
        
        println!("Downloading Fabric from {}", FABRIC_URL);
        download_simple(&client, FABRIC_URL, &dest_path).await?;
        
        Ok(Some(format!(
            "Fabric installer downloaded. Run: java -jar \"{}\" to install",
            dest_path.display()
        )))
    } else {
        Ok(Some(format!(
            "Please download {} mod loader manually",
            mod_loader.id
        )))
    }
}

async fn download_simple(client: &Client, url: &str, path: &Path) -> Result<(), CurseForgeError> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(CurseForgeError::HttpError {
            message: format!("HTTP {} when downloading {}", response.status(), url),
        });
    }
    
    let bytes = response.bytes().await?;
    tokio::fs::write(path, bytes).await?;
    
    Ok(())
}

/// Check if a zip file is a CurseForge modpack
pub async fn is_curseforge_modpack(zip: &mut ZipFileReader) -> bool {
    for file in zip.file().entries().iter() {
        if file.filename().as_bytes() == "manifest.json".as_bytes() {
            return true;
        }
    }
    false
}

/// Check if a zip file is a Modrinth modpack
pub async fn is_modrinth_modpack(zip: &mut ZipFileReader) -> bool {
    for file in zip.file().entries().iter() {
        if file.filename().as_bytes() == "modrinth.index.json".as_bytes() {
            return true;
        }
    }
    false
}
