use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use super::architecture::ArchitectureConfig;
use super::defaults::{CONFIG_FILE_NAME, DEFAULT_CONFIG_SOURCE};
use super::raw::RawConfig;
use super::rules::GitignoreConfig;
use super::structure::ProjectStructureConfig;
use crate::rules::RulesConfig;

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub gitignore: GitignoreConfig,
    pub structure: ProjectStructureConfig,
    pub architecture: ArchitectureConfig,
    pub rules: RulesConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            gitignore: GitignoreConfig::default(),
            structure: ProjectStructureConfig::default(),
            architecture: ArchitectureConfig::default(),
            rules: RulesConfig::default(),
        }
    }
}

impl ProjectConfig {
    pub fn resolve(workspace: &Path, root_override: Option<PathBuf>) -> Result<Self> {
        let raw_config = read_config_file(workspace)?;

        let root = if let Some(root) = root_override {
            absolutize(workspace, root)
        } else if let Some(root) = raw_config
            .project
            .as_ref()
            .and_then(|project| project.root.as_ref())
        {
            absolutize(workspace, root.to_path_buf())
        } else {
            let source_root = workspace.join("src");
            if source_root.is_dir() {
                source_root
            } else {
                workspace.to_path_buf()
            }
        };

        Ok(raw_config.into_project_config(root))
    }
}

pub fn write_default_config(workspace: &Path) -> Result<PathBuf> {
    let config_path = workspace.join(CONFIG_FILE_NAME);
    if config_path.exists() {
        bail!("{} already exists", config_path.display());
    }

    fs::write(&config_path, DEFAULT_CONFIG_SOURCE)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    Ok(config_path)
}

fn absolutize(workspace: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    workspace.join(path)
}

fn read_config_file(workspace: &Path) -> Result<RawConfig> {
    let config_path = workspace.join(CONFIG_FILE_NAME);
    let source = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?
    } else {
        DEFAULT_CONFIG_SOURCE.to_owned()
    };

    let config = toml::from_str(&source)
        .with_context(|| format!("failed to parse config from {}", config_path.display()))?;

    Ok(config)
}

impl RawConfig {
    fn into_project_config(self, root: PathBuf) -> ProjectConfig {
        ProjectConfig {
            root,
            gitignore: self.gitignore(),
            structure: self.structure(),
            architecture: self.architecture(),
            rules: self.rules_config(),
        }
    }
}
