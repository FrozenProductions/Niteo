use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::config::RuleConfig;
use crate::import_graph::ImportGraph;
use crate::rules::{NO_PACKAGE_CYCLE_RULE_ID, Violation};
use crate::syntax::LineIndex;
use crate::workspace::{Workspace, WorkspaceGraph};

const MESSAGE: &str = "Package dependency cycle detected.";

pub struct PackageCycleContext {
    cycles_by_package: HashMap<String, Vec<String>>,
}

impl PackageCycleContext {
    pub fn new(workspace: &Workspace, import_graph: &ImportGraph) -> Self {
        let package_graph = WorkspaceGraph::build(workspace, import_graph);
        let cycles = package_graph.find_cycles();

        let mut cycles_by_package = HashMap::new();

        for cycle in cycles {
            for package_name in &cycle {
                cycles_by_package
                    .entry(package_name.clone())
                    .or_insert_with(|| cycle.clone());
            }
        }

        Self { cycles_by_package }
    }
}

pub fn check_file(
    file: &Path,
    line_index: &LineIndex,
    import_graph: &ImportGraph,
    workspace: Option<&Workspace>,
    context: &PackageCycleContext,
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

    let Some(cycle) = context.cycles_by_package.get(&source_package.name) else {
        return Vec::new();
    };

    let cycle_display = format_cycle(cycle);
    let mut violations = Vec::new();
    let mut seen_targets = HashSet::new();

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

        if cycle.contains(&target_package.name) && seen_targets.insert(target_package.name.clone())
        {
            let pos = line_index.position_for(edge.span);
            violations.push(Violation {
                file: file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_PACKAGE_CYCLE_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: Some(cycle_display.clone()),
                subject: Some(edge.specifier.clone()),
            });
        }
    }

    violations
}

fn format_cycle(cycle: &[String]) -> String {
    let parts: Vec<String> = cycle.to_vec();
    if parts.is_empty() {
        return String::new();
    }
    let mut display = parts.clone();
    display.push(parts[0].clone());
    display.join(" -> ")
}

#[cfg(test)]
mod tests {
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

    fn workspace_with_cycle(root: &std::path::Path) -> Workspace {
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

    fn run_check(
        file_path: &str,
        files: &[(&str, &str)],
        workspace: &Workspace,
    ) -> Vec<Violation> {
        let graph = build_import_graph_from_sources(files, &test_domain(), None);
        let context = PackageCycleContext::new(workspace, &graph);
        let source = files
            .iter()
            .find(|(path, _)| *path == file_path)
            .map(|(_, source)| *source)
            .unwrap_or("");
        let line_index = LineIndex::new(source);
        check_file(
            Path::new(file_path),
            &line_index,
            &graph,
            Some(workspace),
            &context,
            &test_config(),
        )
    }

    #[test]
    fn detects_direct_package_cycle() {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_cycle(&root);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/index';"),
            ("/repo/packages/ui/src/index.ts", "import { App } from '../../app/src/index';"),
            ("/repo/packages/app/src/index.ts", "export const App = 1;"),
        ];
        let violations = run_check("/repo/packages/app/src/main.ts", &files, &workspace);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, NO_PACKAGE_CYCLE_RULE_ID);
        assert!(violations[0].detail.as_ref().unwrap().contains("app"));
        assert!(violations[0].detail.as_ref().unwrap().contains("ui"));
    }

    #[test]
    fn no_violation_in_acyclic_graph() {
        let root = PathBuf::from("/repo");
        let workspace = workspace_with_cycle(&root);
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/index';"),
            ("/repo/packages/ui/src/index.ts", "export const Button = 1;"),
        ];
        let violations = run_check("/repo/packages/app/src/main.ts", &files, &workspace);
        assert!(violations.is_empty());
    }

    #[test]
    fn no_violations_without_workspace() {
        let files = vec![
            ("/repo/packages/app/src/main.ts", "import { Button } from '../../ui/src/index';"),
            ("/repo/packages/ui/src/index.ts", "import { App } from '../../app/src/index';"),
        ];
        let graph = build_import_graph_from_sources(&files, &test_domain(), None);
        let context = PackageCycleContext::new(
            &Workspace {
                root: PathBuf::from("/repo"),
                packages: vec![],
            },
            &graph,
        );
        let line_index = LineIndex::new(files[0].1);
        let violations = check_file(
            Path::new("/repo/packages/app/src/main.ts"),
            &line_index,
            &graph,
            None,
            &context,
            &test_config(),
        );
        assert!(violations.is_empty());
    }
}
