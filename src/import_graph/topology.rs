use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::import_graph::model::ImportGraph;

/// Only resolved edges count as imports; unresolved specifiers must not satisfy
/// orphan-file checks.
pub fn compute_imported_files(graph: &ImportGraph) -> HashSet<PathBuf> {
    graph
        .edges
        .iter()
        .filter_map(|edge| edge.resolved_target.clone())
        .collect()
}

/// Each file in a strongly connected component gets a cycle entry so rules
/// can choose whether to report one or all nodes.
pub fn compute_cycles(graph: &ImportGraph) -> HashMap<PathBuf, Vec<PathBuf>> {
    let adjacency = build_adjacency(graph);
    let sccs = find_strongly_connected_components(&adjacency);

    let mut cycles_by_file = HashMap::new();

    for scc in sccs {
        let Some(first) = scc.first() else {
            continue;
        };

        let is_cyclic = if scc.len() > 1 {
            true
        } else {
            adjacency
                .get(first)
                .is_some_and(|neighbors| neighbors.contains(first))
        };

        if !is_cyclic {
            continue;
        }

        let mut sorted_scc = scc;
        sorted_scc.sort();

        for node in &sorted_scc {
            let node_cycle = reconstruct_cycle(node, &sorted_scc, &adjacency);
            cycles_by_file.insert(node.clone(), node_cycle);
        }
    }

    cycles_by_file
}

fn build_adjacency(graph: &ImportGraph) -> HashMap<PathBuf, Vec<PathBuf>> {
    let mut adjacency: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for edge in &graph.edges {
        if let Some(target) = &edge.resolved_target {
            adjacency
                .entry(edge.source_file.clone())
                .or_default()
                .push(target.clone());
        }
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort();
        neighbors.dedup();
    }
    adjacency
}

fn find_strongly_connected_components(
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
) -> Vec<Vec<PathBuf>> {
    let mut all_nodes: HashSet<PathBuf> = HashSet::new();
    for (source, targets) in adjacency {
        all_nodes.insert(source.clone());
        for target in targets {
            all_nodes.insert(target.clone());
        }
    }

    let mut sorted_nodes: Vec<PathBuf> = all_nodes.into_iter().collect();
    sorted_nodes.sort();

    let mut visited = HashSet::new();
    let mut finish_order = Vec::new();

    for node in &sorted_nodes {
        if !visited.contains(node) {
            dfs_finish(node, adjacency, &mut visited, &mut finish_order);
        }
    }

    let mut transpose: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (source, targets) in adjacency {
        for target in targets {
            transpose
                .entry(target.clone())
                .or_default()
                .push(source.clone());
        }
    }
    for neighbors in transpose.values_mut() {
        neighbors.sort();
        neighbors.dedup();
    }

    let mut visited = HashSet::new();
    let mut sccs = Vec::new();

    for node in finish_order.iter().rev() {
        if !visited.contains(node) {
            let mut scc = Vec::new();
            dfs_collect(node, &transpose, &mut visited, &mut scc);
            sccs.push(scc);
        }
    }

    sccs
}

fn dfs_finish(
    start: &Path,
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
    finish_order: &mut Vec<PathBuf>,
) {
    let mut stack: Vec<(PathBuf, usize)> = vec![(start.to_path_buf(), 0)];
    visited.insert(start.to_path_buf());

    while let Some((node, idx)) = stack.last_mut() {
        if let Some(neighbors) = adjacency.get(node) {
            if *idx < neighbors.len() {
                let next = neighbors[*idx].clone();
                *idx += 1;
                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(node.clone());
                stack.pop();
            }
        } else {
            finish_order.push(node.clone());
            stack.pop();
        }
    }
}

fn dfs_collect(
    start: &Path,
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
    collected: &mut Vec<PathBuf>,
) {
    let mut stack = vec![start.to_path_buf()];
    visited.insert(start.to_path_buf());

    while let Some(node) = stack.pop() {
        collected.push(node.clone());
        if let Some(neighbors) = adjacency.get(&node) {
            for neighbor in neighbors.iter().rev() {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    stack.push(neighbor.clone());
                }
            }
        }
    }
}

fn reconstruct_cycle(
    canonical: &PathBuf,
    scc: &[PathBuf],
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    if scc.len() == 1 {
        return vec![canonical.clone(), canonical.clone(), canonical.clone()];
    }

    let scc_set: HashSet<&PathBuf> = scc.iter().collect();
    let mut path = vec![canonical.clone()];
    let mut visited: HashSet<PathBuf> = HashSet::new();
    visited.insert(canonical.clone());

    if dfs_cycle(
        canonical,
        canonical,
        adjacency,
        &scc_set,
        &mut visited,
        &mut path,
    ) {
        path
    } else {
        vec![canonical.clone(), canonical.clone()]
    }
}

fn dfs_cycle(
    start: &PathBuf,
    current: &PathBuf,
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
    scc_set: &HashSet<&PathBuf>,
    visited: &mut HashSet<PathBuf>,
    path: &mut Vec<PathBuf>,
) -> bool {
    let Some(neighbors) = adjacency.get(current) else {
        return false;
    };

    for neighbor in neighbors {
        if neighbor == start && path.len() > 1 {
            path.push(neighbor.clone());
            return true;
        }
        if !scc_set.contains(neighbor) || visited.contains(neighbor) {
            continue;
        }
        visited.insert(neighbor.clone());
        path.push(neighbor.clone());
        if dfs_cycle(start, neighbor, adjacency, scc_set, visited, path) {
            return true;
        }
        path.pop();
    }

    false
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;

    use crate::config::structure::DomainConfig;
    use crate::import_graph::build_import_graph_from_sources;

    use super::*;

    fn test_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
        }
    }

    #[test]
    fn computes_imported_files() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "\n"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let imported = compute_imported_files(&graph);

        assert!(imported.contains(&PathBuf::from("src/b.ts")));
        assert!(!imported.contains(&PathBuf::from("src/a.ts")));
        Ok(())
    }

    #[test]
    fn computes_direct_cycle() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let cycles = compute_cycles(&graph);

        assert!(cycles.contains_key(&PathBuf::from("src/a.ts")));
        assert!(cycles.contains_key(&PathBuf::from("src/b.ts")));
        assert_eq!(cycles.len(), 2);
        Ok(())
    }

    #[test]
    fn computes_no_cycle_for_acyclic_graph() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "\n"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let cycles = compute_cycles(&graph);

        assert!(cycles.is_empty());
        Ok(())
    }
}
