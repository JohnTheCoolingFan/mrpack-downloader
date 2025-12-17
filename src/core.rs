use std::{
    path::{Path, PathBuf},
};

use async_zip::tokio::read::fs::ZipFileReader;
use futures_util::{TryStreamExt, stream::StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::{Client, StatusCode};
use thiserror::Error;
use tokio::fs::{File, create_dir_all};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::StreamReader};
use url::Url;

use crate::hash_checks::check_hashes;
use crate::schemas::{EnvRequirement, ModpackFile, ModrinthIndex};

pub const ALLOWED_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

pub fn prettify_bytes(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    } else if bytes > 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / 1024.0 / 1024.0)
    } else if bytes > 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} B", bytes)
    }
}

#[derive(Debug, Error)]
pub enum IndexReadError {
    #[error(transparent)]
    AsyncZip(#[from] async_zip::error::ZipError),
    #[error("modrinth.index.json was not found within the modpack file")]
    NotFound,
}

pub async fn read_index_data(buf: &mut Vec<u8>, zip: &mut ZipFileReader) -> Result<(), IndexReadError> {
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

pub fn sanitize_path_check(path: &Path, output_dir: &Path) {
    let sanitized_path = canonicalize_recursively(path).unwrap();
    if !sanitized_path.starts_with(output_dir) {
        panic!(
            "Path {} is outside of output dir ({})",
            path.to_string_lossy(),
            output_dir.to_string_lossy()
        );
    }
}

pub fn canonicalize_recursively(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            return ancestor.canonicalize().ok();
        }
    }
    None
}

pub fn sanitize_zip_filename(filename: &str) -> PathBuf {
    filename
        .replace('\\', "/")
        .split('/')
        .filter(|seg| !matches!(*seg, ".." | ""))
        .collect()
}

pub async fn extract_folder(zip: &mut ZipFileReader, folder_name: &str, output_dir: &Path) {
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

pub async fn download_files(
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
pub enum FileTryDownloadError {
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

pub async fn try_download_file(
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

        let stream_reader = StreamReader::new(stream.map_err(std::io::Error::other));

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
pub enum FileDownloadError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("All downloads have failed")]
    AllDownloadsFailed,
}

pub async fn download_file(
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

pub fn filter_file_list(files: &mut Vec<ModpackFile>, is_server: bool, auto_include_optional: bool) {
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
                EnvRequirement::Optional => auto_include_optional,
            }
        }
    })
}

#[derive(Debug, Error)]
pub enum IndexGetError {
    #[error(transparent)]
    ReadError(#[from] IndexReadError),
    #[error("Failed to deserialize index file: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub async fn get_index_data(zip_file: &mut ZipFileReader) -> Result<ModrinthIndex, IndexGetError> {
    let mut index_data: Vec<u8> = Vec::new();
    read_index_data(&mut index_data, zip_file).await?;

    serde_json::from_slice(&index_data).map_err(Into::into)
}
