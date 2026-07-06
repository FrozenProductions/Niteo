use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::defaults::{CONFIG_FILE_NAME, DEFAULT_CONFIG_SOURCE};
use super::raw::RawConfig;
use super::resolve::{ProjectConfig, resolve_project_root};

#[derive(Debug, Clone)]
pub struct ConfigSet {
    root: ResolvedConfigNode,
    children: Vec<ResolvedConfigNode>,
    lookup: ConfigTrie,
}

#[derive(Debug, Clone)]
pub struct ResolvedConfigNode {
    pub config_path: Option<PathBuf>,
    pub directory: PathBuf,
    pub config: ProjectConfig,
    pub parent_index: Option<usize>,
}

pub struct ConfigSetOptions<'a> {
    pub root_override: Option<PathBuf>,
    pub scan_scope: Option<&'a Path>,
    pub deny_child_configs: bool,
}

#[derive(Debug, Default, Clone)]
struct ConfigTrieNode {
    children: HashMap<OsString, ConfigTrieNode>,
    config_index: Option<usize>,
}

#[derive(Debug, Default, Clone)]
struct ConfigTrie {
    root: ConfigTrieNode,
}

impl ConfigTrie {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, directory: &Path, config_index: usize) {
        let mut node = &mut self.root;
        for component in directory.components() {
            node = node
                .children
                .entry(component.as_os_str().to_os_string())
                .or_default();
        }
        node.config_index = Some(config_index);
    }

    fn find(&self, file: &Path) -> Option<usize> {
        let mut node = &self.root;
        let mut best_index = None;
        for component in file.components() {
            match node.children.get(component.as_os_str()) {
                Some(next) => {
                    node = next;
                    if let Some(index) = node.config_index {
                        best_index = Some(index);
                    }
                }
                None => break,
            }
        }
        best_index
    }
}

impl ConfigSet {
    pub fn resolve(workspace: &Path, options: ConfigSetOptions) -> Result<Self> {
        let root_override = options.root_override;
        let scan_scope = options.scan_scope;
        let deny_child_configs = options.deny_child_configs;
        let root_raw = read_root_config(workspace)?;

        let project_root = resolve_project_root(
            workspace,
            root_override,
            root_raw
                .project
                .as_ref()
                .and_then(|project| project.root.as_deref()),
        );

        let root_config = raw_to_project_config(&root_raw, project_root.clone())?;

        let scan_root = scan_scope.unwrap_or(&project_root);

        let child_configs = discover_child_configs(scan_root, &project_root, workspace)?;

        if deny_child_configs && !child_configs.is_empty() {
            bail!(
                "nested niteo.toml files are not allowed with --deny-child-configs: {}",
                format_child_config_paths(&child_configs)
            );
        }

        let root_node = ResolvedConfigNode {
            config_path: workspace_config_path(workspace),
            directory: project_root.clone(),
            config: root_config,
            parent_index: None,
        };

        let mut children = Vec::new();
        let mut merged_raws: Vec<RawConfig> = Vec::new();
        for (config_path, config_dir) in child_configs {
            let source = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            let child_raw: RawConfig = toml::from_str(&source)
                .with_context(|| format!("failed to parse {}", config_path.display()))?;

            let parent_index = find_parent_node(&root_node, &children, &config_dir);
            let parent_raw = match parent_index {
                Some(0) | None => &root_raw,
                Some(child_index) => &merged_raws[child_index - 1],
            };

            let merged_raw = RawConfig::merge(parent_raw, &child_raw);
            let merged_config = raw_to_project_config(&merged_raw, project_root.clone())
                .with_context(|| {
                    format!("failed to validate rules in {}", config_path.display())
                })?;

            merged_raws.push(merged_raw.clone());

            children.push(ResolvedConfigNode {
                config_path: Some(config_path),
                directory: config_dir,
                config: merged_config,
                parent_index,
            });
        }

        Ok(ConfigSet::from_parts(root_node, children))
    }

    fn from_parts(root: ResolvedConfigNode, children: Vec<ResolvedConfigNode>) -> Self {
        let mut lookup = ConfigTrie::new();
        for (index, node) in children.iter().enumerate() {
            lookup.insert(&node.directory, index + 1);
        }
        Self {
            root,
            children,
            lookup,
        }
    }

    pub fn root(&self) -> &ProjectConfig {
        &self.root.config
    }

    pub fn config_for_file(&self, file: &Path) -> &ProjectConfig {
        self.config_with_id_for_file(file).1
    }

    /// Returns a stable id (0 for root, i+1 for `children[i]`) together with the matching config.
    /// The id can be used as a hash key to group files by config without relying on pointer identity.
    pub fn config_with_id_for_file(&self, file: &Path) -> (usize, &ProjectConfig) {
        match self.lookup.find(file) {
            Some(index) => (index, &self.children[index - 1].config),
            None => (0, &self.root.config),
        }
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

fn raw_to_project_config(raw: &RawConfig, root: PathBuf) -> Result<ProjectConfig> {
    Ok(ProjectConfig {
        root,
        gitignore: raw.gitignore(),
        history: raw
            .project
            .as_ref()
            .and_then(|project| project.history)
            .unwrap_or(true),
        structure: raw.structure(),
        architecture: raw.architecture(),
        rules: raw.rules_config().map_err(anyhow::Error::msg)?,
        fix_overrides: raw.fix.clone().unwrap_or_default(),
        fail_on: raw.fail_on_policy().map_err(anyhow::Error::msg)?,
    })
}

fn workspace_config_path(workspace: &Path) -> Option<PathBuf> {
    let path = workspace.join(CONFIG_FILE_NAME);
    if path.exists() { Some(path) } else { None }
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

fn format_child_config_paths(configs: &[(PathBuf, PathBuf)]) -> String {
    let mut paths: Vec<&Path> = configs.iter().map(|(path, _)| path.as_path()).collect();
    paths.sort();
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
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
    use crate::config::architecture::ArchitectureConfig;
    use crate::config::rules::{GitignoreConfig, Severity};
    use crate::config::structure::ProjectStructureConfig;
    use crate::rules::RulesConfig;
    use anyhow::{Context, Result};
    use std::fs;

    fn remove_dir_if_exists(path: &Path) {
        match fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove {}: {error}", path.display()),
        }
    }

    #[test]
    fn config_for_file_selects_deepest_match() -> Result<()> {
        let root_config = ProjectConfig {
            root: PathBuf::from("/project/src"),
            gitignore: GitignoreConfig::default(),
            history: true,
            structure: ProjectStructureConfig::default(),
            architecture: ArchitectureConfig::default(),
            rules: RulesConfig::default(),
            fix_overrides: std::collections::HashMap::new(),
            fail_on: crate::config::FailurePolicy::default(),
        };

        let child_config = ProjectConfig {
            root: PathBuf::from("/project/src"),
            gitignore: GitignoreConfig::default(),
            history: true,
            structure: ProjectStructureConfig::default(),
            architecture: ArchitectureConfig::default(),
            rules: RulesConfig::default(),
            fix_overrides: std::collections::HashMap::new(),
            fail_on: crate::config::FailurePolicy::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project/src"),
            config: root_config,
            parent_index: None,
        };

        let children = vec![ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project/src/admin"),
            config: child_config,
            parent_index: Some(0),
        }];

        let config_set = ConfigSet::from_parts(root, children);

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
        Ok(())
    }

    #[test]
    fn child_directories_returns_immediate_children() -> Result<()> {
        let make_config = || ProjectConfig {
            root: PathBuf::from("/project"),
            gitignore: GitignoreConfig::default(),
            history: true,
            structure: ProjectStructureConfig::default(),
            architecture: ArchitectureConfig::default(),
            rules: RulesConfig::default(),
            fix_overrides: std::collections::HashMap::new(),
            fail_on: crate::config::FailurePolicy::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project"),
            config: make_config(),
            parent_index: None,
        };

        let children = vec![
            ResolvedConfigNode {
                config_path: None,
                directory: PathBuf::from("/project/a"),
                config: make_config(),
                parent_index: Some(0),
            },
            ResolvedConfigNode {
                config_path: None,
                directory: PathBuf::from("/project/b"),
                config: make_config(),
                parent_index: Some(0),
            },
            ResolvedConfigNode {
                config_path: None,
                directory: PathBuf::from("/project/a/sub"),
                config: make_config(),
                parent_index: Some(1),
            },
        ];

        let config_set = ConfigSet::from_parts(root, children);

        let children_of_root = config_set.child_directories(0);
        assert_eq!(children_of_root.len(), 2);
        assert!(children_of_root.contains(&PathBuf::from("/project/a")));
        assert!(children_of_root.contains(&PathBuf::from("/project/b")));

        let children_of_a = config_set.child_directories(1);
        assert_eq!(children_of_a.len(), 1);
        assert!(children_of_a.contains(&PathBuf::from("/project/a/sub")));
        Ok(())
    }

    #[test]
    fn discover_child_configs_finds_nested_toml() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_discover");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("packages/admin"))?;
        fs::create_dir_all(tmp.join("packages/web"))?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;
        fs::write(
            tmp.join("packages/admin/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n",
        )?;
        fs::write(
            tmp.join("packages/web/niteo.toml"),
            "[rules.no-console]\nseverity = \"off\"\n",
        )?;

        let configs = discover_child_configs(&tmp, &tmp, &tmp)?;
        assert_eq!(configs.len(), 2);

        let dirs: Vec<&Path> = configs
            .iter()
            .map(|(_, directory)| directory.as_path())
            .collect();
        assert!(dirs.contains(&tmp.join("packages/admin").as_path()));
        assert!(dirs.contains(&tmp.join("packages/web").as_path()));

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn discover_child_configs_excludes_root_config() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_discover_root");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(&tmp)?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;

        let configs = discover_child_configs(&tmp, &tmp, &tmp)?;
        assert_eq!(configs.len(), 0);

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn config_for_file_falls_back_to_root() -> Result<()> {
        let root_config = ProjectConfig {
            root: PathBuf::from("/project"),
            gitignore: GitignoreConfig::default(),
            history: true,
            structure: ProjectStructureConfig::default(),
            architecture: ArchitectureConfig::default(),
            rules: RulesConfig::default(),
            fix_overrides: std::collections::HashMap::new(),
            fail_on: crate::config::FailurePolicy::default(),
        };

        let root = ResolvedConfigNode {
            config_path: None,
            directory: PathBuf::from("/project"),
            config: root_config,
            parent_index: None,
        };

        let config_set = ConfigSet::from_parts(root, vec![]);

        let file_outside = Path::new("/other/file.ts");
        assert_eq!(
            config_set.config_for_file(file_outside).root,
            PathBuf::from("/project")
        );
        Ok(())
    }

    #[test]
    fn resolve_default_loads_nested_config() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_resolve_default");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("src/admin"))?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;
        fs::write(
            tmp.join("src/admin/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n",
        )?;

        let config_set = ConfigSet::resolve(
            &tmp,
            ConfigSetOptions {
                root_override: None,
                scan_scope: None,
                deny_child_configs: false,
            },
        )?;
        assert_eq!(config_set.children.len(), 1);

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn resolve_strict_fails_with_nested_config() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_resolve_strict");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("src/admin"))?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;
        fs::write(
            tmp.join("src/admin/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n",
        )?;

        let error_message = ConfigSet::resolve(
            &tmp,
            ConfigSetOptions {
                root_override: None,
                scan_scope: None,
                deny_child_configs: true,
            },
        )
        .err()
        .context("expected resolve to fail")?
        .to_string();
        assert!(error_message.contains("--deny-child-configs"));
        assert!(error_message.contains("niteo.toml"));

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn resolve_strict_succeeds_with_only_root_config() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_resolve_strict_root_only");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("src"))?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;

        let config_set = ConfigSet::resolve(
            &tmp,
            ConfigSetOptions {
                root_override: None,
                scan_scope: None,
                deny_child_configs: true,
            },
        )?;
        assert_eq!(config_set.children.len(), 0);

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn resolve_strict_respects_scope() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_resolve_strict_scope");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("src/admin"))?;
        fs::create_dir_all(tmp.join("packages/other"))?;

        fs::write(tmp.join("niteo.toml"), "[project]\nroot = \"src\"\n")?;
        fs::write(
            tmp.join("packages/other/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n",
        )?;

        let config_set = ConfigSet::resolve(
            &tmp,
            ConfigSetOptions {
                root_override: None,
                scan_scope: Some(tmp.join("src").as_path()),
                deny_child_configs: true,
            },
        )?;
        assert_eq!(config_set.children.len(), 0);

        remove_dir_if_exists(&tmp);
        Ok(())
    }

    #[test]
    fn resolve_three_level_config_cascades_from_direct_parent() -> Result<()> {
        let tmp = std::env::temp_dir().join("niteo_test_resolve_three_level");
        remove_dir_if_exists(&tmp);
        fs::create_dir_all(tmp.join("src"))?;
        fs::create_dir_all(tmp.join("src/packages/admin"))?;

        fs::write(
            tmp.join("niteo.toml"),
            "[project]\nroot = \"src\"\n[rules.no-console]\nseverity = \"warn\"\n[rules.no-debugger]\nseverity = \"off\"\n",
        )?;
        fs::write(
            tmp.join("src/packages/niteo.toml"),
            "[rules.no-console]\nseverity = \"error\"\n[rules.no-debugger]\nseverity = \"warn\"\n",
        )?;
        fs::write(
            tmp.join("src/packages/admin/niteo.toml"),
            "[rules.no-console]\nseverity = \"off\"\n",
        )?;

        let config_set = ConfigSet::resolve(
            &tmp,
            ConfigSetOptions {
                root_override: None,
                scan_scope: None,
                deny_child_configs: false,
            },
        )?;
        assert_eq!(config_set.children.len(), 2);

        let admin_config = config_set.config_for_file(&tmp.join("src/packages/admin/index.ts"));
        assert_eq!(admin_config.rules.no_console.severity, Severity::Off);
        assert_eq!(
            admin_config.rules.no_debugger.severity,
            Severity::Warn,
            "inherits from direct parent, not root"
        );

        remove_dir_if_exists(&tmp);
        Ok(())
    }
}
