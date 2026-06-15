use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cache::edges::{CachedImportEdge, cached_import_edges_to_import, import_edge_to_cached};
use crate::cache::key::{
    CACHE_SCHEMA_VERSION, hash_config_files, hash_content, hash_file_list, hash_tsconfig,
    is_cache_valid, normalize_path_for_cache,
};
use crate::cache::store::{CacheFile, CachedFileAnalysis, read_cache, write_cache};
use crate::import_graph::{ImportEdge, ImportGraph};

#[derive(Debug)]
pub struct CacheState {
    #[allow(dead_code)]
    pub cache: Option<CacheFile>,
    pub file_hashes: HashMap<PathBuf, String>,
    pub cached_edges: HashMap<PathBuf, Vec<ImportEdge>>,
    pub dirty: bool,
}

pub fn prepare_cache(
    project_root: &Path,
    files: &[PathBuf],
    config_paths: &[PathBuf],
    tsconfig_path: Option<&Path>,
) -> Result<Option<CacheState>> {
    let niteo_version = env!("CARGO_PKG_VERSION");
    let file_list_hash = hash_file_list(files);
    let config_hash = hash_config_files(config_paths);
    let tsconfig_hash = tsconfig_path.map(hash_tsconfig);

    let mut cache = read_cache(project_root)?;

    let cache_valid = cache
        .as_ref()
        .map(|c| {
            is_cache_valid(
                c,
                niteo_version,
                &config_hash,
                tsconfig_hash.as_deref(),
                &file_list_hash,
            )
        })
        .unwrap_or(false);

    if !cache_valid {
        cache = None;
    }

    let mut file_hashes = HashMap::new();
    let mut cached_edges = HashMap::new();
    let mut dirty = !cache_valid;

    for file in files {
        let content = match std::fs::read(file) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let hash = hash_content(&content);
        file_hashes.insert(file.clone(), hash.clone());

        if let Some(ref cache) = cache {
            let rel_path = normalize_path_for_cache(file, project_root);
            if let Some(entry) = cache.files.get(&rel_path)
                && entry.content_hash == hash
            {
                let edges = cached_import_edges_to_import(&entry.import_edges, file, project_root);
                cached_edges.insert(file.clone(), edges);
            } else {
                dirty = true;
            }
        } else {
            dirty = true;
        }
    }

    Ok(Some(CacheState {
        cache,
        file_hashes,
        cached_edges,
        dirty,
    }))
}

pub fn finalize_cache(
    project_root: &Path,
    files: &[PathBuf],
    config_paths: &[PathBuf],
    tsconfig_path: Option<&Path>,
    cache_state: &CacheState,
    graph: &ImportGraph,
) -> Result<()> {
    if !cache_state.dirty {
        return Ok(());
    }

    let niteo_version = env!("CARGO_PKG_VERSION");
    let file_list_hash = hash_file_list(files);
    let config_hash = hash_config_files(config_paths);
    let tsconfig_hash = tsconfig_path.map(hash_tsconfig);

    let mut new_cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: niteo_version.to_string(),
        config_hash,
        tsconfig_hash,
        file_list_hash,
        files: HashMap::new(),
    };

    for file in files {
        let rel_path = normalize_path_for_cache(file, project_root);
        let content_hash = cache_state
            .file_hashes
            .get(file)
            .cloned()
            .unwrap_or_else(|| match std::fs::read(file) {
                Ok(content) => hash_content(&content),
                Err(_) => String::new(),
            });

        let edges: Vec<CachedImportEdge> = graph
            .edges_from(file)
            .map(|edge| import_edge_to_cached(edge, project_root))
            .collect();

        new_cache.files.insert(
            rel_path,
            CachedFileAnalysis {
                content_hash,
                import_edges: edges,
                violations: Vec::new(),
                parse_failure: None,
            },
        );
    }

    write_cache(project_root, &new_cache)
}
