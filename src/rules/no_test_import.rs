use std::path::Path;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_TEST_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Production code may not import test files.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &RuleConfig,
    _tests_config: &DomainConfig,
) -> Vec<Violation> {
    let Some(file_node) = import_graph.file_node(file) else {
        return Vec::new();
    };

    if file_node.is_test {
        return Vec::new();
    }

    let mut violations = Vec::new();

    for edge in import_graph.edges_from(file) {
        let Some(target) = &edge.resolved_target else {
            continue;
        };

        let Some(target_node) = import_graph.file_node(target) else {
            continue;
        };

        if !target_node.is_test {
            continue;
        }

        let pos = line_index.position_for(edge.span);
        violations.push(Violation {
            file: file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_TEST_IMPORT_RULE_ID,
            message: MESSAGE,
            severity: config.severity,
            detail: Some(format!("imports `{}`", edge.specifier)),
            subject: Some(edge.specifier.clone()),
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::syntax::LineIndex;

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

    fn run_check(source: &str, file_path: &str) -> Vec<Violation> {
        let files_with_sources = vec![
            (file_path, source),
            ("src/helper.test.ts", ""),
            ("src/setup.tests.ts", ""),
            ("tests/mock.ts", ""),
            ("src/tests/mock.ts", ""),
            ("src/auth.ts", ""),
            ("src/helper.tests.ts", ""),
            ("src/b.test.ts", ""),
        ];
        let graph = build_import_graph_from_sources(&files_with_sources, &test_domain(), None);
        let line_index = LineIndex::new(source);
        check_file(
            std::path::Path::new(file_path),
            &line_index,
            &graph,
            &test_config(),
            &test_domain(),
        )
    }

    #[test]
    fn reports_import_from_test_suffix() {
        let violations = run_check("import { helper } from './helper.test';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("./helper.test"));
    }

    #[test]
    fn reports_import_from_tests_suffix() {
        let violations = run_check("import { setup } from './setup.tests';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_import_from_tests_folder() {
        let violations = run_check("import { mock } from '../tests/mock';", "src/sub/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_export_from_test_file() {
        let violations = run_check("export { helper } from './helper.test';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_export_all_from_test_file() {
        let violations = run_check("export * from './helper.tests';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_dynamic_import_from_test_file() {
        let violations = run_check(
            "const helper = await import('./helper.test');",
            "src/auth.ts",
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_import_from_production_file() {
        let violations = run_check("import { auth } from './auth';", "src/service.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_imports_in_test_files() {
        let violations = run_check(
            "import { helper } from './helper.test';",
            "src/auth.test.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_imports_in_test_folder() {
        let violations = run_check("import { helper } from './helper.test';", "tests/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_package_imports() {
        let violations = run_check(
            "import { render } from '@testing-library/react';",
            "src/auth.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_bare_specifier_with_test_name() {
        let violations = run_check("import { test } from 'vitest';", "src/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_correct_line_position() {
        let source = "import { a } from './a';\nimport { b } from './b.test';\n";
        let violations = run_check(source, "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_export_named_without_source() {
        let violations = run_check("const x = 1; export { x };", "src/auth.ts");
        assert!(violations.is_empty());
    }
}
