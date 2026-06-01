use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_CIRCULAR_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Circular import chain detected.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &RuleConfig,
) -> Vec<Violation> {
    let adjacency = build_adjacency(import_graph);

    let Some(cycle) = find_cycle_from(file, &adjacency) else {
        return Vec::new();
    };

    let canonical = cycle
        .iter()
        .take(cycle.len().saturating_sub(1))
        .min()
        .expect("cycle has at least one element");

    if canonical != &file.to_path_buf() {
        return Vec::new();
    }

    let cycle_display = format_cycle(&cycle);

    let mut violations = Vec::new();
    for edge in import_graph.edges_from(file) {
        let Some(target) = &edge.resolved_target else {
            continue;
        };
        if cycle.get(1) == Some(target) {
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
    adjacency
}

fn find_cycle_from(start: &Path, adjacency: &HashMap<PathBuf, Vec<PathBuf>>) -> Option<Vec<PathBuf>> {
    let mut visited = HashSet::new();
    let mut path = Vec::new();
    path.push(start.to_path_buf());

    if dfs(start, start, adjacency, &mut visited, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn dfs(
    start: &Path,
    current: &Path,
    adjacency: &HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
    path: &mut Vec<PathBuf>,
) -> bool {
    let Some(neighbors) = adjacency.get(current) else {
        return false;
    };

    let mut sorted_neighbors: Vec<&PathBuf> = neighbors.iter().collect();
    sorted_neighbors.sort();

    for neighbor in sorted_neighbors {
        if neighbor.as_path() == start && path.len() > 1 {
            path.push(neighbor.clone());
            return true;
        }
        if visited.contains(neighbor) {
            continue;
        }
        visited.insert(neighbor.clone());
        path.push(neighbor.clone());
        if dfs(start, neighbor, adjacency, visited, path) {
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
    use super::check_file;
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
        let source = files_with_sources
            .iter()
            .find(|(path, _)| *path == file_path)
            .map(|(_, source)| *source)
            .unwrap_or("");
        let line_index = LineIndex::new(source);
        check_file(Path::new(file_path), &line_index, &graph, &test_config())
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
        let files = vec![
            ("src/a.ts", "import { z } from 'zod';\n"),
        ];
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
        let files = vec![
            ("src/a.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations.len(), 1);
    }
}
