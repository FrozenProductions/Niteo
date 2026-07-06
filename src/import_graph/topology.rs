use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::import_graph::helpers::normalize_path;
use crate::import_graph::model::ImportGraph;

/// Only resolved edges count as imports; unresolved specifiers must not satisfy
/// orphan-file checks.
pub fn compute_imported_files(graph: &ImportGraph) -> HashSet<PathBuf> {
    graph
        .edges()
        .iter()
        .filter_map(|edge| edge.resolved_target.as_ref().map(|p| normalize_path(p)))
        .collect()
}

/// Each file in a strongly connected component gets a cycle entry so rules
/// can choose whether to report one or all nodes.
pub fn compute_cycles(graph: &ImportGraph) -> HashMap<PathBuf, Vec<PathBuf>> {
    let adjacency = graph.edges_by_target();
    let sccs = find_strongly_connected_components(adjacency);

    let mut cycles_by_file = HashMap::new();

    for mut scc in sccs {
        let Some(&first) = scc.first() else {
            continue;
        };

        let is_cyclic = if scc.len() > 1 {
            true
        } else {
            adjacency
                .get(first as usize)
                .is_some_and(|neighbors| neighbors.contains(&first))
        };

        if !is_cyclic {
            continue;
        }

        scc.sort_unstable();

        for &node in &scc {
            let node_cycle = reconstruct_cycle(node, &scc, adjacency, graph);
            cycles_by_file.insert(graph.files[node as usize].path.clone(), node_cycle);
        }
    }

    cycles_by_file
}

pub fn find_strongly_connected_components(adjacency: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let node_count = adjacency.len();
    let mut visited = vec![false; node_count];
    let mut finish_order = Vec::with_capacity(node_count);

    for node in 0..node_count as u32 {
        if !visited[node as usize] {
            dfs_finish(node, adjacency, &mut visited, &mut finish_order);
        }
    }

    let mut transpose: Vec<Vec<u32>> = vec![Vec::new(); node_count];
    for source in 0..node_count as u32 {
        for &target in &adjacency[source as usize] {
            transpose[target as usize].push(source);
        }
    }
    for neighbors in transpose.iter_mut() {
        neighbors.sort_unstable();
        neighbors.dedup();
    }

    visited.fill(false);
    let mut sccs = Vec::new();

    for &node in finish_order.iter().rev() {
        if !visited[node as usize] {
            let mut scc = Vec::new();
            dfs_collect(node, &transpose, &mut visited, &mut scc);
            sccs.push(scc);
        }
    }

    sccs
}

fn dfs_finish(
    start: u32,
    adjacency: &[Vec<u32>],
    visited: &mut [bool],
    finish_order: &mut Vec<u32>,
) {
    let mut stack: Vec<(u32, usize)> = vec![(start, 0)];
    visited[start as usize] = true;

    while let Some((node, idx)) = stack.last_mut() {
        if let Some(neighbors) = adjacency.get(*node as usize) {
            if *idx < neighbors.len() {
                let next = neighbors[*idx];
                *idx += 1;
                if !visited[next as usize] {
                    visited[next as usize] = true;
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(*node);
                stack.pop();
            }
        } else {
            finish_order.push(*node);
            stack.pop();
        }
    }
}

fn dfs_collect(start: u32, adjacency: &[Vec<u32>], visited: &mut [bool], collected: &mut Vec<u32>) {
    let mut stack = vec![start];
    visited[start as usize] = true;

    while let Some(node) = stack.pop() {
        collected.push(node);
        if let Some(neighbors) = adjacency.get(node as usize) {
            for &neighbor in neighbors.iter().rev() {
                if !visited[neighbor as usize] {
                    visited[neighbor as usize] = true;
                    stack.push(neighbor);
                }
            }
        }
    }
}

fn reconstruct_cycle(
    canonical: u32,
    scc: &[u32],
    adjacency: &[Vec<u32>],
    graph: &ImportGraph,
) -> Vec<PathBuf> {
    let path = if scc.len() == 1 {
        vec![canonical, canonical, canonical]
    } else {
        let scc_set: HashSet<u32> = scc.iter().copied().collect();
        let mut path = vec![canonical];
        let mut visited: HashSet<u32> = HashSet::new();
        visited.insert(canonical);

        if dfs_cycle(canonical, adjacency, &scc_set, &mut visited, &mut path) {
            path
        } else {
            vec![canonical, canonical]
        }
    };

    path.into_iter()
        .map(|index| graph.files[index as usize].path.clone())
        .collect()
}

fn dfs_cycle(
    start: u32,
    adjacency: &[Vec<u32>],
    scc_set: &HashSet<u32>,
    visited: &mut HashSet<u32>,
    path: &mut Vec<u32>,
) -> bool {
    let mut stack: Vec<(u32, usize)> = vec![(start, 0)];

    while let Some((node, idx)) = stack.last_mut() {
        let Some(neighbors) = adjacency.get(*node as usize) else {
            path.pop();
            stack.pop();
            continue;
        };

        if *idx < neighbors.len() {
            let neighbor = neighbors[*idx];
            *idx += 1;

            if neighbor == start && path.len() > 1 {
                path.push(neighbor);
                return true;
            }

            if !scc_set.contains(&neighbor) || visited.contains(&neighbor) {
                continue;
            }

            visited.insert(neighbor);
            path.push(neighbor);
            stack.push((neighbor, 0));
        } else {
            path.pop();
            stack.pop();
        }
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
