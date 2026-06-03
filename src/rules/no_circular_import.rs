use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_CIRCULAR_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Circular import chain detected.";

pub struct CircularImportContext {
    cycles_by_file: HashMap<PathBuf, Vec<PathBuf>>,
}

impl CircularImportContext {
    pub fn new(import_graph: &ImportGraph) -> Self {
        let adjacency = build_adjacency(import_graph);
        let sccs = find_sccs(&adjacency);

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
            let Some(canonical) = sorted_scc.first().cloned() else {
                continue;
            };

            let cycle = reconstruct_cycle(&canonical, &sorted_scc, &adjacency);
            cycles_by_file.insert(canonical, cycle);
        }

        Self { cycles_by_file }
    }
}

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    context: &CircularImportContext,
    config: &RuleConfig,
) -> Vec<Violation> {
    let Some(cycle) = context.cycles_by_file.get(file) else {
        return Vec::new();
    };

    let cycle_display = format_cycle(cycle);
    let Some(target) = cycle.get(1) else {
        return Vec::new();
    };

    let mut violations = Vec::new();
    for edge in import_graph.edges_from(file) {
        let Some(resolved) = &edge.resolved_target else {
            continue;
        };
        if resolved == target {
            let pos = line_index.position_for(edge.span);
            violations.push(Violation {
                file: file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_CIRCULAR_IMPORT_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: Some(cycle_display.clone()),
                subject: Some(edge.specifier.clone()),
            });
            break;
        }
    }

    violations
}

fn build_adjacency(import_graph: &ImportGraph) -> HashMap<PathBuf, Vec<PathBuf>> {
    let mut adjacency: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for edge in &import_graph.edges {
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

fn find_sccs(adjacency: &HashMap<PathBuf, Vec<PathBuf>>) -> Vec<Vec<PathBuf>> {
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

    if dfs_cycle(canonical, canonical, adjacency, &scc_set, &mut visited, &mut path) {
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

fn format_cycle(cycle: &[PathBuf]) -> String {
    let names: Vec<&str> = cycle
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?")
        })
        .collect();
    names.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::{check_file, CircularImportContext};
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use std::path::Path;

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn test_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
        }
    }

    fn run_check(file_path: &str, files_with_sources: &[(&str, &str)]) -> Vec<Violation> {
        let graph = build_import_graph_from_sources(files_with_sources, &test_domain());
        let context = CircularImportContext::new(&graph);
        let source = files_with_sources
            .iter()
            .find(|(path, _)| *path == file_path)
            .map(|(_, source)| *source)
            .unwrap_or("");
        let line_index = LineIndex::new(source);
        check_file(
            Path::new(file_path),
            &line_index,
            &graph,
            &context,
            &test_config(),
        )
    }

    #[test]
    fn detects_direct_cycle() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations_a = run_check("src/a.ts", &files);
        let violations_b = run_check("src/b.ts", &files);

        assert_eq!(violations_a.len(), 1);
        assert!(violations_a[0].detail.as_ref().unwrap().contains("->"));
        assert_eq!(violations_b.len(), 0);
    }

    #[test]
    fn detects_three_node_cycle() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations_a = run_check("src/a.ts", &files);
        let violations_b = run_check("src/b.ts", &files);
        let violations_c = run_check("src/c.ts", &files);

        assert_eq!(violations_a.len(), 1);
        assert_eq!(violations_b.len(), 0);
        assert_eq!(violations_c.len(), 0);
    }

    #[test]
    fn no_cycle_in_acyclic_graph() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", ""),
        ];
        let violations = run_check("src/a.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn no_cycle_with_external_imports() {
        let files = vec![("src/a.ts", "import { z } from 'zod';\n")];
        let violations = run_check("src/a.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_line_and_column() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn self_import_is_a_cycle() {
        let files = vec![("src/a.ts", "import { a } from './a';\n")];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn two_independent_cycles_both_report() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
            ("src/c.ts", "import { d } from './d';\n"),
            ("src/d.ts", "import { c } from './c';\n"),
        ];
        let violations_a = run_check("src/a.ts", &files);
        let violations_c = run_check("src/c.ts", &files);

        assert_eq!(violations_a.len(), 1);
        assert_eq!(violations_c.len(), 1);
    }

    #[test]
    fn duplicate_imports_do_not_duplicate_reports() {
        let files = vec![
            (
                "src/a.ts",
                "import { b } from './b';\nimport { b2 } from './b';\n",
            ),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn deterministic_reporting_with_multiple_outgoing() {
        let files = vec![
            (
                "src/a.ts",
                "import { b } from './b';\nimport { c } from './c';\n",
            ),
            ("src/b.ts", "import { a } from './a';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations_1 = run_check("src/a.ts", &files);
        let violations_2 = run_check("src/a.ts", &files);

        assert_eq!(violations_1.len(), 1);
        assert_eq!(violations_2.len(), 1);
        assert_eq!(violations_1[0].detail, violations_2[0].detail);
        assert_eq!(violations_1[0].subject, violations_2[0].subject);
    }

    #[test]
    fn non_canonical_files_return_no_violations() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations_b = run_check("src/b.ts", &files);
        let violations_c = run_check("src/c.ts", &files);

        assert_eq!(violations_b.len(), 0);
        assert_eq!(violations_c.len(), 0);
    }

    #[test]
    fn cycle_detail_format() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(
            violations[0].detail.as_ref().unwrap(),
            "a.ts -> b.ts -> a.ts"
        );
    }

    #[test]
    fn three_node_cycle_detail_format() {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(
            violations[0].detail.as_ref().unwrap(),
            "a.ts -> b.ts -> c.ts -> a.ts"
        );
    }
}
