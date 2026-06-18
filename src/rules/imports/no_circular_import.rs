use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::import_graph::topology::compute_cycles;
use crate::rules::{NO_CIRCULAR_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Circular import chain detected.";

pub struct CircularImportContext {
    cycles_by_file: HashMap<PathBuf, Vec<PathBuf>>,
}

impl CircularImportContext {
    pub fn new(import_graph: &ImportGraph) -> Self {
        let cycles_by_file = import_graph
            .cycles_by_file()
            .cloned()
            .unwrap_or_else(|| compute_cycles(import_graph));
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
                span: Some(edge.span),
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

        use anyhow::{Context, Result};
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
        let graph = build_import_graph_from_sources(files_with_sources, &test_domain(), None);
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
    fn detects_direct_cycle() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations_a = run_check("src/a.ts", &files);
        let violations_b = run_check("src/b.ts", &files);

        assert_eq!(violations_a.len(), 1);
        assert!(violations_a[0]
            .detail
            .as_ref()
            .context("expected detail")?
            .contains("->"));
        assert_eq!(violations_b.len(), 0);
        Ok(())
    }

    #[test]
    fn detects_three_node_cycle() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn no_cycle_in_acyclic_graph() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", ""),
        ];
        let violations = run_check("src/a.ts", &files);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn no_cycle_with_external_imports() -> Result<()> {
        let files = vec![("src/a.ts", "import { z } from 'zod';\n")];
        let violations = run_check("src/a.ts", &files);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_line_and_column() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
        Ok(())
    }

    #[test]
    fn self_import_is_a_cycle() -> Result<()> {
        let files = vec![("src/a.ts", "import { a } from './a';\n")];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn two_independent_cycles_both_report() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn duplicate_imports_do_not_duplicate_reports() -> Result<()> {
        let files = vec![
            (
                "src/a.ts",
                "import { b } from './b';\nimport { b2 } from './b';\n",
            ),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn deterministic_reporting_with_multiple_outgoing() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn non_canonical_files_return_no_violations() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations_b = run_check("src/b.ts", &files);
        let violations_c = run_check("src/c.ts", &files);

        assert_eq!(violations_b.len(), 0);
        assert_eq!(violations_c.len(), 0);
        Ok(())
    }

    #[test]
    fn cycle_detail_format() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(
            violations[0].detail.as_ref().context("expected detail")?,
            "a.ts -> b.ts -> a.ts"
        );
        Ok(())
    }

    #[test]
    fn three_node_cycle_detail_format() -> Result<()> {
        let files = vec![
            ("src/a.ts", "import { b } from './b';\n"),
            ("src/b.ts", "import { c } from './c';\n"),
            ("src/c.ts", "import { a } from './a';\n"),
        ];
        let violations = run_check("src/a.ts", &files);
        assert_eq!(
            violations[0].detail.as_ref().context("expected detail")?,
            "a.ts -> b.ts -> c.ts -> a.ts"
        );
        Ok(())
    }
}
