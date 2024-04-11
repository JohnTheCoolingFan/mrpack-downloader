use std::path::PathBuf;

use sha1::{Digest, Sha1};
use sha2::Sha512;
use tokio::{fs::File, io::AsyncReadExt};

use crate::schemas::FileHashes;

pub(crate) async fn check_hashes(hashes: FileHashes, path: PathBuf) {
    let mut file = File::open(&path).await.unwrap();
    let mut file_data = Vec::with_capacity(
        file.metadata()
            .await
            .map(|md| md.len() as usize)
            .unwrap_or(0),
    );
    file.read_to_end(&mut file_data).await.unwrap();
    drop(file);
    let sha1_passed = check_sha1(&file_data, &hashes.sha1);
    let sha512_passed = check_sha512(&file_data, &hashes.sha512);
    if !(sha1_passed && sha512_passed) {
        eprintln!("Deleting corrupted file {}", path.to_string_lossy());
        tokio::fs::remove_file(path).await.unwrap()
    }
}

fn check_sha1(data: &[u8], expected_hash: &[u8; 20]) -> bool {
    let hash = Sha1::digest(data);
    hash.as_slice() == expected_hash
}

fn check_sha512(data: &[u8], expected_hash: &[u8; 64]) -> bool {
    let hash = Sha512::digest(data);
    hash.as_slice() == expected_hash
}
