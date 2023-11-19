use std::{
    cmp::min,
    error::Error,
    iter::Iterator,
    path::{Path, PathBuf},
};

use async_zip::read::fs::ZipFileReader;
use clap::Parser;
use futures_util::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use prompts::{confirm::ConfirmPrompt, Prompt};
use reqwest::Client;
use schemas::ModrinthIndex;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};
use url::Url;

mod schemas;

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
    /// Download the modpack as server version. Currently does nothing.
    #[arg(short, long)]
    server: bool,
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
        .filter(needs_sanitization)
        .collect()
}

fn needs_sanitization(segment: &&str) -> bool {
    !matches!(*segment, ".." | "")
}

async fn extract_folder(zip: &mut ZipFileReader, name: &str, output_dir: &Path) {
    for (i, entry) in zip.file().entries().iter().enumerate() {
        let entry = entry.entry();
        if entry.filename().starts_with(&format!("{name}/")) {
            println!("Extracting {}", entry.filename());
            let zip_path =
                sanitize_zip_filename(entry.filename().strip_prefix(&format!("{name}/")).unwrap());
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

async fn download_files(index: ModrinthIndex, output_dir: &Path) {
    let mpb = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());
    let client = Client::new();
    for file in index.files {
        let path = output_dir.join(&file.path);
        sanitize_path(&path, output_dir);
        if let Err(why) = download_file(client.clone(), file.downloads, path, mpb.clone()).await {
            eprintln!("Failed to download: {why}");
        }
    }
}

async fn try_download_file(
    client: &Client,
    url: &Url,
    path: &Path,
    bar: &ProgressBar,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let res = client.get(url.clone()).send().await?;
    if res.status().is_success() {
        let total_size = res.content_length().unwrap();
        bar.set_length(total_size);

        let mut out_file = File::create(path).await?;
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item?;
            out_file.write_all(&chunk).await?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            bar.set_position(new);
        }
        todo!()
    } else {
        Err(format!(
            "Unexpected status code from {url}: {}; message: {}",
            res.status(),
            res.text().await?
        )
        .into())
    }
}

async fn download_file(
    client: Client,
    urls: Vec<Url>,
    path: PathBuf,
    progress_bars: MultiProgress,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let pb = progress_bars.add(
        ProgressBar::with_draw_target(None, ProgressDrawTarget::stdout())
            .with_message(format!("Downloading {}", path.to_string_lossy()))
            .with_style(
                ProgressStyle::default_bar()
                .template("{msg}\n{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
                .progress_chars("#> ")
            ),
    );

    if !path.parent().unwrap().is_dir() {
        create_dir_all(path.parent().unwrap()).await?;
    }

    for url in urls {
        match try_download_file(&client, &url, &path, &pb).await {
            Ok(()) => {
                pb.finish_with_message(format!(
                    "Downloaded {} from {}",
                    path.to_string_lossy(),
                    url
                ));
                return Ok(());
            }
            Err(why) => {
                eprintln!(
                    "Failed to download file {} from {url}: {why}",
                    path.to_string_lossy(),
                );
            }
        }
    }

    pb.finish_with_message(format!("Failed to download {}", path.to_string_lossy()));

    Err("All downloads failed".into())
}

fn print_info(index_data: &ModrinthIndex) {
    println!("{} version {}", index_data.name, index_data.version_id);
    if let Some(summary) = &index_data.summary {
        println!("\n{summary}");
    }
    println!("\nDependencies:");
    for (dep_id, dep_ver) in &index_data.dependencies {
        println!("{}: {}", dep_id.as_ref(), dep_ver);
    }
}

#[tokio::main]
async fn main() {
    let mut parameters = CliParameters::parse();

    let mut zip_file = ZipFileReader::new(&mut parameters.input_file)
        .await
        .unwrap();

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

    print_info(&modrinth_index_data);

    match ConfirmPrompt::new("Proceed?")
        .set_initial(true)
        .run()
        .await
        .unwrap()
    {
        Some(false) | None => return,
        _ => (),
    }

    println!("Downloading files");
    download_files(modrinth_index_data, &target_path).await;

    println!("Extracting overrides");
    extract_folder(&mut zip_file, "overrides", &target_path).await;
    extract_folder(&mut zip_file, "overrides-client", &target_path).await;
    extract_folder(&mut zip_file, "overrides-server", &target_path).await;
}
