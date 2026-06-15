use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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

impl Workspace {
    pub fn discover(workspace_root: &Path) -> Result<Workspace> {
        let mut packages = Vec::new();

        packages.extend(Self::discovery_from_package_json(workspace_root)?);

        if packages.is_empty() {
            packages.extend(Self::discovery_from_pnpm_workspace_yaml(workspace_root)?);
        }

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
        let package_json: serde_json::Value =
            serde_json::from_str(&source).with_context(|| "failed to parse package.json")?;

        let workspaces =
            if let Some(workspaces) = package_json.get("workspaces").and_then(|w| w.as_array()) {
                workspaces
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else if let Some(workspaces) = package_json
                .get("workspaces")
                .and_then(|w| w.get("packages"))
                .and_then(|p| p.as_array())
            {
                workspaces
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            };

        Self::collect_packages_from_globs(workspace_root, &workspaces)
    }

    fn discovery_from_pnpm_workspace_yaml(workspace_root: &Path) -> Result<Vec<Package>> {
        let pnpm_workspace_path = workspace_root.join("pnpm-workspace.yaml");
        if !pnpm_workspace_path.exists() {
            return Ok(Vec::new());
        }

        let source = std::fs::read_to_string(&pnpm_workspace_path)?;
        let mut pnpm_packages = Vec::new();
        let mut in_packages = false;
        for line in source.lines() {
            if line.trim() == "packages:" {
                in_packages = true;
                continue;
            }
            if in_packages {
                let trimmed = line.trim();
                if trimmed.starts_with("- ") {
                    let pkg = trimmed.trim_start_matches("- ").trim().trim_matches('\'');
                    pnpm_packages.push(pkg.to_string());
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    break;
                }
            }
        }
        let pnpm_workspace = PnpmWorkspace {
            packages: pnpm_packages,
        };

        Self::collect_packages_from_globs(workspace_root, &pnpm_workspace.packages)
    }

    fn collect_packages_from_globs(
        workspace_root: &Path,
        globs: &[String],
    ) -> Result<Vec<Package>> {
        let mut packages = Vec::new();
        let mut seen_dirs = HashSet::new();

        for glob_pattern in globs {
            let is_recursive = glob_pattern.contains("**");
            let glob_parts: Vec<&str> = glob_pattern.split('/').collect();

            if is_recursive {
                let base = glob_parts
                    .iter()
                    .take_while(|p| !p.starts_with("**"))
                    .fold(PathBuf::new(), |acc, &part| acc.join(part));
                let base_path = workspace_root.join(&base);

                if let Ok(entries) = std::fs::read_dir(&base_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir()
                            && !seen_dirs.contains(&path)
                            && Self::is_valid_package_dir(&path)
                        {
                            seen_dirs.insert(path.clone());
                            if let Some(package) = Self::load_package(&path) {
                                packages.push(package);
                            }
                        }
                    }
                }
            } else if glob_parts.len() == 2 && glob_pattern.ends_with("/*") {
                let first_part = glob_parts
                    .first()
                    .context("glob pattern is missing base directory")?;
                let base = workspace_root.join(first_part);
                let prefix = format!("{}/", first_part);
                let glob_suffix = glob_pattern.get(prefix.len()..).unwrap_or("");
                if glob_suffix == "*"
                    && let Ok(entries) = std::fs::read_dir(&base)
                {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir()
                            && !seen_dirs.contains(&path)
                            && Self::is_valid_package_dir(&path)
                        {
                            seen_dirs.insert(path.clone());
                            if let Some(package) = Self::load_package(&path) {
                                packages.push(package);
                            }
                        }
                    }
                }
            } else {
                let path = workspace_root.join(glob_pattern);
                if !seen_dirs.contains(&path) && Self::is_valid_package_dir(&path) {
                    seen_dirs.insert(path.clone());
                    if let Some(package) = Self::load_package(&path) {
                        packages.push(package);
                    }
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
        let package_json: serde_json::Value = serde_json::from_str(&source).ok()?;

        let name = package_json
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or(directory.file_name()?.to_str()?)
            .to_string();

        let public_entrypoints = Self::resolve_public_entrypoints(directory, &package_json);

        Some(Package {
            name,
            directory: directory.to_path_buf(),
            public_entrypoints,
        })
    }

    fn resolve_public_entrypoints(
        directory: &Path,
        package_json: &serde_json::Value,
    ) -> Vec<PathBuf> {
        let mut entrypoints = Vec::new();

        if let Some(exports) = package_json.get("exports") {
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
            for field in ["main", "module"] {
                if let Some(entrypoint) = package_json.get(field).and_then(|v| v.as_str()) {
                    entrypoints.push(directory.join(entrypoint));
                }
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

#[derive(Debug, Deserialize)]
struct PnpmWorkspace {
    packages: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceGraph {
    pub package_graph: HashMap<String, HashSet<String>>,
}

impl WorkspaceGraph {
    pub fn build(workspace: &Workspace, import_graph: &crate::import_graph::ImportGraph) -> Self {
        let mut package_graph: HashMap<String, HashSet<String>> = HashMap::new();

        for edge in &import_graph.edges {
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
        let mut cycles = Vec::new();
        let mut visited = HashMap::<String, VisitState>::new();
        let mut stack = Vec::new();

        for package_name in self.package_graph.keys() {
            if !visited.contains_key(package_name) {
                self.dfs(package_name, &mut visited, &mut stack, &mut cycles);
            }
        }

        cycles
    }

    fn dfs(
        &self,
        current: &str,
        visited: &mut HashMap<String, VisitState>,
        stack: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(current.to_string(), VisitState::InStack);
        stack.push(current.to_string());

        if let Some(neighbors) = self.package_graph.get(current) {
            for neighbor in neighbors {
                match visited.get(neighbor) {
                    Some(VisitState::InStack) => {
                        if let Some(pos) = stack.iter().position(|s| s == neighbor) {
                            let cycle = stack.get(pos..).unwrap_or(&[]).to_vec();
                            let mut exists = false;
                            for existing_cycle in &*cycles {
                                if existing_cycle.len() == cycle.len()
                                    && existing_cycle.iter().all(|item| cycle.contains(item))
                                {
                                    exists = true;
                                    break;
                                }
                            }
                            if !exists {
                                cycles.push(cycle);
                            }
                        }
                    }
                    None => {
                        self.dfs(neighbor, visited, stack, cycles);
                    }
                    Some(VisitState::Done) => {}
                }
            }
        }

        stack.pop();
        visited.insert(current.to_string(), VisitState::Done);
    }
}

#[derive(PartialEq, Eq)]
enum VisitState {
    InStack,
    Done,
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
}
