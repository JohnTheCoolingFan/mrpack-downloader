use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use eframe::egui;
use egui::{Color32, RichText, Vec2};

// Color theme constants
const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 35);
const TEXT_COLOR: Color32 = Color32::from_rgb(220, 220, 220);
const PRIMARY_GREEN: Color32 = Color32::from_rgb(30, 180, 100);
const INFO_BLUE: Color32 = Color32::from_rgb(100, 200, 255);
const SUCCESS_GREEN: Color32 = Color32::from_rgb(0, 255, 0);
const ERROR_RED: Color32 = Color32::from_rgb(255, 100, 100);

#[derive(Clone, Debug)]
pub enum DownloadState {
    Idle,
    LoadingIndex,
    ReadyToDownload(ModpackInfo),
    Downloading(DownloadProgress),
    Completed,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub dependencies: Vec<(String, String)>,
    pub total_files: usize,
    pub total_size: u64,
    pub format: crate::schemas::ModpackFormat,
}

#[derive(Clone, Debug)]
pub struct DownloadProgress {
    pub current_file: usize,
    pub total_files: usize,
    pub current_file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

pub struct MrpackDownloaderApp {
    pub input_file: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub is_server: bool,
    pub ignore_hashes: bool,
    pub skip_host_check: bool,
    pub include_optional: bool,
    pub concurrent_downloads: usize,
    pub state: Arc<Mutex<DownloadState>>,
    pub show_settings: bool,
}

impl Default for MrpackDownloaderApp {
    fn default() -> Self {
        Self {
            input_file: None,
            output_dir: None,
            is_server: false,
            ignore_hashes: false,
            skip_host_check: false,
            include_optional: true,
            concurrent_downloads: 5,
            state: Arc::new(Mutex::new(DownloadState::Idle)),
            show_settings: false,
        }
    }
}

impl MrpackDownloaderApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.heading(
                RichText::new("🎮 Modpack Downloader")
                    .size(32.0)
                    .color(PRIMARY_GREEN)
            );
            ui.label(
                RichText::new("Download and install Modrinth (.mrpack) and CurseForge (.zip) modpacks")
                    .size(14.0)
                    .color(Color32::GRAY)
            );
            ui.add_space(10.0);
        });
        ui.separator();
    }

    fn render_file_selection(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_height(120.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("📁 File Selection").size(18.0).strong());
                ui.add_space(5.0);

                // Input file selection
                ui.horizontal(|ui| {
                    ui.label("Modpack file (.mrpack or .zip):");
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Modpack Files", &["mrpack", "zip"])
                            .add_filter("Modrinth Modpack", &["mrpack"])
                            .add_filter("CurseForge Modpack", &["zip"])
                            .pick_file()
                        {
                            self.input_file = Some(path);
                        }
                    }
                });
                
                if let Some(path) = &self.input_file {
                    ui.horizontal(|ui| {
                        ui.label("📦");
                        ui.label(
                            RichText::new(path.to_string_lossy())
                                .color(INFO_BLUE)
                        );
                    });
                }

                ui.add_space(5.0);

                // Output directory selection
                ui.horizontal(|ui| {
                    ui.label("Output directory:          ");
                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = Some(path);
                        }
                    }
                });

                if let Some(path) = &self.output_dir {
                    ui.horizontal(|ui| {
                        ui.label("📂");
                        ui.label(
                            RichText::new(path.to_string_lossy())
                                .color(INFO_BLUE)
                        );
                    });
                }
            });
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚙️ Settings").size(18.0).strong());
                    if ui.button(if self.show_settings { "▼" } else { "▶" }).clicked() {
                        self.show_settings = !self.show_settings;
                    }
                });

                if self.show_settings {
                    ui.add_space(5.0);
                    
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.is_server, "Server mode");
                        ui.label("💻");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.include_optional, "Include optional mods");
                        ui.label("📦");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.ignore_hashes, "Skip hash verification");
                        ui.label("⚠️");
                    });
                    
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.skip_host_check, "Skip host checking");
                        ui.label("🔓");
                    });

                    ui.add_space(5.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Concurrent downloads:");
                        ui.add(egui::Slider::new(&mut self.concurrent_downloads, 1..=20));
                    });
                }
            });
        });
    }

    fn render_modpack_info(&self, ui: &mut egui::Ui, info: &ModpackInfo) {
        ui.group(|ui| {
            ui.set_min_height(150.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("📋 Modpack Information").size(18.0).strong());
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Format:").strong());
                    let format_text = match info.format {
                        crate::schemas::ModpackFormat::Modrinth => "🟢 Modrinth (.mrpack)",
                        crate::schemas::ModpackFormat::CurseForge => "🟧 CurseForge (.zip)",
                    };
                    ui.label(RichText::new(format_text).color(INFO_BLUE));
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name:").strong());
                    ui.label(RichText::new(&info.name).color(INFO_BLUE));
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Version:").strong());
                    ui.label(&info.version);
                });

                if let Some(summary) = &info.summary {
                    ui.add_space(3.0);
                    ui.label(RichText::new("Summary:").strong());
                    ui.label(summary);
                }

                ui.add_space(5.0);
                ui.label(RichText::new("Dependencies:").strong());
                for (name, version) in &info.dependencies {
                    ui.horizontal(|ui| {
                        ui.label("  •");
                        ui.label(format!("{}: {}", name, version));
                    });
                }

                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Files to download:").strong());
                    ui.label(format!("{}", info.total_files));
                });

                if info.total_size > 0 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Total size:").strong());
                        ui.label(crate::core::prettify_bytes(info.total_size));
                    });
                }
            });
        });
    }

    fn render_download_progress(&self, ui: &mut egui::Ui, progress: &DownloadProgress) {
        ui.group(|ui| {
            ui.set_min_height(100.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("⏬ Download Progress").size(18.0).strong());
                ui.add_space(10.0);

                let file_progress = if progress.total_files > 0 {
                    progress.current_file as f32 / progress.total_files as f32
                } else {
                    0.0
                };

                ui.label(format!(
                    "File {} of {}",
                    progress.current_file, progress.total_files
                ));
                
                let progress_bar = egui::ProgressBar::new(file_progress)
                    .show_percentage()
                    .text(format!("{:.1}%", file_progress * 100.0));
                ui.add(progress_bar);

                ui.add_space(5.0);
                ui.label(format!("Current: {}", progress.current_file_name));

                if progress.total_bytes > 0 {
                    let byte_progress = progress.downloaded_bytes as f32 / progress.total_bytes as f32;
                    ui.label(format!(
                        "Downloaded: {} / {}",
                        crate::core::prettify_bytes(progress.downloaded_bytes),
                        crate::core::prettify_bytes(progress.total_bytes)
                    ));
                    let byte_bar = egui::ProgressBar::new(byte_progress);
                    ui.add(byte_bar);
                }
            });
        });
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        
        let state = self.state.lock().unwrap().clone();
        
        ui.horizontal(|ui| {
            let can_load = self.input_file.is_some() && self.output_dir.is_some();
            
            match state {
                DownloadState::Idle => {
                    if ui
                        .add_enabled(can_load, egui::Button::new(
                            RichText::new("📂 Load Modpack").size(16.0)
                        ).min_size(Vec2::new(150.0, 40.0)))
                        .clicked()
                    {
                        self.load_modpack();
                    }
                }
                DownloadState::LoadingIndex => {
                    ui.add_enabled(
                        false,
                        egui::Button::new(RichText::new("⏳ Loading...").size(16.0))
                            .min_size(Vec2::new(150.0, 40.0))
                    );
                }
                DownloadState::ReadyToDownload(_) => {
                    if ui
                        .add(egui::Button::new(
                            RichText::new("⬇️ Start Download").size(16.0)
                        ).min_size(Vec2::new(150.0, 40.0)))
                        .clicked()
                    {
                        self.start_download();
                    }
                }
                DownloadState::Downloading(_) => {
                    ui.add_enabled(
                        false,
                        egui::Button::new(RichText::new("⏬ Downloading...").size(16.0))
                            .min_size(Vec2::new(150.0, 40.0))
                    );
                }
                DownloadState::Completed => {
                    ui.label(
                        RichText::new("✅ Download Complete!")
                            .size(16.0)
                            .color(SUCCESS_GREEN)
                    );
                    if ui.button(RichText::new("🔄 Reset").size(16.0)).clicked() {
                        *self.state.lock().unwrap() = DownloadState::Idle;
                    }
                }
                DownloadState::Error(_) => {
                    if ui.button(RichText::new("🔄 Reset").size(16.0)).clicked() {
                        *self.state.lock().unwrap() = DownloadState::Idle;
                    }
                }
            }
        });
    }

    fn load_modpack(&mut self) {
        let input_file = self.input_file.clone().unwrap();
        let state = self.state.clone();

        *state.lock().unwrap() = DownloadState::LoadingIndex;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match load_modpack_info(&input_file).await {
                    Ok(info) => {
                        *state.lock().unwrap() = DownloadState::ReadyToDownload(info);
                    }
                    Err(e) => {
                        *state.lock().unwrap() = DownloadState::Error(format!("Failed to load modpack: {}", e));
                    }
                }
            });
        });
    }

    fn start_download(&mut self) {
        let input_file = self.input_file.clone().unwrap();
        let output_dir = self.output_dir.clone().unwrap();
        let is_server = self.is_server;
        let ignore_hashes = self.ignore_hashes;
        let skip_host_check = self.skip_host_check;
        let include_optional = self.include_optional;
        let jobs = self.concurrent_downloads;
        let state = self.state.clone();

        *state.lock().unwrap() = DownloadState::Downloading(DownloadProgress {
            current_file: 0,
            total_files: 0,
            current_file_name: String::new(),
            downloaded_bytes: 0,
            total_bytes: 0,
        });

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                match perform_download(
                    &input_file,
                    &output_dir,
                    is_server,
                    ignore_hashes,
                    skip_host_check,
                    include_optional,
                    jobs,
                    state.clone(),
                )
                .await
                {
                    Ok(_) => {
                        *state.lock().unwrap() = DownloadState::Completed;
                    }
                    Err(e) => {
                        *state.lock().unwrap() = DownloadState::Error(format!("Download failed: {}", e));
                    }
                }
            });
        });
    }
}

impl eframe::App for MrpackDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint to keep UI responsive
        ctx.request_repaint();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(BG_COLOR)
                    .inner_margin(20.0)
            )
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(TEXT_COLOR);
                
                self.render_header(ui);
                ui.add_space(10.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_file_selection(ui);
                    ui.add_space(10.0);

                    self.render_settings(ui);
                    ui.add_space(10.0);

                    let state = self.state.lock().unwrap().clone();
                    match &state {
                        DownloadState::ReadyToDownload(info) => {
                            self.render_modpack_info(ui, info);
                        }
                        DownloadState::Downloading(progress) => {
                            self.render_download_progress(ui, progress);
                        }
                        DownloadState::Error(msg) => {
                            ui.group(|ui| {
                                ui.label(
                                    RichText::new("❌ Error")
                                        .size(18.0)
                                        .color(ERROR_RED)
                                );
                                ui.label(msg);
                            });
                        }
                        DownloadState::Completed => {
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new("✅ Download Complete!")
                                            .size(24.0)
                                            .color(SUCCESS_GREEN)
                                    );
                                    ui.label("Your modpack has been successfully downloaded and installed!");
                                });
                            });
                        }
                        _ => {}
                    }

                    self.render_action_buttons(ui);
                });
            });
    }
}

async fn load_modpack_info(input_file: &PathBuf) -> Result<ModpackInfo, String> {
    use async_zip::tokio::read::fs::ZipFileReader;
    use crate::core::get_index_data;
    use crate::curseforge::{read_curseforge_manifest, is_curseforge_modpack, is_modrinth_modpack};

    let mut zip_file = ZipFileReader::new(input_file)
        .await
        .map_err(|e| format!("Failed to open zip file: {}", e))?;

    // Detect modpack format
    let is_cf = is_curseforge_modpack(&mut zip_file).await;
    let is_mr = is_modrinth_modpack(&mut zip_file).await;

    if is_cf {
        // CurseForge modpack
        let manifest = read_curseforge_manifest(&mut zip_file)
            .await
            .map_err(|e| format!("Failed to read CurseForge manifest: {}", e))?;

        let dependencies: Vec<(String, String)> = manifest.minecraft.mod_loaders
            .iter()
            .map(|loader| {
                let name = if loader.id.starts_with("forge-") {
                    "Forge".to_string()
                } else if loader.id.starts_with("fabric") {
                    "Fabric".to_string()
                } else if loader.id.starts_with("neoforge") {
                    "NeoForge".to_string()
                } else {
                    loader.id.clone()
                };
                let version = loader.id.split('-').last().unwrap_or(&loader.id).to_string();
                (name, version)
            })
            .collect();

        let mut deps = vec![("Minecraft".to_string(), manifest.minecraft.version.clone())];
        deps.extend(dependencies);

        Ok(ModpackInfo {
            name: manifest.name,
            version: manifest.version,
            summary: manifest.author.map(|a| format!("by {}", a)),
            dependencies: deps,
            total_files: manifest.files.len(),
            total_size: 0, // CurseForge doesn't provide total size upfront
            format: crate::schemas::ModpackFormat::CurseForge,
        })
    } else if is_mr {
        // Modrinth modpack
        let index = get_index_data(&mut zip_file)
            .await
            .map_err(|e| format!("Failed to read index: {}", e))?;

        let total_size: u64 = index.files.iter().map(|f| f.file_size as u64).sum();
        
        let dependencies: Vec<(String, String)> = index
            .dependencies
            .iter()
            .map(|(k, v)| (format!("{}", k), v.to_string()))
            .collect();

        Ok(ModpackInfo {
            name: index.name.clone(),
            version: index.version_id.clone(),
            summary: index.summary.clone(),
            dependencies,
            total_files: index.files.len(),
            total_size,
            format: crate::schemas::ModpackFormat::Modrinth,
        })
    } else {
        Err("Could not detect modpack format. Expected modrinth.index.json or manifest.json".to_string())
    }
}

async fn perform_download(
    input_file: &PathBuf,
    output_dir: &PathBuf,
    is_server: bool,
    ignore_hashes: bool,
    skip_host_check: bool,
    include_optional: bool,
    jobs: usize,
    state: Arc<Mutex<DownloadState>>,
) -> Result<(), String> {
    use async_zip::tokio::read::fs::ZipFileReader;
    use crate::core::{get_index_data, download_files_with_callback, extract_folder, filter_file_list, ALLOWED_HOSTS};
    use crate::curseforge::{read_curseforge_manifest, download_curseforge_files, extract_curseforge_overrides, download_mod_loader, is_curseforge_modpack, is_modrinth_modpack};

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        tokio::fs::create_dir_all(output_dir)
            .await
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let mut zip_file = ZipFileReader::new(input_file)
        .await
        .map_err(|e| format!("Failed to open zip file: {}", e))?;

    // Detect modpack format
    let is_cf = is_curseforge_modpack(&mut zip_file).await;
    let is_mr = is_modrinth_modpack(&mut zip_file).await;

    let target_path = output_dir
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize output path: {}", e))?;

    if is_cf {
        // CurseForge modpack download
        let manifest = read_curseforge_manifest(&mut zip_file)
            .await
            .map_err(|e| format!("Failed to read CurseForge manifest: {}", e))?;

        let total_files = manifest.files.len();

        *state.lock().unwrap() = DownloadState::Downloading(DownloadProgress {
            current_file: 0,
            total_files,
            current_file_name: String::from("Starting CurseForge download..."),
            downloaded_bytes: 0,
            total_bytes: 0,
        });

        // Create progress callback
        let state_clone = state.clone();
        let progress_callback = Box::new(move |current: usize, total: usize, file_name: String, downloaded: u64, total_bytes: u64| {
            *state_clone.lock().unwrap() = DownloadState::Downloading(DownloadProgress {
                current_file: current,
                total_files: total,
                current_file_name: file_name,
                downloaded_bytes: downloaded,
                total_bytes,
            });
        });

        download_curseforge_files(&manifest, &target_path, jobs, Some(progress_callback))
            .await
            .map_err(|e| format!("CurseForge download failed: {}", e))?;

        // Extract overrides
        let overrides = manifest.overrides.as_deref().unwrap_or("overrides");
        extract_curseforge_overrides(&mut zip_file, overrides, &target_path).await;

        // Download mod loader
        if let Err(e) = download_mod_loader(&manifest, &target_path).await {
            eprintln!("Warning: Failed to download mod loader: {}", e);
        }

    } else if is_mr {
        // Modrinth modpack download
        let mut modrinth_index_data = get_index_data(&mut zip_file)
            .await
            .map_err(|e| format!("Failed to read index: {}", e))?;

        // Host check
        if !skip_host_check {
            for file in modrinth_index_data.files.iter() {
                for url in file.downloads.iter() {
                    if !ALLOWED_HOSTS.contains(
                        &url.domain()
                            .ok_or("IP addresses are not allowed in download URLs")?,
                    ) {
                        return Err(format!(
                            "Downloading from {} is not allowed.",
                            url.domain().unwrap()
                        ));
                    }
                }
            }
        }

        filter_file_list(
            &mut modrinth_index_data.files,
            is_server,
            include_optional,
        );

        let total_files = modrinth_index_data.files.len();
        let total_bytes: u64 = modrinth_index_data.files.iter().map(|f| f.file_size as u64).sum();

        *state.lock().unwrap() = DownloadState::Downloading(DownloadProgress {
            current_file: 0,
            total_files,
            current_file_name: String::from("Starting..."),
            downloaded_bytes: 0,
            total_bytes,
        });

        // Create progress callback
        let state_clone = state.clone();
        let progress_callback = Box::new(move |current: usize, total: usize, file_name: String, downloaded: u64, total_bytes: u64| {
            *state_clone.lock().unwrap() = DownloadState::Downloading(DownloadProgress {
                current_file: current,
                total_files: total,
                current_file_name: file_name,
                downloaded_bytes: downloaded,
                total_bytes,
            });
        });

        download_files_with_callback(modrinth_index_data.clone(), &target_path, ignore_hashes, jobs, Some(progress_callback))
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        extract_folder(&mut zip_file, "overrides", &target_path).await;
        if is_server {
            extract_folder(&mut zip_file, "overrides-server", &target_path).await;
        } else {
            extract_folder(&mut zip_file, "overrides-client", &target_path).await;
        }
    } else {
        return Err("Could not detect modpack format".to_string());
    }

    Ok(())
}
