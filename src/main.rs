use std::{
    iter::Iterator,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use async_zip::tokio::read::fs::ZipFileReader;
use clap::Parser;
use dialoguer::Confirm;
use futures_util::{stream::StreamExt, TryStreamExt};
use hash_checks::check_hashes;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::{Client, StatusCode};
use schemas::{EnvRequirement, ModpackFile, ModrinthIndex};
use thiserror::Error;
use tokio::fs::{create_dir_all, File};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::StreamReader};
use url::Url;

mod hash_checks;
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
}

#[derive(Debug, Error)]
enum IndexReadError {
    #[error(transparent)]
    AsyncZip(#[from] async_zip::error::ZipError),
    #[error("modrinth.index.json was not found within the modpack file")]
    NotFound,
}

async fn read_index_data(buf: &mut Vec<u8>, zip: &mut ZipFileReader) -> Result<(), IndexReadError> {
    let mut found = false;
    for (i, file) in zip.file().entries().iter().enumerate() {
        if file.filename().as_bytes() == "modrinth.index.json".as_bytes() {
            found = true;
            let mut entry = zip.reader_with_entry(i).await?;
            entry.read_to_end_checked(buf).await?;
            break;
        }
    }
    if !found {
        Err(IndexReadError::NotFound)
    } else {
        Ok(())
    }
}

fn sanitize_path_check(path: &Path, output_dir: &Path) {
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
        .filter(|seg| !matches!(*seg, ".." | ""))
        .collect()
}

async fn extract_folder(zip: &mut ZipFileReader, folder_name: &str, output_dir: &Path) {
    for (i, entry) in zip.file().entries().iter().enumerate() {
        let filename = entry.filename().as_str().unwrap();
        if filename.starts_with(&format!("{folder_name}/")) {
            println!("Extracting {filename}");
            let zip_path =
                sanitize_zip_filename(filename.strip_prefix(&format!("{folder_name}/")).unwrap());
            let zip_path = output_dir.join(zip_path);
            sanitize_path_check(&zip_path, output_dir);
            if entry.dir().unwrap() {
                if !zip_path.exists() {
                    create_dir_all(&zip_path).await.unwrap()
                }
            } else {
                let parent = zip_path.parent().unwrap();
                if !parent.is_dir() {
                    create_dir_all(parent).await.unwrap()
                }
                let mut out_file = File::create(zip_path).await.unwrap();
                let mut entry_reader = zip.reader_with_entry(i).await.unwrap().compat();
                tokio::io::copy(&mut entry_reader, &mut out_file)
                    .await
                    .unwrap();
            }
        }
    }
}

async fn download_files(
    index: ModrinthIndex,
    output_dir: &Path,
    ignore_hashes: bool,
    jobs: usize,
) -> Result<(), FileDownloadError> {
    let mpb = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());
    let client = Client::new();
    let files_stream = futures::stream::iter(index.files);
    files_stream
        .map::<Result<_, FileDownloadError>, _>(Ok)
        .try_for_each_concurrent(jobs, |file| {
            let client_clone = client.clone();
            let mpb_clone = mpb.clone();
            let path = output_dir.join(&file.path);
            sanitize_path_check(&path, output_dir);
            async move {
                download_file(client_clone, &file.downloads, &path, mpb_clone).await?;
                if !ignore_hashes {
                    check_hashes(file.hashes, path).await;
                };
                Ok(())
            }
        })
        .await
}

#[derive(Debug, Error)]
enum FileTryDownloadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Request to {url} failed. Status code: {status}; message: {message}")]
    RequestFailed {
        url: Url,
        status: StatusCode,
        message: String,
    },
}

async fn try_download_file(
    client: &Client,
    url: &Url,
    path: &Path,
    bar: &ProgressBar,
) -> Result<(), FileTryDownloadError> {
    let res = client.get(url.clone()).send().await?;
    let status = res.status();
    if status.is_success() {
        if let Some(total_size) = res.content_length() {
            bar.set_length(total_size);
        }

        let mut out_file = File::create(path).await?;
        let stream = res.bytes_stream();

        let stream_reader = StreamReader::new(
            stream.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );

        let mut bar_reader = bar.wrap_async_read(stream_reader);

        tokio::io::copy(&mut bar_reader, &mut out_file).await?;

        Ok(())
    } else {
        Err(FileTryDownloadError::RequestFailed {
            url: url.clone(),
            status,
            message: res.text().await?,
        })
    }
}

#[derive(Debug, Error)]
enum FileDownloadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("All downloads have failed")]
    AllDownloadsFailed,
}

async fn download_file(
    client: Client,
    urls: &[Url],
    path: &Path,
    progress_bars: MultiProgress,
) -> Result<(), FileDownloadError> {
    let pb = progress_bars.add(
        ProgressBar::with_draw_target(None, ProgressDrawTarget::stdout())
            .with_message(format!("Downloading {}", path.to_string_lossy()))
            .with_style(
                ProgressStyle::default_bar()
                .template("{msg}\n{spinner} [{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").expect("Incorrect template provided")
                .progress_chars("#> ")
            ),
    );

    // The directories will be created in case the parent directory doesn't exist or the parent is
    // actually a file, which is an error condition and will be reported in the error.
    if !path.parent().unwrap().is_dir() {
        create_dir_all(path.parent().unwrap()).await?;
    }

    let mut urls_iter = urls.iter();

    // This loop tries all urls until one of them succedes or it runs out of urls. The iterator is
    // finite (fused) which guarantees that the loop will finish.
    loop {
        match urls_iter.next() {
            // Try next url in the list
            Some(url) => match try_download_file(&client, url, path, &pb).await {
                // Downloads succeded, stop looping and return.
                Ok(()) => {
                    pb.finish_with_message(format!(
                        "Downloaded {} from {}",
                        path.to_string_lossy(),
                        url
                    ));
                    break Ok(());
                }
                // An error occured. Report and go to the next url.
                Err(why) => {
                    eprintln!(
                        "Failed to download file {} from {url}: {why}",
                        path.to_string_lossy(),
                    );
                }
            },
            // No more urls to try.
            None => {
                pb.finish_with_message(format!("Failed to download {}", path.to_string_lossy()));
                break Err(FileDownloadError::AllDownloadsFailed);
            }
        }
    }
}

fn filter_file_list(files: &mut Vec<ModpackFile>, is_server: bool) {
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
                EnvRequirement::Optional => !matches!(
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
                ),
            }
        }
    })
}

#[derive(Debug, Error)]
enum IndexGetError {
    #[error(transparent)]
    ReadError(#[from] IndexReadError),
    #[error("Failed to deserialize index file: {0}")]
    SerdeError(#[from] serde_json::Error),
}

async fn get_index_data(zip_file: &mut ZipFileReader) -> Result<ModrinthIndex, IndexGetError> {
    let mut index_data: Vec<u8> = Vec::new();
    read_index_data(&mut index_data, zip_file).await?;

    serde_json::from_slice(&index_data).map_err(Into::into)
}

#[tokio::main]
async fn main() {
    let parameters = CliParameters::parse();

    let mut zip_file = ZipFileReader::new(parameters.input_file).await.unwrap();

    let mut modrinth_index_data = get_index_data(&mut zip_file).await.unwrap();
    if !parameters.skip_host_check {
        for file in modrinth_index_data.files.iter() {
            for url in file.downloads.iter() {
                if !ALLOWED_HOSTS.contains(
                    &url.domain()
                        .expect("IP addresses are not allowed in download URLs"),
                ) {
                    panic!("Downloading from {} is not allowed. See https://docs.modrinth.com/modpacks/format#downloads", url.domain().unwrap());
                }
            }
        }
    }

    let target_path = parameters.output_dir.canonicalize().unwrap();

    modrinth_index_data.print_info();

    if parameters.server {
        println!("Downloading as a server version is enabled");
    }

    filter_file_list(&mut modrinth_index_data.files, parameters.server);

    println!(
        "Total amount of files to download after filtering: {}",
        modrinth_index_data.files.len()
    );

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

    println!("Downloading files");
    if let Err(why) = download_files(
        modrinth_index_data,
        &target_path,
        parameters.ignore_hashes,
        parameters.jobs.get(),
    )
    .await
    {
        panic!("Download failed: {why}");
    }

    println!("Extracting additional files (overrides)");
    extract_folder(&mut zip_file, "overrides", &target_path).await;
    if parameters.server {
        extract_folder(&mut zip_file, "overrides-server", &target_path).await;
    } else {
        extract_folder(&mut zip_file, "overrides-client", &target_path).await;
    }
}
