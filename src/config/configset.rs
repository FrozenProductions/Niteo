use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::defaults::{CONFIG_FILE_NAME, DEFAULT_CONFIG_SOURCE};
use super::raw::RawConfig;
use super::resolve::ProjectConfig;

#[derive(Debug)]
pub struct ConfigSet {
    root: ResolvedConfigNode,
    children: Vec<ResolvedConfigNode>,
}

#[derive(Debug)]
pub struct ResolvedConfigNode {
    #[allow(dead_code)]
    pub config_path: Option<PathBuf>,
    pub directory: PathBuf,
    pub config: ProjectConfig,
    pub parent_index: Option<usize>,
}

impl ConfigSet {
    pub fn resolve(
        workspace: &Path,
        root_override: Option<PathBuf>,
        scan_scope: Option<&Path>,
    ) -> Result<Self> {
        let root_raw = read_root_config(workspace)?;

        let project_root = if let Some(root) = root_override {
            absolutize(workspace, root)
        } else if let Some(root) = root_raw
            .project
            .as_ref()
            .and_then(|project| project.root.as_ref())
        {
            absolutize(workspace, root.clone())
        } else {
            let source_root = workspace.join("src");
            if source_root.is_dir() {
                source_root
            } else {
                workspace.to_path_buf()
            }
        };

        let root_config = raw_to_project_config(&root_raw, project_root.clone());

        let scan_root = scan_scope.unwrap_or(&project_root);

        let child_configs = discover_child_configs(scan_root, &project_root, workspace)?;

        let root_node = ResolvedConfigNode {
            config_path: workspace_config_path(workspace),
            directory: project_root.clone(),
            config: root_config,
            parent_index: None,
        };

        let mut children = Vec::new();
        for (config_path, config_dir) in child_configs {
            let source = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            let child_raw: RawConfig = toml::from_str(&source)
                .with_context(|| format!("failed to parse {}", config_path.display()))?;

            let merged_raw = RawConfig::merge(&root_raw, &child_raw);
            let merged_config = raw_to_project_config(&merged_raw, project_root.clone());

            let parent_index = find_parent_node(&root_node, &children, &config_dir);

            children.push(ResolvedConfigNode {
                config_path: Some(config_path),
                directory: config_dir,
                config: merged_config,
                parent_index,
            });
        }

        Ok(ConfigSet {
            root: root_node,
            children,
        })
    }

    pub fn root(&self) -> &ProjectConfig {
        &self.root.config
    }

    pub fn config_for_file(&self, file: &Path) -> &ProjectConfig {
        let mut best_match = &self.root;
        let mut best_depth = if file.starts_with(&self.root.directory) {
            self.root.directory.components().count()
        } else {
            0
        };

        for node in &self.children {
            if file.starts_with(&node.directory) {
                let depth = node.directory.components().count();
                if depth > best_depth {
                    best_depth = depth;
                    best_match = node;
                }
            }
        }

        &best_match.config
    }

    pub fn configs(&self) -> impl Iterator<Item = &ResolvedConfigNode> {
        std::iter::once(&self.root).chain(self.children.iter())
    }

    pub fn child_directories(&self, parent_index: usize) -> Vec<PathBuf> {
        self.children
            .iter()
            .filter(|node| node.parent_index == Some(parent_index))
            .map(|node| node.directory.clone())
            .collect()
    }
}

fn read_root_config(workspace: &Path) -> Result<RawConfig> {
    let config_path = workspace.join(CONFIG_FILE_NAME);
    let source = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?
    } else {
        DEFAULT_CONFIG_SOURCE.to_owned()
    };

    toml::from_str(&source)
        .with_context(|| format!("failed to parse config from {}", config_path.display()))
}

fn raw_to_project_config(raw: &RawConfig, root: PathBuf) -> ProjectConfig {
    ProjectConfig {
        root,
        gitignore: raw.gitignore(),
        structure: raw.structure(),
        rules: raw.rules_config(),
    }
}

fn workspace_config_path(workspace: &Path) -> Option<PathBuf> {
    let path = workspace.join(CONFIG_FILE_NAME);
    if path.exists() { Some(path) } else { None }
}

fn absolutize(workspace: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    workspace.join(path)
}

fn discover_child_configs(
    scan_root: &Path,
    project_root: &Path,
    _workspace: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    if !scan_root.exists() {
        return Ok(Vec::new());
    }

    let mut configs = Vec::new();
    let root_config_at_project_root = project_root.join(CONFIG_FILE_NAME);

    for entry in ignore::WalkBuilder::new(scan_root)
        .git_ignore(true)
        .hidden(false)
        .follow_links(false)
        .build()
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if path.file_name().and_then(|name| name.to_str()) != Some(CONFIG_FILE_NAME) {
            continue;
        }

        if path == root_config_at_project_root {
            continue;
        }

        let config_dir = path.parent().unwrap_or(scan_root).to_path_buf();
        configs.push((path.to_path_buf(), config_dir));
    }

    configs.sort_by_key(|(_, dir)| dir.components().count());

    Ok(configs)
}

fn find_parent_node(
    root: &ResolvedConfigNode,
    children: &[ResolvedConfigNode],
    dir: &Path,
) -> Option<usize> {
    let mut best_index = None;
    let mut best_depth = 0;

    if dir.starts_with(&root.directory) {
        best_depth = root.directory.components().count();
        best_index = Some(0);
    }

    for (i, node) in children.iter().enumerate() {
        let child_index = i + 1;
        if dir.starts_with(&node.directory) {
            let depth = node.directory.components().count();
            if depth > best_depth {
                best_depth = depth;
                best_index = Some(child_index);
            }
        }
    }

    best_index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::rules::GitignoreConfig;
    use crate::config::structure::ProjectStructureConfig;
    use crate::rules::RulesConfig;
    use std::fs;

    fn remove_dir_if_exists(path: &Path) {
        match fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove {}: {error}", path.display()),
        }
    }

    #[test]
    fn config_for_file_selects_deepest_match() {
        let root_config = ProjectConfig {
            root: PathBuf::from("/project/src"),
            gitignore: GitignoreConfig::default(),
            structure: ProjectStructureConfig::default(),
            rules: RulesConfig::default(),
        };

        let child_config = ProjectConfig {
            root: PathBuf::from("/project/src"),
            gitignore: GitignoreConfig::default(),
            structure: ProjectStructureConfig::default(),
            rules: RulesConfig::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project/src"),
            config: root_config,
            parent_index: None,
        };

        let children = vec![ResolvedConfigNode {
            config_path: Some(PathBuf::from("/project/src/admin/niteo.toml")),
            directory: PathBuf::from("/project/src/admin"),
            config: child_config,
            parent_index: Some(0),
        }];

        let config_set = ConfigSet { root, children };

        let file_in_admin = Path::new("/project/src/admin/page.ts");
        let file_in_src = Path::new("/project/src/utils/format.ts");

        assert_eq!(
            config_set.config_for_file(file_in_admin).root,
            PathBuf::from("/project/src")
        );
        assert_eq!(
            config_set.config_for_file(file_in_src).root,
            PathBuf::from("/project/src")
        );
    }

    #[test]
    fn child_directories_returns_immediate_children() {
        let make_config = || ProjectConfig {
            root: PathBuf::from("/project"),
            gitignore: GitignoreConfig::default(),
            structure: ProjectStructureConfig::default(),
            rules: RulesConfig::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project"),
            config: make_config(),
            parent_index: None,
        };

        let children = vec![
            ResolvedConfigNode {
                config_path: Some(PathBuf::from("/project/a/niteo.toml")),
                directory: PathBuf::from("/project/a"),
                config: make_config(),
                parent_index: Some(0),
            },
            ResolvedConfigNode {
                config_path: Some(PathBuf::from("/project/b/niteo.toml")),
                directory: PathBuf::from("/project/b"),
                config: make_config(),
                parent_index: Some(0),
            },
            ResolvedConfigNode {
                config_path: Some(PathBuf::from("/project/a/sub/niteo.toml")),
                directory: PathBuf::from("/project/a/sub"),
                config: make_config(),
                parent_index: Some(1),
            },
        ];

        let config_set = ConfigSet { root, children };

        let children_of_root = config_set.child_directories(0);
        assert_eq!(children_of_root.len(), 2);
        assert!(children_of_root.contains(&PathBuf::from("/project/a")));
        assert!(children_of_root.contains(&PathBuf::from("/project/b")));

        let children_of_a = config_set.child_directories(1);
        assert_eq!(children_of_a.len(), 1);
        assert!(children_of_a.contains(&PathBuf::from("/project/a/sub")));
    }

    #[test]
    fn discover_child_configs_finds_nested_toml() {
        let tmp = std::env::temp_dir().join("niteo_test_discover");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("packages/admin")).unwrap();
        fs::create_dir_all(tmp.join("packages/web")).unwrap();

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n").unwrap();
        fs::write(
            tmp.join("packages/admin/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n",
        )
        .unwrap();
        fs::write(
            tmp.join("packages/web/niteo.toml"),
            "[rules.no-console]\nseverity = \"off\"\n",
        )
        .unwrap();

        let configs = discover_child_configs(&tmp, &tmp, &tmp).unwrap();
        assert_eq!(configs.len(), 2);

        let dirs: Vec<&Path> = configs
            .iter()
            .map(|(_, directory)| directory.as_path())
            .collect();
        assert!(dirs.contains(&tmp.join("packages/admin").as_path()));
        assert!(dirs.contains(&tmp.join("packages/web").as_path()));

        remove_dir_if_exists(&tmp);
    }

    #[test]
    fn discover_child_configs_excludes_root_config() {
        let tmp = std::env::temp_dir().join("niteo_test_discover_root");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n").unwrap();

        let configs = discover_child_configs(&tmp, &tmp, &tmp).unwrap();
        assert_eq!(configs.len(), 0);

        remove_dir_if_exists(&tmp);
    }

    #[test]
    fn config_for_file_falls_back_to_root() {
        let root_config = ProjectConfig {
            root: PathBuf::from("/project"),
            gitignore: GitignoreConfig::default(),
            structure: ProjectStructureConfig::default(),
            rules: RulesConfig::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project"),
            config: root_config,
            parent_index: None,
        };

        let config_set = ConfigSet {
            root,
            children: vec![],
        };

        let file_outside = Path::new("/other/file.ts");
        assert_eq!(
            config_set.config_for_file(file_outside).root,
            PathBuf::from("/project")
        );
    }
}
