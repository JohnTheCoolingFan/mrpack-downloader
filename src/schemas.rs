use std::{collections::HashMap, fmt::Display, path::PathBuf};

use convert_case::Casing;
use semver::Version;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndex {
    #[allow(unused)]
    pub format_version: u32,
    #[allow(unused)]
    pub game: String,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    pub dependencies: HashMap<ModpackDependencyId, Version>,
}

impl ModrinthIndex {
    pub(crate) fn print_info(&self) {
        println!("{} version {}", self.name, self.version_id);
        if let Some(summary) = &self.summary {
            println!("\n{summary}");
        }
        println!("\nDependencies:");
        for (dep_id, dep_ver) in &self.dependencies {
            println!("{}: {}", dep_id, dep_ver);
        }
    }
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
    pub sha1: [u8; 20],
    #[serde(deserialize_with = "hex::deserialize")]
    pub sha512: [u8; 64],
    #[serde(flatten)]
    #[allow(unused)]
    pub other_hashes: HashMap<String, String>,
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

impl Display for ModpackDependencyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minecraft => write!(f, "Minecraft"),
            Self::Forge => write!(f, "Forge"),
            Self::Neoforge => write!(f, "NeoForge"),
            Self::FabricLoader => write!(f, "Fabric"),
            Self::QuiltLoader => write!(f, "Quilt"),
            Self::Other(name) => write!(f, "{}", name.to_case(convert_case::Case::Pascal)),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModpackDependencyId {
    Minecraft,
    Forge,
    Neoforge,
    FabricLoader,
    QuiltLoader,
    #[serde(untagged)]
    Other(String),
}

// ==================== CurseForge Modpack Schemas ====================

/// CurseForge manifest.json structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManifest {
    pub minecraft: CurseForgeMinecraft,
    pub manifest_type: String,
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub files: Vec<CurseForgeFile>,
    pub overrides: Option<String>,
}

impl CurseForgeManifest {
    pub fn print_info(&self) {
        println!("{} version {}", self.name, self.version);
        if let Some(author) = &self.author {
            println!("Author: {}", author);
        }
        println!("\nMinecraft: {}", self.minecraft.version);
        println!("Mod Loaders:");
        for loader in &self.minecraft.mod_loaders {
            let primary_str = if loader.primary { " (primary)" } else { "" };
            println!("  {}{}", loader.id, primary_str);
        }
        println!("\nTotal files: {}", self.files.len());
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeMinecraft {
    pub version: String,
    pub mod_loaders: Vec<CurseForgeModLoader>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseForgeModLoader {
    pub id: String,
    pub primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeFile {
    #[serde(rename = "projectID")]
    pub project_id: u64,
    #[serde(rename = "fileID")]
    pub file_id: u64,
    pub required: bool,
}

/// Response from CurseForge API (cfwidget)
#[derive(Debug, Clone, Deserialize)]
pub struct CurseForgeProjectInfo {
    pub id: u64,
    pub title: String,
    #[serde(rename = "type")]
    pub project_type: String,
    pub files: Vec<CurseForgeProjectFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseForgeProjectFile {
    pub id: u64,
    pub name: String,
    pub filesize: u64,
}

/// Enum to represent modpack format type
#[derive(Debug, Clone, PartialEq)]
pub enum ModpackFormat {
    Modrinth,
    CurseForge,
}
