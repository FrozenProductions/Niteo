use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
    build_import_graph_with_cache(files, is_test_file, tsconfig, &HashMap::new())
}

pub fn build_import_graph_with_cache(
    files: &[PathBuf],
    is_test_file: impl Fn(&Path) -> bool,
    tsconfig: Option<&TsConfig>,
    cached_edges: &HashMap<PathBuf, Vec<ImportEdge>>,
) -> Result<ImportGraph> {
    let mut graph = ImportGraph::new();

    for file in files {
        let is_barrel = is_barrel_file(file);
        let is_test = is_test_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    let resolver = ImportResolverIndex::new(files, tsconfig);

    for file in files {
        if let Some(edges) = cached_edges.get(file) {
            graph.edges.extend(edges.clone());
        } else {
            let source = std::fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let edges = extract_imports(file, &source, &resolver);
            graph.edges.extend(edges);
        }
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

    for (path, source) in files_with_sources {
        let file = PathBuf::from(path);
        let edges = extract_imports(&file, source, &resolver);
        graph.edges.extend(edges);
    }

    graph.build_edges_by_source();

    graph
}
