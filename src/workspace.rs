use anyhow::{Context, Result};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::import_graph::topology::find_strongly_connected_components;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub directory: PathBuf,
    pub public_entrypoints: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    main: Option<String>,
    module: Option<String>,
    #[serde(default)]
    exports: Option<serde_json::Value>,
    workspaces: Option<Workspaces>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Workspaces {
    Array(Vec<String>),
    Object { packages: Vec<String> },
}

#[derive(Debug, Deserialize)]
struct PnpmWorkspace {
    packages: Vec<String>,
}

impl Workspace {
    pub fn discover(workspace_root: &Path) -> Result<Workspace> {
        let mut packages = Vec::new();

        packages.extend(Self::discovery_from_package_json(workspace_root)?);

        if packages.is_empty() {
            packages.extend(Self::discovery_from_pnpm_workspace_yaml(workspace_root)?);
        }

        // Nested packages should win lookups before their parent workspaces.
        packages.sort_by(|a, b| a.directory.cmp(&b.directory));
        packages.reverse();

        Ok(Workspace {
            root: workspace_root.to_path_buf(),
            packages,
        })
    }

    fn discovery_from_package_json(workspace_root: &Path) -> Result<Vec<Package>> {
        let package_json_path = workspace_root.join("package.json");
        if !package_json_path.exists() {
            return Ok(Vec::new());
        }

        let source = std::fs::read_to_string(&package_json_path)
            .with_context(|| "failed to read package.json")?;
        let package_json: PackageJson =
            serde_json::from_str(&source).with_context(|| "failed to parse package.json")?;

        let workspaces = match package_json.workspaces {
            Some(Workspaces::Array(workspaces)) => workspaces,
            Some(Workspaces::Object { packages }) => packages,
            None => Vec::new(),
        };

        Self::collect_packages_from_globs(workspace_root, &workspaces)
    }

    fn discovery_from_pnpm_workspace_yaml(workspace_root: &Path) -> Result<Vec<Package>> {
        let pnpm_workspace_path = workspace_root.join("pnpm-workspace.yaml");
        if !pnpm_workspace_path.exists() {
            return Ok(Vec::new());
        }

        let source = std::fs::read_to_string(&pnpm_workspace_path)?;
        let pnpm_workspace: PnpmWorkspace =
            serde_yaml::from_str(&source).with_context(|| "failed to parse pnpm-workspace.yaml")?;

        Self::collect_packages_from_globs(workspace_root, &pnpm_workspace.packages)
    }

    fn collect_packages_from_globs(
        workspace_root: &Path,
        globs: &[String],
    ) -> Result<Vec<Package>> {
        let mut packages = Vec::new();
        let mut seen_dirs = HashSet::new();

        if globs.is_empty() {
            return Ok(packages);
        }

        let mut overrides = OverrideBuilder::new(workspace_root);
        for glob in globs {
            let normalized = glob
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .strip_prefix("./")
                .unwrap_or(glob.trim())
                .trim_end_matches('/');
            if normalized.is_empty() {
                continue;
            }
            overrides.add(normalized)?;
        }
        let overrides = overrides.build()?;

        let mut walker = WalkBuilder::new(workspace_root);
        walker.git_ignore(true);
        walker.hidden(false);
        walker.follow_links(false);
        walker.filter_entry(|entry| {
            if entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
            {
                let name = entry.file_name();
                if name == OsStr::new("node_modules") || name == OsStr::new(".git") {
                    return false;
                }
            }
            true
        });

        for entry in walker.build() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path == workspace_root {
                continue;
            }

            let Ok(relative) = path.strip_prefix(workspace_root) else {
                continue;
            };
            if relative.as_os_str().is_empty() {
                continue;
            }

            let matched = overrides.matched(relative, true);
            if matched.is_whitelist()
                && !seen_dirs.contains(path)
                && Self::is_valid_package_dir(path)
            {
                seen_dirs.insert(path.to_path_buf());
                if let Some(package) = Self::load_package(path) {
                    packages.push(package);
                }
            }
        }

        Ok(packages)
    }

    fn is_valid_package_dir(path: &Path) -> bool {
        path.join("package.json").exists()
    }

    fn load_package(directory: &Path) -> Option<Package> {
        let package_json_path = directory.join("package.json");
        let source = std::fs::read_to_string(&package_json_path).ok()?;
        let package_json: PackageJson = serde_json::from_str(&source).ok()?;

        let name = package_json
            .name
            .as_ref()
            .map(|name| name.to_string())
            .or_else(|| {
                directory
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })?;

        let public_entrypoints = Self::resolve_public_entrypoints(directory, &package_json);

        Some(Package {
            name,
            directory: directory.to_path_buf(),
            public_entrypoints,
        })
    }

    fn resolve_public_entrypoints(directory: &Path, package_json: &PackageJson) -> Vec<PathBuf> {
        let mut entrypoints = Vec::new();

        if let Some(exports) = &package_json.exports {
            if let Some(exports_obj) = exports.as_object() {
                for (key, value) in exports_obj {
                    if !key.starts_with('.') {
                        continue;
                    }
                    Self::collect_export_paths(directory, value, &mut entrypoints);
                }
            } else if let Some(exports_str) = exports.as_str() {
                entrypoints.push(directory.join(exports_str));
            }
        }

        if entrypoints.is_empty() {
            for entrypoint in [package_json.main.as_ref(), package_json.module.as_ref()]
                .into_iter()
                .flatten()
            {
                entrypoints.push(directory.join(entrypoint));
            }
        }

        if entrypoints.is_empty() {
            for &candidate in &["src/index.ts", "index.ts"] {
                let path = directory.join(candidate);
                if path.exists() {
                    entrypoints.push(path);
                }
            }
        }

        entrypoints
    }

    fn collect_export_paths(
        directory: &Path,
        value: &serde_json::Value,
        entrypoints: &mut Vec<PathBuf>,
    ) {
        match value {
            serde_json::Value::String(s) => {
                entrypoints.push(directory.join(s.trim_start_matches("./")));
            }
            serde_json::Value::Object(obj) => {
                for value in obj.values() {
                    Self::collect_export_paths(directory, value, entrypoints);
                }
            }
            _ => {}
        }
    }

    pub fn package_for(&self, path: &Path) -> Option<&Package> {
        self.packages
            .iter()
            .find(|package| path.starts_with(&package.directory))
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceGraph {
    pub package_graph: HashMap<String, HashSet<String>>,
}

impl WorkspaceGraph {
    pub fn build(workspace: &Workspace, import_graph: &crate::import_graph::ImportGraph) -> Self {
        let mut package_graph: HashMap<String, HashSet<String>> = HashMap::new();

        for edge in import_graph.edges() {
            let Some(ref resolved_target) = edge.resolved_target else {
                continue;
            };
            let (source, target) = match (
                workspace.package_for(&edge.source_file),
                workspace.package_for(resolved_target),
            ) {
                (Some(source), Some(target)) => (source, target),
                _ => continue,
            };

            if source.directory == target.directory {
                continue;
            }

            package_graph
                .entry(source.name.clone())
                .or_default()
                .insert(target.name.clone());
        }

        WorkspaceGraph { package_graph }
    }

    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        let package_names: Vec<String> = self.package_graph.keys().cloned().collect();
        let name_to_index: HashMap<String, u32> = package_names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), index as u32))
            .collect();

        let node_count = package_names.len();
        let mut adjacency: Vec<Vec<u32>> = vec![Vec::new(); node_count];
        for (source_name, targets) in &self.package_graph {
            let source_index = name_to_index[source_name] as usize;
            for target_name in targets {
                if let Some(&target_index) = name_to_index.get(target_name) {
                    adjacency[source_index].push(target_index);
                }
            }
        }
        for neighbors in adjacency.iter_mut() {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        let sccs = find_strongly_connected_components(&adjacency);
        let mut cycles = Vec::new();

        for scc in sccs {
            let is_cyclic = if scc.len() > 1 {
                true
            } else {
                adjacency[scc[0] as usize].contains(&scc[0])
            };

            if !is_cyclic {
                continue;
            }

            if let Some(cycle) = reconstruct_package_cycle(&scc, &adjacency, &package_names) {
                cycles.push(cycle);
            }
        }

        cycles
    }
}

fn reconstruct_package_cycle(
    scc: &[u32],
    adjacency: &[Vec<u32>],
    package_names: &[String],
) -> Option<Vec<String>> {
    let start = scc[0];
    let scc_set: HashSet<u32> = scc.iter().copied().collect();
    let mut path = vec![start];
    let mut visited: HashSet<u32> = HashSet::new();
    visited.insert(start);

    let cycle = if dfs_package_cycle(start, adjacency, &scc_set, &mut visited, &mut path) {
        path
    } else {
        vec![start, start]
    };

    Some(
        cycle
            .into_iter()
            .map(|index| package_names[index as usize].clone())
            .collect(),
    )
}

fn dfs_package_cycle(
    start: u32,
    adjacency: &[Vec<u32>],
    scc_set: &HashSet<u32>,
    visited: &mut HashSet<u32>,
    path: &mut Vec<u32>,
) -> bool {
    let mut stack: Vec<(u32, usize)> = vec![(start, 0)];

    while let Some((node, idx)) = stack.last_mut() {
        let Some(neighbors) = adjacency.get(*node as usize) else {
            path.pop();
            stack.pop();
            continue;
        };

        if *idx < neighbors.len() {
            let neighbor = neighbors[*idx];
            *idx += 1;

            if neighbor == start && path.len() > 1 {
                path.push(neighbor);
                return true;
            }

            if !scc_set.contains(&neighbor) || visited.contains(&neighbor) {
                continue;
            }

            visited.insert(neighbor);
            path.push(neighbor);
            stack.push((neighbor, 0));
        } else {
            path.pop();
            stack.pop();
        }
    }

    false
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::{Context, Result};
    use std::fs;

    fn create_package(dir: &Path, name: &str) -> Result<()> {
        let package_json = serde_json::json!({
            "name": name,
            "main": "index.js"
        });
        fs::write(
            dir.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;
        Ok(())
    }

    #[test]
    fn discovers_workspaces_from_package_json() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["packages/*"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/ui"))?;
        fs::create_dir_all(root.join("packages/shared"))?;
        create_package(&root.join("packages/ui"), "ui")?;
        create_package(&root.join("packages/shared"), "shared")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 2);
        Ok(())
    }

    #[test]
    fn discovers_workspaces_from_package_json_object() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": {
                "packages": ["packages/*", "apps/*"]
            }
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/a"))?;
        fs::create_dir_all(root.join("apps/b"))?;
        create_package(&root.join("packages/a"), "a")?;
        create_package(&root.join("apps/b"), "b")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 2);
        Ok(())
    }

    #[test]
    fn discovers_workspaces_from_pnpm_workspace() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        fs::write(
            root.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )?;

        fs::create_dir_all(root.join("packages/ui"))?;
        create_package(&root.join("packages/ui"), "ui")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 1);
        assert_eq!(workspace.packages[0].name, "ui");
        Ok(())
    }

    #[test]
    fn missing_workspaces_returns_empty() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        fs::write(root.join("package.json"), r#"{"name":"root"}"#)?;

        let workspace = Workspace::discover(root)?;
        assert!(workspace.packages.is_empty());
        Ok(())
    }

    #[test]
    fn file_inside_package_maps_to_package() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["packages/*"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/ui"))?;
        create_package(&root.join("packages/ui"), "ui")?;

        let workspace = Workspace::discover(root)?;
        let file_path = root.join("packages/ui/src/components/Button.tsx");
        let package = workspace.package_for(&file_path);

        assert!(package.is_some());
        assert_eq!(package.context("expected package")?.name, "ui");
        Ok(())
    }

    #[test]
    fn file_outside_packages_maps_to_none() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        fs::create_dir_all(root.join("src"))?;
        fs::write(
            root.join("package.json"),
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        )?;
        fs::create_dir_all(root.join("packages/ui"))?;
        create_package(&root.join("packages/ui"), "ui")?;

        let workspace = Workspace::discover(root)?;
        let file_path = root.join("src/main.ts");
        let package = workspace.package_for(&file_path);

        assert!(package.is_none());
        Ok(())
    }

    #[test]
    fn resolves_exports_entrypoints() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("pkg");
        fs::create_dir_all(&dir)?;

        let package_json = serde_json::json!({
            "name": "test-pkg",
            "exports": {
                ".": "./index.ts",
                "./internal": "./internal.ts"
            }
        });
        fs::write(
            dir.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        let package = Workspace::load_package(&dir).context("expected package to load")?;
        assert!(
            package
                .public_entrypoints
                .iter()
                .any(|p| p.ends_with("index.ts"))
        );
        assert!(
            package
                .public_entrypoints
                .iter()
                .any(|p| p.ends_with("internal.ts"))
        );
        Ok(())
    }

    #[test]
    fn falls_back_to_main_module_fields() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("pkg");
        fs::create_dir_all(&dir)?;

        let package_json = serde_json::json!({
            "name": "test-pkg",
            "main": "dist/index.js",
            "module": "dist/index.mjs"
        });
        fs::write(
            dir.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        let package = Workspace::load_package(&dir).context("expected package to load")?;
        assert!(
            package
                .public_entrypoints
                .iter()
                .any(|p| p.ends_with("index.js"))
        );
        Ok(())
    }

    #[test]
    fn workspace_graph_finds_cycles() -> Result<()> {
        let graph = WorkspaceGraph {
            package_graph: {
                let mut map = HashMap::new();
                map.insert("a".to_string(), {
                    let mut set = HashSet::new();
                    set.insert("b".to_string());
                    set
                });
                map.insert("b".to_string(), {
                    let mut set = HashSet::new();
                    set.insert("a".to_string());
                    set
                });
                map
            },
        };

        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        Ok(())
    }

    #[test]
    fn workspace_graph_allows_acyclic() -> Result<()> {
        let graph = WorkspaceGraph {
            package_graph: {
                let mut map = HashMap::new();
                map.insert("a".to_string(), {
                    let mut set = HashSet::new();
                    set.insert("b".to_string());
                    set
                });
                map.insert("b".to_string(), {
                    let mut set = HashSet::new();
                    set.insert("c".to_string());
                    set
                });
                map.insert("c".to_string(), HashSet::new());
                map
            },
        };

        let cycles = graph.find_cycles();
        assert!(cycles.is_empty());
        Ok(())
    }

    #[test]
    fn discovers_nested_workspaces() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["packages/*/*"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/group/a"))?;
        fs::create_dir_all(root.join("packages/group/b"))?;
        create_package(&root.join("packages/group/a"), "a")?;
        create_package(&root.join("packages/group/b"), "b")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 2);
        Ok(())
    }

    #[test]
    fn discovers_apps_packages_pattern() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["apps/*/packages/*"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("apps/web/packages/ui"))?;
        fs::create_dir_all(root.join("apps/admin/packages/shared"))?;
        create_package(&root.join("apps/web/packages/ui"), "ui")?;
        create_package(&root.join("apps/admin/packages/shared"), "shared")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 2);
        Ok(())
    }

    #[test]
    fn excludes_negative_workspace_patterns() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["packages/*", "!packages/excluded"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/included"))?;
        fs::create_dir_all(root.join("packages/excluded"))?;
        create_package(&root.join("packages/included"), "included")?;
        create_package(&root.join("packages/excluded"), "excluded")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 1);
        assert_eq!(workspace.packages[0].name, "included");
        Ok(())
    }

    #[test]
    fn discovers_recursive_workspace_pattern() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let package_json = serde_json::json!({
            "name": "root",
            "workspaces": ["packages/**"]
        });
        fs::write(
            root.join("package.json"),
            serde_json::to_string_pretty(&package_json)?,
        )?;

        fs::create_dir_all(root.join("packages/ui"))?;
        fs::create_dir_all(root.join("packages/shared/utils"))?;
        create_package(&root.join("packages/ui"), "ui")?;
        create_package(&root.join("packages/shared/utils"), "utils")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 2);
        Ok(())
    }

    #[test]
    fn discovers_pnpm_negative_workspace() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        fs::write(
            root.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - '!packages/excluded'\n",
        )?;

        fs::create_dir_all(root.join("packages/keep"))?;
        fs::create_dir_all(root.join("packages/excluded"))?;
        create_package(&root.join("packages/keep"), "keep")?;
        create_package(&root.join("packages/excluded"), "excluded")?;

        let workspace = Workspace::discover(root)?;
        assert_eq!(workspace.packages.len(), 1);
        assert_eq!(workspace.packages[0].name, "keep");
        Ok(())
    }
}
