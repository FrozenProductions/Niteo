use std::collections::HashSet;
use std::path::Path;

use crate::config::NoOrphanFilesRuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_ORPHAN_FILES_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "File is not imported by any other file in the project.";

pub fn check_file(
    file: &Path,
    _line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &NoOrphanFilesRuleConfig,
) -> Vec<Violation> {
    let Some(file_node) = import_graph.file_node(file) else {
        return Vec::new();
    };

    if file_node.is_test {
        return Vec::new();
    }

    if is_entry_file(file, &config.entry_files) {
        return Vec::new();
    }

    let imported_files: HashSet<&Path> = import_graph
        .edges
        .iter()
        .filter_map(|edge| edge.resolved_target.as_deref())
        .collect();

    if imported_files.contains(file) {
        return Vec::new();
    }

    vec![Violation {
        file: file.to_path_buf(),
        line: Some(1),
        column: Some(1),
        rule: NO_ORPHAN_FILES_RULE_ID,
        message: MESSAGE,
        severity: config.severity,
        detail: None,
        subject: None,
    }]
}

fn is_entry_file(file: &Path, entry_files: &[String]) -> bool {
    let Some(stem) = file.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    entry_files.iter().any(|entry| entry == stem)
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::structure::DomainConfig;
    use crate::config::{NoOrphanFilesRuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use std::path::Path;

    fn test_config() -> NoOrphanFilesRuleConfig {
        NoOrphanFilesRuleConfig {
            severity: Severity::Warn,
            entry_files: vec!["main".to_string(), "app".to_string()],
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
    fn reports_orphan_file() {
        let files = vec![
            ("src/orphan.ts", "export const x = 1;\n"),
            ("src/used.ts", "export const y = 2;\n"),
            ("src/main.ts", "import { y } from './used';\n"),
        ];
        let violations = run_check("src/orphan.ts", &files);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn allows_imported_file() {
        let files = vec![
            ("src/used.ts", "export const y = 2;\n"),
            ("src/main.ts", "import { y } from './used';\n"),
        ];
        let violations = run_check("src/used.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_entry_file() {
        let files = vec![
            ("src/main.ts", "console.log('entry');\n"),
        ];
        let violations = run_check("src/main.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_app_entry_file() {
        let files = vec![
            ("src/app.ts", "console.log('app');\n"),
        ];
        let violations = run_check("src/app.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_test_file() {
        let files = vec![
            ("src/utils.test.ts", "import { test } from 'vitest';\n"),
        ];
        let violations = run_check("src/utils.test.ts", &files);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_file_only_importing_external() {
        let files = vec![
            ("src/lonely.ts", "import { z } from 'zod';\n"),
        ];
        let violations = run_check("src/lonely.ts", &files);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_re_exported_file() {
        let files = vec![
            ("src/utils.ts", "export const x = 1;\n"),
            ("src/index.ts", "export { x } from './utils';\n"),
            ("src/main.ts", "import { x } from './index';\n"),
        ];
        let violations = run_check("src/utils.ts", &files);
        assert!(violations.is_empty());
    }
}
