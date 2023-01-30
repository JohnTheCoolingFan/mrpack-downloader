use async_zip::read::fs::ZipFileReader;
use clap::Parser;
use futures_util::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use std::{
    cmp::min,
    collections::HashMap,
    error::Error,
    iter::Iterator,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use url::Url;

const ALLOWED_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
struct CliParameters {
    input_file: PathBuf,
    output_dir: PathBuf,
    #[arg(short, long)]
    server: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct ModrinthIndex {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    game: String,
    #[serde(rename = "versionId")]
    version_id: String,
    name: String,
    summary: Option<String>,
    files: Vec<ModpackFile>,
    dependencies: HashMap<ModpackDependencyId, Version>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModpackFile {
    path: PathBuf,
    hashes: HashMap<String, String>,
    env: Option<FileEnv>,
    downloads: Vec<Url>,
    #[serde(rename = "fileSize")]
    file_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct FileEnv {
    client: EnvRequirement,
    server: EnvRequirement,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
enum EnvRequirement {
    #[serde(rename = "required")]
    Required,
    #[serde(rename = "optional")]
    Optional,
    #[serde(rename = "unsupported")]
    Unsupported,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
enum ModpackDependencyId {
    #[serde(rename = "minecraft")]
    Minecraft,
    #[serde(rename = "forge")]
    Forge,
    #[serde(rename = "fabric-loader")]
    FabricLoader,
    #[serde(rename = "quilt-loader")]
    QuiltLoader,
}

async fn get_index_data(
    buf: &mut Vec<u8>,
    zip: &mut ZipFileReader,
) -> async_zip::error::Result<()> {
    let mut found = false;
    for (i, file) in zip.file().entries().iter().enumerate() {
        if file.entry().filename() == "modrinth.index.json" {
            found = true;
            let mut entry = zip.entry(i).await?;
            entry.read_to_end_checked(buf, file.entry()).await?;
            break;
        }
    }
    if !found {
        panic!("modrinth.index.json not found within modpack file");
    }
    Ok(())
}

fn sanitize_path(path: &Path, output_dir: &Path) {
    let sanitized_path = canonicalize_recursively(path).unwrap();
    if !sanitized_path.starts_with(output_dir) {
        panic!(
            "Path {} is outside of output dir ({})",
            path.to_string_lossy(),
            output_dir.to_string_lossy()
        );
    }
}

fn canonicalize_recursively(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            return ancestor.canonicalize().ok();
        }
    }
    None
}

fn sanitize_zip_filename(filename: &str) -> PathBuf {
    filename
        .replace('\\', "/")
        .split('/')
        .filter(sanitize_segment)
        .collect()
}

fn sanitize_segment(segment: &&str) -> bool {
    !matches!(*segment, ".." | "")
}

async fn extract_overrides(zip: &mut ZipFileReader, output_dir: &Path) {
    for (i, entry) in zip.file().entries().iter().enumerate() {
        let entry = entry.entry();
        if entry.filename().starts_with("overrides/") {
            println!("Extracting {}", entry.filename());
            let zip_path =
                sanitize_zip_filename(entry.filename().strip_prefix("overrides/").unwrap());
            let zip_path = output_dir.join(zip_path);
            sanitize_path(&zip_path, output_dir);
            let mut entry_reader = zip.entry(i).await.unwrap();
            if entry.dir() {
                if !zip_path.exists() {
                    create_dir_all(&zip_path).await.unwrap()
                }
            } else {
                let parent = zip_path.parent().unwrap();
                if !parent.is_dir() {
                    create_dir_all(parent).await.unwrap()
                }
                let mut out_file = File::create(zip_path).await.unwrap();
                tokio::io::copy(&mut entry_reader, &mut out_file)
                    .await
                    .unwrap();
            }
        }
    }
}

async fn download_files(index: &ModrinthIndex, output_dir: &Path) {
    let mpb = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());
    let client = Client::new();
    for file in &index.files {
        let path = output_dir.join(&file.path);
        sanitize_path(&path, output_dir);
        if let Err(why) = download_file(&client, &file.downloads[0], &path, &mpb).await {
            println!("Failed to download: {why}");
        }
    }
}

async fn download_file(
    client: &Client,
    url: &Url,
    path: &Path,
    progress_bars: &MultiProgress,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let res = client.get(url.clone()).send().await?;
    let total_size = res.content_length().unwrap();

    let pb = progress_bars.add(
        ProgressBar::with_draw_target(Some(total_size), ProgressDrawTarget::stdout())
            .with_message(format!("Downloading {url}"))
            .with_style(
                ProgressStyle::default_bar()
                .template("{msg}\n{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
                .progress_chars("#> ")
            ),
    );

    if !path.parent().unwrap().is_dir() {
        create_dir_all(path.parent().unwrap()).await?;
    }

    let mut out_file = File::create(path).await?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        out_file.write_all(&chunk).await?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!(
        "Downloaded {} from {}",
        path.to_string_lossy(),
        url
    ));
    Ok(())
}

#[tokio::main]
async fn main() {
    let mut parameters = CliParameters::parse();

    let mut zip_file = ZipFileReader::new(&mut parameters.input_file)
        .await
        .unwrap();

    println!("Reading index data");
    let mut index_data: Vec<u8> = Vec::new();
    get_index_data(&mut index_data, &mut zip_file)
        .await
        .unwrap();

    let modrinth_index_data: ModrinthIndex = serde_json::from_slice(&index_data).unwrap();
    for file in modrinth_index_data.files.iter() {
        for url in file.downloads.iter() {
            if !ALLOWED_HOSTS.contains(&url.domain().unwrap()) {
                panic!("Downloading from {} is not allowed.", url.domain().unwrap());
            }
        }
    }

    let target_path = parameters.output_dir.canonicalize().unwrap();

    println!("Downloading files");
    download_files(&modrinth_index_data, &target_path).await;

    println!("Extracting overrides");
    extract_overrides(&mut zip_file, &target_path).await
}
