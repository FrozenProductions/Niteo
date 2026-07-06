use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cache::edges::CachedImportEdge;

const CACHE_FILE_NAME: &str = ".niteo/cache.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheFile {
    pub version: u32,
    pub niteo_version: String,
    pub config_hash: String,
    pub tsconfig_hash: Option<String>,
    pub file_list_hash: String,
    pub files: HashMap<String, CachedFileAnalysis>,
    #[serde(default)]
    pub graph: Option<CachedGraph>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedGraph {
    pub edge_hash: String,
    pub cycles: Vec<CachedCycle>,
    pub imported_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedCycle {
    pub canonical: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedFileAnalysis {
    pub content_hash: String,
    pub import_edges: Vec<CachedImportEdge>,
    pub violations: Vec<CachedViolation>,
    pub parse_failure: Option<CachedParseFailure>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedViolation {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub rule: String,
    pub message: String,
    pub severity: String,
    pub detail: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedParseFailure {
    pub message: String,
}

pub fn cache_path(project_root: &Path) -> PathBuf {
    project_root.join(CACHE_FILE_NAME)
}

pub fn read_cache(project_root: &Path) -> Result<Option<CacheFile>> {
    let path = cache_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read cache from {}", path.display()))?;
    let cache: CacheFile = serde_json::from_str(&source)
        .with_context(|| format!("failed to parse cache from {}", path.display()))?;
    Ok(Some(cache))
}

pub fn write_cache(project_root: &Path, cache: &CacheFile) -> Result<()> {
    let path = cache_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let source = serde_json::to_string_pretty(cache).context("failed to serialize cache")?;
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, source)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    std::fs::rename(&temp_path, &path).with_context(|| {
        format!(
            "failed to rename {} to {}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

pub fn clear_cache(project_root: &Path) -> Result<()> {
    let path = cache_path(project_root);
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}
