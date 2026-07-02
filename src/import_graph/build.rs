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
use crate::tsconfig::TsConfig;

#[cfg(test)]
use crate::config::structure::DomainConfig;

pub fn build_import_graph(
    files: &[PathBuf],
    is_test_file: impl Fn(&Path) -> bool,
    tsconfig: Option<&TsConfig>,
) -> Result<ImportGraph> {
    build_import_graph_with_cache(files, is_test_file, tsconfig, &HashMap::new(), false)
}

pub fn build_import_graph_with_cache(
    files: &[PathBuf],
    is_test_file: impl Fn(&Path) -> bool,
    tsconfig: Option<&TsConfig>,
    cached_edges: &HashMap<PathBuf, Vec<ImportEdge>>,
    verbose: bool,
) -> Result<ImportGraph> {
    let mut graph = ImportGraph::new();

    for file in files {
        let is_barrel = is_barrel_file(file);
        let is_test = is_test_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    let resolver = ImportResolverIndex::new(files, tsconfig);

    let total = files.len();
    let progress_bar = if verbose && total > 0 {
        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );
        bar.set_message("parsing imports");
        Some(bar)
    } else {
        None
    };
    let processed = AtomicUsize::new(0);

    let extracted: Vec<Vec<ImportEdge>> = files
        .par_iter()
        .map(|file| {
            let result = if let Some(edges) = cached_edges.get(file) {
                Ok(edges.clone())
            } else {
                let source = std::fs::read_to_string(file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                Ok(extract_imports(file, &source, &resolver))
            };
            let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(ref bar) = progress_bar {
                bar.set_position(count as u64);
            }
            result
        })
        .collect::<Result<Vec<_>>>()?;

    if let Some(bar) = progress_bar {
        bar.finish_and_clear();
    }

    for edges in extracted {
        graph.edges.extend(edges);
    }

    graph.build_edges_by_source();

    Ok(graph)
}

#[cfg(test)]
pub fn build_import_graph_from_sources(
    files_with_sources: &[(&str, &str)],
    tests_config: &DomainConfig,
    tsconfig: Option<&TsConfig>,
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

    let resolver = ImportResolverIndex::new(&files, tsconfig);

    let extracted: Vec<Vec<ImportEdge>> = files_with_sources
        .par_iter()
        .map(|(path, source)| {
            let file = PathBuf::from(path);
            extract_imports(&file, source, &resolver)
        })
        .collect();

    for edges in extracted {
        graph.edges.extend(edges);
    }

    graph.build_edges_by_source();

    graph
}
