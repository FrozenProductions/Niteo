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

        let is_public = target_package
            .public_entrypoints
            .iter()
            .any(|entrypoint| target == entrypoint);

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
    use crate::import_graph::build_import_graph_from_sources_with_workspace;
    use crate::workspace::{ExportMap, Package, Workspace, WorkspaceResolver};
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
                    exports: ExportMap::default(),
                    public_entrypoints: vec![root.join("packages/app/src/index.ts")],
                },
                Package {
                    name: "ui".to_string(),
                    directory: root.join("packages/ui"),
                    exports: ExportMap::default(),
                    public_entrypoints: vec![root.join("packages/ui/src/index.ts")],
                },
            ],
        }
    }

    fn workspace_with_ui_exports(root: &std::path::Path, export_button: bool) -> Workspace {
        let exports = if export_button {
            serde_json::json!({
                ".": "./src/index.ts",
                "./internal/button": "./internal/button.ts"
            })
        } else {
            serde_json::json!({ ".": "./src/index.ts" })
        };
        let ui_directory = root.join("packages/ui");
        let entrypoints = if export_button {
            vec![
                ui_directory.join("src/index.ts"),
                ui_directory.join("internal/button.ts"),
            ]
        } else {
            vec![ui_directory.join("src/index.ts")]
        };
        Workspace {
            root: root.to_path_buf(),
            packages: vec![
                Package {
                    name: "app".to_string(),
                    directory: root.join("packages/app"),
                    exports: ExportMap::default(),
                    public_entrypoints: vec![root.join("packages/app/src/index.ts")],
                },
                Package {
                    name: "@scope/ui".to_string(),
                    directory: ui_directory.clone(),
                    exports: ExportMap::from_json(&ui_directory, &exports),
                    public_entrypoints: entrypoints,
                },
            ],
        }
    }

    fn graph_with_workspace(
        files: &[(&str, &str)],
        workspace: &Workspace,
    ) -> crate::import_graph::ImportGraph {
        let resolver = WorkspaceResolver::build(workspace);
        build_import_graph_from_sources_with_workspace(files, &test_domain(), None, Some(&resolver))
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

    #[test]
    fn allows_exported_subpath_import_by_package_name() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_ui_exports(&root, true);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '@scope/ui/internal/button';"),
            ("/repo/packages/ui/src/index.ts", "export const Button = 1;"),
            ("/repo/packages/ui/internal/button.ts", "export const Button = 1;"),
        ];
        let graph = graph_with_workspace(&files, &workspace);
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
    fn reports_non_exported_subpath_import_by_package_name() -> Result<()> {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_ui_exports(&root, false);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '@scope/ui/internal/button';"),
            ("/repo/packages/ui/src/index.ts", "export const Button = 1;"),
            ("/repo/packages/ui/internal/button.ts", "export const Button = 1;"),
        ];
        let graph = graph_with_workspace(&files, &workspace);
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
        assert_eq!(violations[0].subject.as_deref(), Some("@scope/ui/internal/button"));

        Ok(())}
}
