use std::path::Path;

use crate::config::architecture::LayerBoundaryConfig;
use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{LAYER_BOUNDARIES_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import crosses a layer boundary against the defined layer order.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    config: &RuleConfig,
    layers: &LayerBoundaryConfig,
) -> Vec<Violation> {
    if !layers.is_configured() {
        return Vec::new();
    }

    let source_layer = match layers.layer_for_file(file) {
        Some(layer) => layer,
        None => return Vec::new(),
    };

    let source_index = match index_of(&layers.order, source_layer) {
        Some(index) => index,
        None => return Vec::new(),
    };

    let mut violations = Vec::new();

    for edge in import_graph.edges_from(file) {
        let target = match &edge.resolved_target {
            Some(path) => path,
            None => continue,
        };

        let target_layer = match layers.layer_for_file(target) {
            Some(layer) => layer,
            None => continue,
        };

        let target_index = match index_of(&layers.order, target_layer) {
            Some(index) => index,
            None => continue,
        };

        if target_index < source_index {
            let pos = line_index.position_for(edge.span);
            let detail = format!(
                "{} cannot import {}. Allowed direction is {} (index {}) -> {} (index {}).",
                source_layer,
                target_layer,
                target_layer,
                target_index,
                source_layer,
                source_index,
            );

            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(edge.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: LAYER_BOUNDARIES_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: Some(detail),
                subject: None,
            });
        }
    }

    violations
}

fn index_of(order: &[String], name: &str) -> Option<usize> {
    order.iter().position(|item| item == name)
}

#[cfg(test)]
mod tests {

        use anyhow::{Context, Result};
    use super::*;
    use crate::config::architecture::LayerBoundaryConfig;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::syntax::LineIndex;
    use std::collections::HashMap;

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn test_layers() -> LayerBoundaryConfig {
        let mut order = Vec::new();
        order.push("app".to_string());
        order.push("features".to_string());
        order.push("entities".to_string());
        order.push("shared".to_string());

        let mut definitions = HashMap::new();
        definitions.insert(
            "app".to_string(),
            DomainConfig {
                folders: vec!["app".to_string()],
                file_suffixes: vec![],
            },
        );
        definitions.insert(
            "features".to_string(),
            DomainConfig {
                folders: vec!["features".to_string()],
                file_suffixes: vec![],
            },
        );
        definitions.insert(
            "entities".to_string(),
            DomainConfig {
                folders: vec!["entities".to_string()],
                file_suffixes: vec![],
            },
        );
        definitions.insert(
            "shared".to_string(),
            DomainConfig {
                folders: vec!["shared".to_string()],
                file_suffixes: vec![],
            },
        );

        LayerBoundaryConfig { order, definitions }
    }

    fn test_tests_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
        }
    }

    fn run_check(
        source_file: &str,
        source: &str,
        config: &RuleConfig,
        layers: &LayerBoundaryConfig,
    ) -> Vec<Violation> {
        let base_files = vec![
            ("app/index.ts", r#"import { Feature } from "../features/index";"#),
            ("features/index.ts", r#"import { Entity } from "../entities/index";"#),
            ("entities/index.ts", r#"import { Helper } from "../shared/helper";"#),
            ("shared/helper.ts", ""),
            ("features/auth/session.ts", ""),
        ];

        let mut files_with_sources: Vec<(&str, &str)> = base_files
            .into_iter()
            .filter(|(name, _)| *name != source_file)
            .collect();
        files_with_sources.push((source_file, source));

        let graph = build_import_graph_from_sources(&files_with_sources, &test_tests_domain(), None);
        let line_index = LineIndex::new(source);
        check_file(
            std::path::Path::new(source_file),
            &line_index,
            &graph,
            config,
            layers,
        )
    }

    #[test]
    fn allows_downward_import() -> Result<()> {
        let source = r#"import { Thing } from "../entities/index";"#;
        let violations = run_check("features/index.ts", source, &test_config(), &test_layers());
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_same_layer_import() -> Result<()> {
        let source = r#"import { Other } from "./other";"#;
        let violations = run_check("features/index.ts", source, &test_config(), &test_layers());
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_upward_import() -> Result<()> {
        let source = r#"import { getSession } from "../features/auth/session";"#;
        let violations = run_check("shared/date.ts", source, &test_config(), &test_layers());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "layer-boundaries");
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
        let detail = violations[0].detail.as_deref().context("expected detail")?;
        assert!(detail.contains("shared cannot import features"));
        Ok(())
    }

    #[test]
    fn ignores_unknown_source_layer() -> Result<()> {
        let source = r#"import { Something } from "../shared/helper";"#;
        let violations = run_check("lib/unknown.ts", source, &test_config(), &test_layers());
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn ignores_unknown_target_layer() -> Result<()> {
        let source = r#"import { Something } from "../lib/unknown";"#;
        let violations = run_check("features/index.ts", source, &test_config(), &test_layers());
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn quiet_when_no_layers_configured() -> Result<()> {
        let empty_layers = LayerBoundaryConfig::default();
        let source = r#"import { Something } from "../features";"#;
        let files_with_sources = vec![
            ("shared/date.ts", source),
            ("features/index.ts", r#"import { Thing } from "../lib";"#),
        ];
        let graph = build_import_graph_from_sources(&files_with_sources, &test_tests_domain(), None);
        let line_index = LineIndex::new(source);
        let violations = check_file(
            std::path::Path::new("shared/date.ts"),
            &line_index,
            &graph,
            &test_config(),
            &empty_layers,
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_edge_span_positions() -> Result<()> {
        let source = r#"
import { getSession } from "../features/auth/session";
"#;
        let violations = run_check("shared/date.ts", source.trim(), &test_config(), &test_layers());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        Ok(())
    }

    #[test]
    fn reports_reexport_violation() -> Result<()> {
        let source = r#"export { getSession } from "../features/auth/session";"#;
        let violations = run_check("shared/date.ts", source, &test_config(), &test_layers());
        assert_eq!(violations.len(), 1);
        Ok(())
    }
}
