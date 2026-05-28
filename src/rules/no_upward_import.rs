use std::path::Path;

use crate::config::UpwardImportRuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_UPWARD_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Replace upward relative imports with local or project-root imports.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &UpwardImportRuleConfig,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for edge in import_graph.edges_from(file) {
        let depth = upward_depth(edge.specifier.as_bytes());
        if depth > config.max_depth {
            let pos = line_index.position_for(edge.span);
            violations.push(Violation {
                file: file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_UPWARD_IMPORT_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: None,
                subject: None,
            });
        }
    }

    violations
}

fn upward_depth(specifier: &[u8]) -> usize {
    specifier
        .split(|byte| *byte == b'/')
        .take_while(|segment| *segment == b"..")
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{Severity, UpwardImportRuleConfig};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::syntax::LineIndex;

    fn test_config() -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            severity: Severity::Warn,
            max_depth: 0,
        }
    }

    fn test_config_with_depth(max_depth: usize) -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            max_depth,
            ..test_config()
        }
    }

    fn test_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
        }
    }

    fn run_check(source: &str, config: &UpwardImportRuleConfig) -> Vec<Violation> {
        let files_with_sources = vec![("Button.ts", source)];
        let graph = build_import_graph_from_sources(&files_with_sources, &test_domain());
        let line_index = LineIndex::new(source);
        check_file(std::path::Path::new("Button.ts"), &line_index, &graph, config)
    }

    #[test]
    fn reports_upward_relative_imports() {
        let source = r#"import { shared } from "../../../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_upward_relative_export_from() {
        let source = r#"export { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_dynamic_upward_imports() {
        let source = r#"const shared = await import("../../shared");
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn keeps_line_positions_after_multiline_imports() {
        let source = r#"import {
  local,
} from "./local";
import { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(4));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_same_folder_and_downward_imports() {
        let source = r#"import { value } from "./value";
export { other } from "./other";
const shared = import("shared");
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_treat_export_default_as_export_from() {
        let source = r#"export default function Component() {}
import { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn allows_configured_upward_depth() {
        let source = r#"import { shared } from "../shared";
import { other } from "../../other";
"#;
        let violations = run_check(source, &test_config_with_depth(1));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn reports_export_all_upward() {
        let source = r#"export * from "../other";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }
}
