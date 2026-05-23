use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "niteo.toml";
const DEFAULT_CONFIG_SOURCE: &str = r#"[project]
root = "src"

[rules]
[rules.no-comments]
severity = "warn"
allow-doc-comments = true
"#;

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub no_comments: CommentsRuleConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            no_comments: CommentsRuleConfig::default(),
        }
    }
}

impl ProjectConfig {
    pub fn resolve(workspace: &Path, root_override: Option<PathBuf>) -> Result<Self> {
        let config = read_config_file(workspace)?;

        if let Some(root) = root_override {
            return Ok(Self {
                root: absolutize(workspace, root),
                no_comments: config.no_comments(),
            });
        }

        if let Some(root) = config
            .project
            .as_ref()
            .and_then(|project| project.root.as_ref())
        {
            return Ok(Self {
                root: absolutize(workspace, root.to_path_buf()),
                no_comments: config.no_comments(),
            });
        }

        let source_root = workspace.join("src");
        if source_root.is_dir() {
            return Ok(Self {
                root: source_root,
                no_comments: config.no_comments(),
            });
        }

        Ok(Self {
            root: workspace.to_path_buf(),
            no_comments: config.no_comments(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Off,
    Warn,
    Error,
}

impl Severity {
    pub fn from_str(value: &str) -> Self {
        match value {
            "off" => Self::Off,
            "error" => Self::Error,
            _ => Self::Warn,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warning",
            Self::Error => "error",
        }
    }

    pub fn is_enabled(self) -> bool {
        self != Self::Off
    }
}

#[derive(Debug, Clone)]
pub struct CommentsRuleConfig {
    pub severity: Severity,
    pub allow_doc_comments: bool,
}

impl Default for CommentsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_doc_comments: true,
        }
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
    if !config_path.exists() {
        return Ok(RawConfig::default());
    }

    let source = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let config = toml::from_str(&source)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    Ok(config)
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    project: Option<RawProjectConfig>,
    rules: Option<HashMap<String, RawRuleConfig>>,
}

impl RawConfig {
    fn no_comments(&self) -> CommentsRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-comments"))
            .map(RawRuleConfig::to_comments_config)
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
struct RawProjectConfig {
    root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawRuleConfig {
    Severity(String),
    Options(RawRuleOptions),
}

impl RawRuleConfig {
    fn to_comments_config(&self) -> CommentsRuleConfig {
        match self {
            Self::Severity(severity) => CommentsRuleConfig {
                severity: Severity::from_str(severity),
                allow_doc_comments: true,
            },
            Self::Options(options) => CommentsRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                allow_doc_comments: options.allow_doc_comments.unwrap_or(true),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRuleOptions {
    severity: Option<String>,
    #[serde(rename = "allow-doc-comments")]
    allow_doc_comments: Option<bool>,
}
