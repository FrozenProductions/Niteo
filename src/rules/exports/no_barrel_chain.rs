use std::path::Path;

use crate::config::RuleConfig;
use crate::import_graph::{ImportGraph, ImportKind};
use crate::rules::{NO_BARREL_CHAIN_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Barrel files cannot re-export from other barrel files.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &RuleConfig,
) -> Vec<Violation> {
    let Some(file_node) = import_graph.file_node(file) else {
        return Vec::new();
    };

    if !file_node.is_barrel {
        return Vec::new();
    }

    let mut violations = Vec::new();

    for edge in import_graph.edges_from(file) {
        if edge.kind != ImportKind::ReExport {
            continue;
        }

        let Some(target) = &edge.resolved_target else {
            continue;
        };

        let Some(target_node) = import_graph.file_node(target) else {
            continue;
        };

        if !target_node.is_barrel {
            continue;
        }

        let pos = line_index.position_for(edge.span);
        violations.push(Violation {
            file: file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_BARREL_CHAIN_RULE_ID,
            message: MESSAGE,
            severity: config.severity,
            detail: Some(format!(
                "Re-export target '{}' resolves to another index.ts barrel.",
                edge.specifier
            )),
            subject: Some(edge.specifier.clone()),
        });
    }

    violations
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::check_file;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use std::path::Path;

    #[test]
    fn reports_re_export_from_folder_barrel() -> Result<()> {
        let files_with_sources = vec![
            ("src/components/index.ts", "export { Button } from './button';\n"),
            ("src/components/button/index.ts", ""),
        ];
        let violations = run_check(
            "src/components/index.ts",
            &files_with_sources,
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
        assert_eq!(violations[0].subject.as_deref(), Some("./button"));
    
        Ok(())}

    #[test]
    fn reports_re_export_from_explicit_barrel_file() -> Result<()> {
        let files_with_sources = vec![
            ("src/index.ts", "export * from './components/index';\n"),
            ("src/components/index.ts", ""),
        ];
        let violations = run_check("src/index.ts", &files_with_sources);

        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_type_re_export_from_barrel() -> Result<()> {
        let files_with_sources = vec![
            ("src/index.ts", "export type { Props } from './types';\n"),
            ("src/types/index.ts", ""),
        ];
        let violations = run_check("src/index.ts", &files_with_sources);

        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_re_export_from_non_barrel_file() -> Result<()> {
        let files_with_sources = vec![
            ("src/index.ts", "export { Button } from './Button';\n"),
            ("src/Button.ts", ""),
        ];
        let violations = run_check("src/index.ts", &files_with_sources);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_non_barrel_files() -> Result<()> {
        let files_with_sources = vec![
            ("src/Button.ts", "export { Button } from './components';\n"),
            ("src/components/index.ts", ""),
        ];
        let violations = run_check("src/Button.ts", &files_with_sources);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_external_re_exports() -> Result<()> {
        let files_with_sources = vec![
            ("src/index.ts", "export { z } from 'zod';\n"),
        ];
        let violations = run_check("src/index.ts", &files_with_sources);

        assert!(violations.is_empty());
    
        Ok(())}

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
        let source = files_with_sources
            .iter()
            .find(|(path, _)| *path == file_path)
            .map(|(_, source)| *source)
            .unwrap_or("");
        let line_index = LineIndex::new(source);
        check_file(Path::new(file_path), &line_index, &graph, &test_config())
    }
}
