use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::cache::edges::{CachedImportEdge, cached_import_edges_to_import, import_edge_to_cached};
use crate::cache::key::{
    CACHE_SCHEMA_VERSION, denormalize_path_from_cache, hash_config_files, hash_content,
    hash_file_list, hash_tsconfig, is_cache_valid, normalize_path_for_cache,
};
use crate::cache::store::{
    CacheFile, CachedCycle, CachedFileAnalysis, CachedGraph, CachedParseFailure, read_cache,
    write_cache,
};
use crate::cache::violations::{
    StringInterner, build_rule_lookup, cached_violations_to_violations, violation_to_cached,
};
use crate::import_graph::{ImportEdge, ImportGraph};
use crate::rules::Violation;

#[derive(Debug)]
pub struct CacheState {
    pub file_hashes: HashMap<PathBuf, String>,
    pub cached_edges: HashMap<PathBuf, Vec<ImportEdge>>,
    pub cached_violations: Arc<HashMap<PathBuf, Vec<Violation>>>,
    pub cached_parse_failures: HashMap<PathBuf, CachedParseFailure>,
    pub cached_topology: Option<CachedGraph>,
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
    let mut cached_violations_map = HashMap::new();
    let mut cached_parse_failures = HashMap::new();
    let mut dirty = !cache_valid;
    let cached_topology = cache.as_ref().and_then(|cache| cache.graph.clone());

    let rule_lookup = build_rule_lookup();
    let mut message_interner = StringInterner::new();

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

                let violations = cached_violations_to_violations(
                    &entry.violations,
                    file.clone(),
                    &rule_lookup,
                    &mut message_interner,
                );
                cached_violations_map.insert(file.clone(), violations);

                if let Some(ref parse_failure) = entry.parse_failure {
                    cached_parse_failures.insert(file.clone(), parse_failure.clone());
                }
            } else {
                dirty = true;
            }
        } else {
            dirty = true;
        }
    }

    Ok(Some(CacheState {
        file_hashes,
        cached_edges,
        cached_violations: Arc::new(cached_violations_map),
        cached_parse_failures,
        cached_topology,
        dirty,
    }))
}

#[allow(clippy::too_many_arguments)]
pub fn finalize_cache(
    project_root: &Path,
    files: &[PathBuf],
    config_paths: &[PathBuf],
    tsconfig_path: Option<&Path>,
    cache_state: &CacheState,
    graph: &ImportGraph,
    violations: &[Violation],
    parse_failures: &HashMap<PathBuf, String>,
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
        graph: Some(graph_to_cached_graph(graph, project_root)),
    };

    let violations_by_file = group_violations_by_file(violations);

    for file in files {
        let rel_path = normalize_path_for_cache(file, project_root);
        let content_hash = cache_state
            .file_hashes
            .get(file)
            .cloned()
            .unwrap_or_else(|| match std::fs::read(file) {
                Ok(content) => hash_content(&content),
                Err(_) => String::from("<unreadable-file>"),
            });

        let edges: Vec<CachedImportEdge> = graph
            .edges_from(file)
            .map(|edge| import_edge_to_cached(edge, project_root))
            .collect();

        let cached = cache_state.cached_violations.get(file);
        let (file_violations, parse_failure) = if let Some(cached_violations) = cached {
            let cached_parse_failure = cache_state.cached_parse_failures.get(file).cloned();
            (
                cached_violations.iter().map(violation_to_cached).collect(),
                cached_parse_failure,
            )
        } else {
            let new_violations = violations_by_file
                .get(file)
                .map(|file_violations| {
                    file_violations
                        .iter()
                        .map(|v| violation_to_cached(v))
                        .collect()
                })
                .unwrap_or_default();
            let parse_failure = parse_failures.get(file).map(|message| CachedParseFailure {
                message: message.clone(),
            });
            (new_violations, parse_failure)
        };

        new_cache.files.insert(
            rel_path,
            CachedFileAnalysis {
                content_hash,
                import_edges: edges,
                violations: file_violations,
                parse_failure,
            },
        );
    }

    write_cache(project_root, &new_cache)
}

fn group_violations_by_file(violations: &[Violation]) -> HashMap<PathBuf, Vec<&Violation>> {
    let mut grouped: HashMap<PathBuf, Vec<&Violation>> = HashMap::new();
    for violation in violations {
        grouped
            .entry(violation.file.clone())
            .or_default()
            .push(violation);
    }
    grouped
}

pub fn cached_graph_to_cycles(
    cached: &CachedGraph,
    project_root: &Path,
) -> HashMap<PathBuf, Vec<PathBuf>> {
    cached
        .cycles
        .iter()
        .map(|cycle| {
            (
                denormalize_path_from_cache(&cycle.canonical, project_root),
                cycle
                    .files
                    .iter()
                    .map(|path| denormalize_path_from_cache(path, project_root))
                    .collect(),
            )
        })
        .collect()
}

pub fn cached_graph_to_imported_files(
    cached: &CachedGraph,
    project_root: &Path,
) -> HashSet<PathBuf> {
    cached
        .imported_files
        .iter()
        .map(|path| denormalize_path_from_cache(path, project_root))
        .collect()
}

fn graph_to_cached_graph(graph: &ImportGraph, project_root: &Path) -> CachedGraph {
    let mut cycles: Vec<CachedCycle> = graph
        .cycles_by_file()
        .map(|cycles| {
            cycles
                .iter()
                .map(|(canonical, files)| {
                    let mut files: Vec<String> = files
                        .iter()
                        .map(|path| normalize_path_for_cache(path, project_root))
                        .collect();
                    files.sort();
                    CachedCycle {
                        canonical: normalize_path_for_cache(canonical, project_root),
                        files,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    cycles.sort_by(|a, b| a.canonical.cmp(&b.canonical));

    let mut imported_files = graph
        .imported_files()
        .map(|files| {
            let mut paths: Vec<String> = files
                .iter()
                .map(|path| normalize_path_for_cache(path, project_root))
                .collect();
            paths.sort();
            paths
        })
        .unwrap_or_default();
    imported_files.sort();

    CachedGraph {
        edge_hash: graph.compute_edge_hash(),
        cycles,
        imported_files,
    }
}

pub fn ensure_graph_topology(graph: &mut ImportGraph) {
    if graph.cycles_by_file().is_none() {
        graph.set_cycles_by_file(crate::import_graph::topology::compute_cycles(graph));
    }
    if graph.imported_files().is_none() {
        graph.set_imported_files(crate::import_graph::topology::compute_imported_files(graph));
    }
}
