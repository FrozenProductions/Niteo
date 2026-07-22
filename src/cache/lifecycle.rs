use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;

use crate::cache::edges::{CachedImportEdge, cached_import_edges_to_import, import_edge_to_cached};
use crate::cache::key::{
    CACHE_SCHEMA_VERSION, compute_rule_hashes, denormalize_path_from_cache, hash_content,
    hash_file_list, hash_tsconfig, is_cache_valid, normalize_path_for_cache,
};
use crate::cache::store::{
    CacheFile, CachedCycle, CachedFileAnalysis, CachedGraph, CachedParseFailure, CachedViolation,
    read_cache, write_cache,
};
use crate::cache::violations::{
    StringInterner, build_rule_lookup, cached_violations_to_violations, violation_to_cached,
};
use crate::import_graph::{ImportEdge, ImportGraph};
use crate::rules::Violation;

#[derive(Debug)]
pub struct CacheState {
    pub file_hashes: HashMap<PathBuf, String>,
    pub sources: HashMap<PathBuf, String>,
    pub cached_edges: HashMap<PathBuf, Vec<ImportEdge>>,
    pub cached_violations: Arc<HashMap<PathBuf, Vec<Violation>>>,
    pub cached_parse_failures: HashMap<PathBuf, CachedParseFailure>,
    pub cached_topology: Option<CachedGraph>,
    pub dirty: bool,
    pub file_list_hash: String,
    pub rule_hashes: HashMap<String, String>,
    pub changed_rules: Arc<HashSet<crate::rules::RuleId>>,
    pub tsconfig_hash: Option<String>,
}

pub fn prepare_cache(
    project_root: &Path,
    files: &[PathBuf],
    config_set: &crate::config::ConfigSet,
    tsconfig_path: Option<&Path>,
) -> Result<Option<CacheState>> {
    let niteo_version = env!("CARGO_PKG_VERSION");
    let file_list_hash = hash_file_list(files);
    let rule_hashes = compute_rule_hashes(config_set);
    let tsconfig_hash = tsconfig_path.map(hash_tsconfig);

    let mut cache = read_cache(project_root)?;

    let cache_valid = cache
        .as_ref()
        .map(|c| is_cache_valid(c, niteo_version, tsconfig_hash.as_deref(), &file_list_hash))
        .unwrap_or(false);

    if !cache_valid {
        cache = None;
    }

    let cached_rule_hashes = cache.as_ref().map(|c| &c.rule_hashes);
    let mut changed_rules: HashSet<crate::rules::RuleId> = HashSet::new();
    if let Some(cached_hashes) = cached_rule_hashes {
        for rule_id in crate::rules::known_rule_ids() {
            let current_hash = rule_hashes.get(*rule_id);
            let cached_hash = cached_hashes.get(*rule_id);
            if current_hash != cached_hash {
                changed_rules.insert(*rule_id);
            }
        }
    }
    let changed_rules = Arc::new(changed_rules);

    let mut dirty = !cache_valid || !changed_rules.is_empty();
    let cached_topology = cache.as_ref().and_then(|cache| cache.graph.clone());

    struct CacheHit {
        edges: Vec<ImportEdge>,
        violations: Vec<CachedViolation>,
        parse_failure: Option<CachedParseFailure>,
    }

    struct PreparedFile {
        file: PathBuf,
        hash: String,
        hit: Option<CacheHit>,
        source: Option<String>,
    }

    let prepared: Vec<PreparedFile> = files
        .par_iter()
        .map(|file| match std::fs::read(file) {
            Ok(content) => {
                let hash = hash_content(&content);
                let source = String::from_utf8_lossy(&content).into_owned();
                let hit = cache.as_ref().and_then(|cache| {
                    let rel_path = normalize_path_for_cache(file, project_root);
                    let entry = cache.files.get(&rel_path)?;
                    if entry.content_hash != hash {
                        return None;
                    }
                    let edges =
                        cached_import_edges_to_import(&entry.import_edges, file, project_root);
                    Some(CacheHit {
                        edges,
                        violations: entry.violations.clone(),
                        parse_failure: entry.parse_failure.clone(),
                    })
                });
                PreparedFile {
                    file: file.clone(),
                    hash,
                    hit,
                    source: Some(source),
                }
            }
            Err(_) => PreparedFile {
                file: file.clone(),
                hash: String::new(),
                hit: None,
                source: None,
            },
        })
        .collect();

    let mut file_hashes = HashMap::with_capacity(prepared.len());
    let mut sources = HashMap::with_capacity(prepared.len());
    let mut cached_edges = HashMap::new();
    let mut cached_violations_map = HashMap::new();
    let mut cached_parse_failures = HashMap::new();

    let rule_lookup = build_rule_lookup();
    let mut message_interner = StringInterner::new();

    for pf in prepared {
        if pf.hash.is_empty() {
            continue;
        }

        file_hashes.insert(pf.file.clone(), pf.hash);
        if let Some(source) = pf.source {
            sources.insert(pf.file.clone(), source);
        }

        match pf.hit {
            Some(hit) => {
                cached_edges.insert(pf.file.clone(), hit.edges);
                let violations = cached_violations_to_violations(
                    &hit.violations,
                    pf.file.clone(),
                    &rule_lookup,
                    &mut message_interner,
                );

                let kept: Vec<Violation> = violations
                    .into_iter()
                    .filter(|violation| {
                        let is_known = crate::rules::known_rule_ids().contains(&violation.rule);
                        let is_changed = changed_rules.contains(violation.rule);
                        is_known && !is_changed
                    })
                    .collect();
                cached_violations_map.insert(pf.file.clone(), kept);

                if let Some(parse_failure) = hit.parse_failure {
                    cached_parse_failures.insert(pf.file.clone(), parse_failure);
                }
            }
            None if cache_valid => dirty = true,
            None => {}
        }
    }

    Ok(Some(CacheState {
        file_hashes,
        sources,
        cached_edges,
        cached_violations: Arc::new(cached_violations_map),
        cached_parse_failures,
        cached_topology,
        dirty,
        file_list_hash,
        rule_hashes,
        changed_rules,
        tsconfig_hash,
    }))
}

pub fn finalize_cache(
    project_root: &Path,
    files: &[PathBuf],
    cache_state: &CacheState,
    graph: &ImportGraph,
    violations: &[Violation],
    parse_failures: &HashMap<PathBuf, String>,
) -> Result<()> {
    if !cache_state.dirty {
        return Ok(());
    }

    let niteo_version = env!("CARGO_PKG_VERSION");

    let mut new_cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: niteo_version.to_string(),
        rule_hashes: cache_state.rule_hashes.clone(),
        tsconfig_hash: cache_state.tsconfig_hash.clone(),
        file_list_hash: cache_state.file_list_hash.clone(),
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
        .map(|path| {
            crate::import_graph::helpers::normalize_path(&denormalize_path_from_cache(
                path,
                project_root,
            ))
        })
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
