use std::{
    num::NonZeroUsize,
    path::PathBuf,
};

use async_zip::tokio::read::fs::ZipFileReader;
use clap::Parser;
use dialoguer::Confirm;

use core::{extract_folder, download_files, get_index_data, ALLOWED_HOSTS};
use schemas::EnvRequirement;

mod hash_checks;
mod schemas;
mod core;
mod gui;
mod curseforge;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct CliParameters {
    /// Launch GUI mode (if no input file is provided, GUI will launch automatically)
    #[arg(short, long)]
    gui: bool,
    
    input_file: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    /// Download the modpack as server version.
    #[arg(short, long)]
    server: bool,
    /// If enabled, hash checking stage will be skipped.
    #[arg(short, long)]
    ignore_hashes: bool,
    /// Set the number of concurrent downloads.
    #[arg(short, long, default_value_t = unsafe {NonZeroUsize::new_unchecked(5)})]
    jobs: NonZeroUsize,
    /// Skip download host check.
    ///
    /// See https://docs.modrinth.com/modpacks/format#downloads
    #[arg(long)]
    skip_host_check: bool,
    /// Skip all confirmation prompts (unattended mode).
    #[arg(short, long)]
    unattended: bool,
}

fn filter_file_list_cli(files: &mut Vec<schemas::ModpackFile>, is_server: bool, unattended: bool) {
    files.retain(|file| match &file.env {
        None => true,
        Some(reqs) => {
            let req = if is_server {
                &reqs.server
            } else {
                &reqs.client
            };
            match req {
                EnvRequirement::Required => true,
                EnvRequirement::Unsupported => false,
                EnvRequirement::Optional => {
                    if unattended {
                        true // In unattended mode, download all optional files
                    } else {
                        !matches!(
                            Confirm::new()
                                .with_prompt(format!(
                                    "Download optional {}?",
                                    file.path.to_string_lossy()
                                ))
                                .default(true)
                                .wait_for_newline(false)
                                .interact_opt()
                                .unwrap(),
                            Some(false) | None
                        )
                    }
                }
            }
        }
    })
}

fn main() {
    let parameters = CliParameters::parse();

    // Launch GUI if no input file provided or --gui flag is set
    if parameters.gui || parameters.input_file.is_none() {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 700.0])
                .with_min_inner_size([600.0, 500.0])
                .with_icon(
                    eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon.png")[..])
                        .unwrap_or_default(),
                ),
            ..Default::default()
        };
        
        if let Err(e) = eframe::run_native(
            "Modrinth Modpack Downloader",
            native_options,
            Box::new(|cc| Ok(Box::new(gui::MrpackDownloaderApp::new(cc)))),
        ) {
            eprintln!("Failed to launch GUI: {}", e);
        }
    } else {
        // Run CLI mode in tokio runtime
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_cli(parameters));
    }
}

async fn run_cli(parameters: CliParameters) {
    let input_file = parameters.input_file.clone().unwrap_or_else(|| {
        eprintln!("Error: Input .mrpack or .zip file is required when running in CLI mode.");
        eprintln!("Usage: mrpack-downloader <input.mrpack|input.zip> <output-dir>");
        eprintln!("Or use: mrpack-downloader --gui to launch the graphical interface");
        std::process::exit(1);
    });
    let output_dir = parameters.output_dir.clone().unwrap_or_else(|| {
        eprintln!("Error: Output directory is required when running in CLI mode.");
        eprintln!("Usage: mrpack-downloader <input.mrpack|input.zip> <output-dir>");
        eprintln!("Or use: mrpack-downloader --gui to launch the graphical interface");
        std::process::exit(1);
    });

    let mut zip_file = ZipFileReader::new(&input_file).await.unwrap();

    // Detect modpack format
    let is_curseforge = curseforge::is_curseforge_modpack(&mut zip_file).await;
    let is_modrinth = curseforge::is_modrinth_modpack(&mut zip_file).await;

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        tokio::fs::create_dir_all(&output_dir).await.unwrap();
    }

    let target_path = output_dir.canonicalize().unwrap();

    if is_curseforge {
        // CurseForge modpack
        println!("Detected CurseForge modpack format");
        run_curseforge_cli(&mut zip_file, &target_path, &parameters).await;
    } else if is_modrinth {
        // Modrinth modpack
        println!("Detected Modrinth modpack format");
        run_modrinth_cli(&mut zip_file, &target_path, &parameters).await;
    } else {
        eprintln!("Error: Could not detect modpack format.");
        eprintln!("The file should contain either 'modrinth.index.json' (Modrinth) or 'manifest.json' (CurseForge).");
        std::process::exit(1);
    }
}

async fn run_curseforge_cli(zip_file: &mut ZipFileReader, target_path: &std::path::Path, parameters: &CliParameters) {
    let manifest = curseforge::read_curseforge_manifest(zip_file).await.unwrap();
    
    manifest.print_info();

    println!(
        "\nTotal files to download: {}",
        manifest.files.len()
    );

    if !parameters.unattended {
        match Confirm::new()
            .with_prompt("Proceed to downloading?")
            .default(true)
            .wait_for_newline(true)
            .interact_opt()
            .unwrap()
        {
            Some(false) | None => return,
            _ => (),
        }
    }

    println!("\nDownloading files...");
    if let Err(why) = curseforge::download_curseforge_files(
        &manifest,
        target_path,
        parameters.jobs.get(),
        None,
    )
    .await
    {
        panic!("Download failed: {why}");
    }

    println!("\nExtracting overrides...");
    let overrides = manifest.overrides.as_deref().unwrap_or("overrides");
    curseforge::extract_curseforge_overrides(zip_file, overrides, target_path).await;

    println!("\nDownloading mod loader...");
    match curseforge::download_mod_loader(&manifest, target_path).await {
        Ok(Some(msg)) => println!("{}", msg),
        Ok(None) => println!("No mod loader specified"),
        Err(e) => eprintln!("Warning: Failed to download mod loader: {}", e),
    }

    println!("\n✅ {} v{} downloaded successfully!", manifest.name, manifest.version);
}

async fn run_modrinth_cli(zip_file: &mut ZipFileReader, target_path: &std::path::Path, parameters: &CliParameters) {
    let mut modrinth_index_data = get_index_data(zip_file).await.unwrap();
    if !parameters.skip_host_check {
        for file in modrinth_index_data.files.iter() {
            for url in file.downloads.iter() {
                if !ALLOWED_HOSTS.contains(
                    &url.domain()
                        .expect("IP addresses are not allowed in download URLs"),
                ) {
                    panic!(
                        "Downloading from {} is not allowed. See https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack#h_e2af55e39e",
                        url.domain().unwrap()
                    );
                }
            }
        }
    }

    modrinth_index_data.print_info();

    if parameters.server {
        println!("Downloading as a server version is enabled");
    }

    filter_file_list_cli(
        &mut modrinth_index_data.files,
        parameters.server,
        parameters.unattended,
    );

    println!(
        "Total amount of files to download after filtering: {}",
        modrinth_index_data.files.len()
    );

    println!(
        "Total download size: {}",
        core::prettify_bytes(
            modrinth_index_data
                .files
                .iter()
                .map(|file| file.file_size as u64)
                .sum()
        )
    );

    if !parameters.unattended {
        match Confirm::new()
            .with_prompt("Proceed to downloading?")
            .default(true)
            .wait_for_newline(true)
            .interact_opt()
            .unwrap()
        {
            Some(false) | None => return,
            _ => (),
        }
    }

    println!("Downloading files");
    if let Err(why) = download_files(
        modrinth_index_data,
        target_path,
        parameters.ignore_hashes,
        parameters.jobs.get(),
    )
    .await
    {
        panic!("Download failed: {why}");
    }

    println!("Extracting additional files (overrides)");
    extract_folder(zip_file, "overrides", target_path).await;
    if parameters.server {
        extract_folder(zip_file, "overrides-server", target_path).await;
    } else {
        extract_folder(zip_file, "overrides-client", target_path).await;
    }
}
