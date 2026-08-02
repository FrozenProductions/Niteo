use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::import_graph::extract::extract_imports;
use crate::import_graph::helpers::is_barrel_file;
use crate::import_graph::model::{ImportEdge, ImportGraph};
use crate::import_resolver::ImportResolverIndex;
use crate::syntax::ParseFailure;
use crate::tsconfig::TsConfig;
use crate::workspace::WorkspaceResolver;

#[cfg(test)]
use crate::config::structure::DomainConfig;

const PAR_CHUNK_SIZE: usize = 128;
const PROGRESS_INTERVAL: usize = 16;

fn extract_for_file(
    file: &Path,
    resolver: &ImportResolverIndex,
    cached_edges: &HashMap<PathBuf, &[ImportEdge]>,
    sources: &HashMap<PathBuf, String>,
) -> Result<(Vec<ImportEdge>, Vec<ParseFailure>)> {
    if let Some(edges) = cached_edges.get(file) {
        return Ok((edges.to_vec(), Vec::new()));
    }
    if let Some(source) = sources.get(file) {
        let (edges, failures) = extract_imports(file, source, resolver);
        return Ok((edges, failures));
    }
    let source = std::fs::read_to_string(file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let (edges, failures) = extract_imports(file, &source, resolver);
    Ok((edges, failures))
}

pub fn build_import_graph(
    files: &[PathBuf],
    is_test_file: impl Fn(&Path) -> bool,
    tsconfig: Option<&TsConfig>,
) -> Result<ImportGraph> {
    build_import_graph_with_cache(
        files,
        is_test_file,
        tsconfig,
        None,
        &HashMap::new(),
        &HashMap::new(),
        0,
    )
}

pub fn build_import_graph_with_cache(
    files: &[PathBuf],
    is_test_file: impl Fn(&Path) -> bool,
    tsconfig: Option<&TsConfig>,
    workspace: Option<&WorkspaceResolver>,
    cached_edges: &HashMap<PathBuf, &[ImportEdge]>,
    sources: &HashMap<PathBuf, String>,
    verbose: u8,
) -> Result<ImportGraph> {
    let mut graph = ImportGraph::new();

    for file in files {
        let is_barrel = is_barrel_file(file);
        let is_test = is_test_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    let resolver = ImportResolverIndex::new(files, tsconfig, workspace);

    let total = files.len();
    let progress_bar = if verbose >= 2 && total > 0 {
        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-"),
        );
        bar.set_message("parsing imports");
        Some(bar)
    } else {
        None
    };
    let processed = AtomicUsize::new(0);

    let extracted: Vec<Result<(Vec<ImportEdge>, Vec<ParseFailure>)>> = if total > PAR_CHUNK_SIZE * 2
    {
        files
            .par_chunks(PAR_CHUNK_SIZE)
            .flat_map_iter(|chunk| {
                chunk.iter().map(|file| {
                    let result = extract_for_file(file, &resolver, cached_edges, sources);
                    let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    if (count.is_multiple_of(PROGRESS_INTERVAL) || count == total)
                        && let Some(ref bar) = progress_bar
                    {
                        bar.set_position(count as u64);
                    }
                    result
                })
            })
            .collect()
    } else {
        files
            .par_iter()
            .map(|file| {
                let result = extract_for_file(file, &resolver, cached_edges, sources);
                let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(ref bar) = progress_bar
                    && (count.is_multiple_of(PROGRESS_INTERVAL) || count == total)
                {
                    bar.set_position(count as u64);
                }
                result
            })
            .collect()
    };

    let extracted: Vec<(Vec<ImportEdge>, Vec<ParseFailure>)> =
        extracted.into_iter().collect::<Result<Vec<_>>>()?;

    if let Some(bar) = progress_bar {
        bar.finish_and_clear();
    }

    for (_file, (edges, failures)) in files.iter().zip(extracted) {
        graph.extend_edges(edges);
        for failure in failures {
            graph.add_parse_failure(failure);
        }
    }

    graph.build_edges_by_source();
    let _hash = graph.compute_edge_hash();

    Ok(graph)
}

#[cfg(test)]
pub fn build_import_graph_from_sources(
    files_with_sources: &[(&str, &str)],
    tests_config: &DomainConfig,
    tsconfig: Option<&TsConfig>,
) -> ImportGraph {
    build_import_graph_from_sources_with_workspace(files_with_sources, tests_config, tsconfig, None)
}

#[cfg(test)]
pub fn build_import_graph_from_sources_with_workspace(
    files_with_sources: &[(&str, &str)],
    tests_config: &DomainConfig,
    tsconfig: Option<&TsConfig>,
    workspace: Option<&WorkspaceResolver>,
) -> ImportGraph {
    let mut graph = ImportGraph::new();
    let files: Vec<PathBuf> = files_with_sources
        .iter()
        .map(|(path, _)| PathBuf::from(path))
        .collect();

    for file in &files {
        let is_barrel = is_barrel_file(file);
        let is_test = tests_config.matches_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    let resolver = ImportResolverIndex::new(&files, tsconfig, workspace);

    let extracted: Vec<(Vec<ImportEdge>, Vec<ParseFailure>)> = files_with_sources
        .par_iter()
        .map(|(path, source)| {
            let file = PathBuf::from(path);
            extract_imports(&file, source, &resolver)
        })
        .collect();

    for (_file, (edges, failures)) in files.iter().zip(extracted.into_iter()) {
        graph.extend_edges(edges);
        for failure in failures {
            graph.add_parse_failure(failure);
        }
    }

    graph.build_edges_by_source();
    let _hash = graph.compute_edge_hash();

    graph
}
