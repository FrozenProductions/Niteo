use std::path::{Path, PathBuf};

use crate::cache::store::CacheFile;

pub const CACHE_SCHEMA_VERSION: u32 = 3;

pub fn hash_content(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

pub fn hash_string(content: &str) -> String {
    hash_content(content.as_bytes())
}

pub fn hash_file_list(files: &[PathBuf]) -> String {
    let mut sorted: Vec<String> = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    sorted.sort();
    hash_string(&sorted.join("\n"))
}

pub fn hash_config_files(config_paths: &[PathBuf]) -> String {
    let mut sorted_paths = config_paths.to_vec();
    sorted_paths.sort();
    let mut hasher_input = String::new();
    for path in &sorted_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            hasher_input.push_str(&content);
            hasher_input.push('\n');
        }
    }
    hash_string(&hasher_input)
}

pub fn hash_tsconfig(tsconfig_path: &Path) -> String {
    match std::fs::read_to_string(tsconfig_path) {
        Ok(content) => hash_string(&content),
        Err(_) => hash_string("<unreadable-tsconfig>"),
    }
}

pub fn is_cache_valid(
    cache: &CacheFile,
    niteo_version: &str,
    config_hash: &str,
    tsconfig_hash: Option<&str>,
    file_list_hash: &str,
) -> bool {
    cache.version == CACHE_SCHEMA_VERSION
        && cache.niteo_version == niteo_version
        && cache.config_hash == config_hash
        && cache.tsconfig_hash.as_deref() == tsconfig_hash
        && cache.file_list_hash == file_list_hash
}

pub fn normalize_path_for_cache(path: &Path, project_root: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

pub fn denormalize_path_from_cache(path_str: &str, project_root: &Path) -> PathBuf {
    project_root.join(path_str)
}
