use std::{collections::HashMap, path::PathBuf};

use semver::Version;
use serde::Deserialize;
use strum_macros::AsRefStr;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    pub dependencies: HashMap<ModpackDependencyId, Version>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFile {
    pub path: PathBuf,
    pub hashes: FileHashes,
    pub env: Option<FileEnv>,
    pub downloads: Vec<Url>,
    pub file_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileHashes {
    #[serde(deserialize_with = "hex::deserialize")]
    sha1: [u8; 20],
    #[serde(deserialize_with = "hex::deserialize")]
    sha512: [u8; 64],
    #[serde(flatten)]
    other_hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEnv {
    pub client: EnvRequirement,
    pub server: EnvRequirement,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvRequirement {
    Required,
    Optional,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize, AsRefStr)]
#[serde(rename_all = "kebab-case")]
pub enum ModpackDependencyId {
    Minecraft,
    Forge,
    FabricLoader,
    QuiltLoader,
}
