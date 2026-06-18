use std::path::Path;

use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_PRIVATE_PACKAGE_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;
use crate::workspace::Workspace;

const MESSAGE: &str = "Import from another package's internal file is not allowed.";

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    workspace: Option<&Workspace>,
    config: &RuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let Some(workspace) = workspace else {
        return Vec::new();
    };

    let Some(source_package) = workspace.package_for(file) else {
        return Vec::new();
    };

    let mut violations = Vec::new();

    for edge in import_graph.edges_from(file) {
        let Some(ref target) = edge.resolved_target else {
            continue;
        };

        let Some(target_package) = workspace.package_for(target) else {
            continue;
        };

        if source_package.directory == target_package.directory {
            continue;
        }

        let is_public = target_package.public_entrypoints.iter().any(|entrypoint| {
            target == entrypoint
                || target.parent() == Some(entrypoint.as_path())
                || target.starts_with(entrypoint.parent().unwrap_or(entrypoint.as_path()))
                    && entrypoint.file_name() == target.file_name()
        });

        if !is_public {
            let pos = line_index.position_for(edge.span);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(edge.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_PRIVATE_PACKAGE_IMPORT_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: Some(format!(
                    "imports from `{}` package internal",
                    target_package.name
                )),
                subject: Some(edge.specifier.clone()),
            });
        }
    }

    violations
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::import_graph::build_import_graph_from_sources;
    use crate::workspace::{Package, Workspace};
    use std::path::PathBuf;

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn test_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string()],
        }
    }

    fn workspace_with_packages(root: &std::path::Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            packages: vec![
                Package {
                    name: "app".to_string(),
                    directory: root.join("packages/app"),
                    public_entrypoints: vec![root.join("packages/app/src/index.ts")],
                },
                Package {
                    name: "ui".to_string(),
                    directory: root.join("packages/ui"),
                    public_entrypoints: vec![root.join("packages/ui/src/index.ts")],
                },
            ],
        }
    }

    #[test]
    fn allows_public_import_from_another_package() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_packages(&root);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/index';"),
            ("/repo/packages/ui/src/index.ts", "export const Button = 1;"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/packages/app/src/main.ts"),
            &line_index,
            &graph,
            Some(&workspace),
            &test_config(),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_private_import_by_relative_path() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_packages(&root);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/internal/Button';"),
            ("/repo/packages/ui/src/internal/Button.ts", "export const Button = 1;"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/packages/app/src/main.ts"),
            &line_index,
            &graph,
            Some(&workspace),
            &test_config(),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, NO_PRIVATE_PACKAGE_IMPORT_RULE_ID);
    
        Ok(())}

    #[test]
    fn allows_same_package_internal_import() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_packages(&root);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { helper } from './internal/helper';"),
            ("/repo/packages/app/src/internal/helper.ts", "export const helper = 1;"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/packages/app/src/main.ts"),
            &line_index,
            &graph,
            Some(&workspace),
            &test_config(),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn no_violations_without_workspace() -> Result<()> {
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/internal/Button';"),
            ("/repo/packages/ui/src/internal/Button.ts", "export const Button = 1;"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/packages/app/src/main.ts"),
            &line_index,
            &graph,
            None,
            &test_config(),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn no_violations_when_source_outside_packages() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_packages(&root);
        let files = vec![
            ("/repo/src/main.ts", "import { Button } from '../packages/ui/src/internal/Button';"),
            ("/repo/packages/ui/src/internal/Button.ts", "export const Button = 1;"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/src/main.ts"),
            &line_index,
            &graph,
            Some(&workspace),
            &test_config(),
        );
        assert!(violations.is_empty());
    
        Ok(())}
}
