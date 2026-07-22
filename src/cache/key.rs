use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cache::store::CacheFile;

pub const CACHE_SCHEMA_VERSION: u32 = 4;

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
    let mut hasher = blake3::Hasher::new();
    for (index, path) in sorted.iter().enumerate() {
        if index > 0 {
            hasher.update(b"\n");
        }
        hasher.update(path.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
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
    tsconfig_hash: Option<&str>,
    file_list_hash: &str,
) -> bool {
    cache.version == CACHE_SCHEMA_VERSION
        && cache.niteo_version == niteo_version
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

/// Compute a per-rule hash from the resolved config set.
///
/// Each rule's hash captures:
/// - The rule's own options and severity in every config node.
/// - The project structure and architecture in every config node (some rules
///   depend on these indirectly via `TypeLocationStyle` or adapter context).
///
/// This lets unrelated rule option changes keep cached violations for rules
/// whose effective configuration did not change.
pub fn compute_rule_hashes(config_set: &crate::config::ConfigSet) -> HashMap<String, String> {
    let node_contexts: Vec<(String, String, String, HashMap<String, String>)> = config_set
        .configs()
        .map(|node| {
            let directory = node.directory.to_string_lossy().to_string();
            let structure = serde_json::to_string(&node.config.structure).unwrap_or_default();
            let architecture = serde_json::to_string(&node.config.architecture).unwrap_or_default();
            let rule_options = node.config.rules.rule_option_hashes();
            (directory, structure, architecture, rule_options)
        })
        .collect();

    let mut rule_hashes = HashMap::new();
    for rule_id in crate::rules::known_rule_ids() {
        let mut hasher = blake3::Hasher::new();
        for (directory, structure, architecture, rule_options) in &node_contexts {
            hasher.update(directory.as_bytes());
            hasher.update(b"\0");
            let option_json = rule_options.get(*rule_id).map(|s| s.as_str()).unwrap_or("");
            hasher.update(option_json.as_bytes());
            hasher.update(b"\0");
            hasher.update(structure.as_bytes());
            hasher.update(b"\0");
            hasher.update(architecture.as_bytes());
            hasher.update(b"\0");
        }
        rule_hashes.insert(rule_id.to_string(), hasher.finalize().to_hex().to_string());
    }
    rule_hashes
}
