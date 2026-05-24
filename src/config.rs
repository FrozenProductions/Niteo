use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "niteo.toml";
const DEFAULT_CONFIG_SOURCE: &str = r#"[project]
root = "src"
respect-gitignore = true

[rules]
[rules.no-comments]
severity = "warn"
allow-doc-comments = true

[rules.no-logic-in-barrel]
severity = "warn"

[rules.no-default-export]
severity = "warn"

[rules.no-inline-types]
severity = "warn"

[rules.max-file-exports]
severity = "warn"
max-exports = 10

[rules.no-upward-import]
severity = "warn"
max-depth = 0

[rules.no-large-file]
severity = "warn"
max-lines = 500

[rules.no-enums]
severity = "warn"

[rules.no-barrel-files]
severity = "warn"

[rules.no-console]
severity = "warn"
allow-patterns = []

[rules.no-debugger]
severity = "warn"

[rules.no-eval]
severity = "warn"

[rules.no-logic-in-domain]
severity = "warn"
extra-folders = []
extra-file-suffixes = []

[rules.no-empty-directories]
severity = "warn"
ignore-dirs = []

[rules.no-duplicate-file-names]
severity = "warn"
ignore-names = []

[rules.max-files-per-directory]
severity = "warn"
max-files = 20
ignore-dirs = []
"#;

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub root: PathBuf,
    pub no_comments: CommentsRuleConfig,
    pub no_logic_in_barrel: RuleConfig,
    pub no_default_export: RuleConfig,
    pub no_inline_types: RuleConfig,
    pub max_file_exports: FileExportsRuleConfig,
    pub no_upward_import: UpwardImportRuleConfig,
    pub no_large_file: FileLengthRuleConfig,
    pub no_enums: RuleConfig,
    pub no_barrel_files: RuleConfig,
    pub no_console: NoConsoleRuleConfig,
    pub no_debugger: RuleConfig,
    pub no_eval: RuleConfig,
    pub no_logic_in_domain: NoLogicInDomainRuleConfig,
    pub no_empty_directories: NoEmptyDirectoriesRuleConfig,
    pub no_duplicate_file_names: NoDuplicateFileNamesRuleConfig,
    pub max_files_per_directory: MaxFilesPerDirectoryRuleConfig,
    pub gitignore: GitignoreConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            no_comments: CommentsRuleConfig::default(),
            no_logic_in_barrel: RuleConfig::default(),
            no_default_export: RuleConfig::default(),
            no_inline_types: RuleConfig::default(),
            max_file_exports: FileExportsRuleConfig::default(),
            no_upward_import: UpwardImportRuleConfig::default(),
            no_large_file: FileLengthRuleConfig::default(),
            no_enums: RuleConfig::default(),
            no_barrel_files: RuleConfig::default(),
            no_console: NoConsoleRuleConfig::default(),
            no_debugger: RuleConfig::default(),
            no_eval: RuleConfig::default(),
            no_logic_in_domain: NoLogicInDomainRuleConfig::default(),
            no_empty_directories: NoEmptyDirectoriesRuleConfig::default(),
            no_duplicate_file_names: NoDuplicateFileNamesRuleConfig::default(),
            max_files_per_directory: MaxFilesPerDirectoryRuleConfig::default(),
            gitignore: GitignoreConfig::default(),
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
                no_logic_in_barrel: config.no_logic_in_barrel(),
                no_default_export: config.no_default_export(),
                no_inline_types: config.no_inline_types(),
                max_file_exports: config.max_file_exports(),
                no_upward_import: config.no_upward_import(),
                no_large_file: config.no_large_file(),
                no_enums: config.no_enums(),
                no_barrel_files: config.no_barrel_files(),
                no_console: config.no_console(),
                no_debugger: config.no_debugger(),
                no_eval: config.no_eval(),
                no_logic_in_domain: config.no_logic_in_domain(),
                no_empty_directories: config.no_empty_directories(),
                no_duplicate_file_names: config.no_duplicate_file_names(),
                max_files_per_directory: config.max_files_per_directory(),
                gitignore: config.gitignore(),
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
                no_logic_in_barrel: config.no_logic_in_barrel(),
                no_default_export: config.no_default_export(),
                no_inline_types: config.no_inline_types(),
                max_file_exports: config.max_file_exports(),
                no_upward_import: config.no_upward_import(),
                no_large_file: config.no_large_file(),
                no_enums: config.no_enums(),
                no_barrel_files: config.no_barrel_files(),
                no_console: config.no_console(),
                no_debugger: config.no_debugger(),
                no_eval: config.no_eval(),
                no_logic_in_domain: config.no_logic_in_domain(),
                no_empty_directories: config.no_empty_directories(),
                no_duplicate_file_names: config.no_duplicate_file_names(),
                max_files_per_directory: config.max_files_per_directory(),
                gitignore: config.gitignore(),
            });
        }

        let source_root = workspace.join("src");
        if source_root.is_dir() {
            return Ok(Self {
                root: source_root,
                no_comments: config.no_comments(),
                no_logic_in_barrel: config.no_logic_in_barrel(),
                no_default_export: config.no_default_export(),
                no_inline_types: config.no_inline_types(),
                max_file_exports: config.max_file_exports(),
                no_upward_import: config.no_upward_import(),
                no_large_file: config.no_large_file(),
                no_enums: config.no_enums(),
                no_barrel_files: config.no_barrel_files(),
                no_console: config.no_console(),
                no_debugger: config.no_debugger(),
                no_eval: config.no_eval(),
                no_logic_in_domain: config.no_logic_in_domain(),
                no_empty_directories: config.no_empty_directories(),
                no_duplicate_file_names: config.no_duplicate_file_names(),
                max_files_per_directory: config.max_files_per_directory(),
                gitignore: config.gitignore(),
            });
        }

        Ok(Self {
            root: workspace.to_path_buf(),
            no_comments: config.no_comments(),
            no_logic_in_barrel: config.no_logic_in_barrel(),
            no_default_export: config.no_default_export(),
            no_inline_types: config.no_inline_types(),
            max_file_exports: config.max_file_exports(),
            no_upward_import: config.no_upward_import(),
            no_large_file: config.no_large_file(),
            no_enums: config.no_enums(),
            no_barrel_files: config.no_barrel_files(),
            no_console: config.no_console(),
            no_debugger: config.no_debugger(),
            no_eval: config.no_eval(),
            no_logic_in_domain: config.no_logic_in_domain(),
            no_empty_directories: config.no_empty_directories(),
            no_duplicate_file_names: config.no_duplicate_file_names(),
            max_files_per_directory: config.max_files_per_directory(),
            gitignore: config.gitignore(),
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

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub severity: Severity,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileLengthRuleConfig {
    pub severity: Severity,
    pub max_lines: usize,
}

impl Default for FileLengthRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_lines: 300,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileExportsRuleConfig {
    pub severity: Severity,
    pub max_exports: usize,
}

impl Default for FileExportsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_exports: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpwardImportRuleConfig {
    pub severity: Severity,
    pub max_depth: usize,
}

impl Default for UpwardImportRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_depth: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitignoreConfig {
    pub enabled: bool,
}

impl Default for GitignoreConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone)]
pub struct NoEmptyDirectoriesRuleConfig {
    pub severity: Severity,
    pub ignore_dirs: Vec<String>,
}

impl Default for NoEmptyDirectoriesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoDuplicateFileNamesRuleConfig {
    pub severity: Severity,
    pub ignore_names: Vec<String>,
}

impl Default for NoDuplicateFileNamesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            ignore_names: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxFilesPerDirectoryRuleConfig {
    pub severity: Severity,
    pub max_files: usize,
    pub ignore_dirs: Vec<String>,
}

impl Default for MaxFilesPerDirectoryRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_files: 20,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoLogicInDomainRuleConfig {
    pub severity: Severity,
    pub extra_folders: Vec<String>,
    pub extra_file_suffixes: Vec<String>,
}

impl Default for NoLogicInDomainRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoConsoleRuleConfig {
    pub severity: Severity,
    pub allow_patterns: Vec<String>,
}

impl Default for NoConsoleRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_patterns: vec![],
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

    fn no_logic_in_barrel(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-logic-in-barrel"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_default_export(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-default-export"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_inline_types(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-inline-types"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn max_file_exports(&self) -> FileExportsRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("max-file-exports"))
            .map(RawRuleConfig::to_file_exports_config)
            .unwrap_or_default()
    }

    fn no_upward_import(&self) -> UpwardImportRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-upward-import"))
            .map(RawRuleConfig::to_upward_import_config)
            .unwrap_or_default()
    }

    fn no_large_file(&self) -> FileLengthRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-large-file"))
            .map(RawRuleConfig::to_file_length_config)
            .unwrap_or_default()
    }

    fn no_enums(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-enums"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_barrel_files(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-barrel-files"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_console(&self) -> NoConsoleRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-console"))
            .map(RawRuleConfig::to_no_console_config)
            .unwrap_or_default()
    }

    fn no_debugger(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-debugger"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_eval(&self) -> RuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-eval"))
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_logic_in_domain(&self) -> NoLogicInDomainRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-logic-in-domain"))
            .map(RawRuleConfig::to_no_logic_in_domain_config)
            .unwrap_or_default()
    }

    fn no_empty_directories(&self) -> NoEmptyDirectoriesRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-empty-directories"))
            .map(RawRuleConfig::to_no_empty_directories_config)
            .unwrap_or_default()
    }

    fn no_duplicate_file_names(&self) -> NoDuplicateFileNamesRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("no-duplicate-file-names"))
            .map(RawRuleConfig::to_no_duplicate_file_names_config)
            .unwrap_or_default()
    }

    fn max_files_per_directory(&self) -> MaxFilesPerDirectoryRuleConfig {
        self.rules
            .as_ref()
            .and_then(|rules| rules.get("max-files-per-directory"))
            .map(RawRuleConfig::to_max_files_per_directory_config)
            .unwrap_or_default()
    }

    fn gitignore(&self) -> GitignoreConfig {
        let project = self.project.as_ref();
        GitignoreConfig {
            enabled: project
                .and_then(|p| p.respect_gitignore)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawProjectConfig {
    root: Option<PathBuf>,
    #[serde(rename = "respect-gitignore")]
    respect_gitignore: Option<bool>,
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

    fn to_rule_config(&self) -> RuleConfig {
        match self {
            Self::Severity(severity) => RuleConfig {
                severity: Severity::from_str(severity),
            },
            Self::Options(options) => RuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
            },
        }
    }

    fn to_file_length_config(&self) -> FileLengthRuleConfig {
        match self {
            Self::Severity(severity) => FileLengthRuleConfig {
                severity: Severity::from_str(severity),
                max_lines: 300,
            },
            Self::Options(options) => FileLengthRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_lines: options.max_lines.unwrap_or(300),
            },
        }
    }

    fn to_file_exports_config(&self) -> FileExportsRuleConfig {
        match self {
            Self::Severity(severity) => FileExportsRuleConfig {
                severity: Severity::from_str(severity),
                max_exports: 10,
            },
            Self::Options(options) => FileExportsRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_exports: options.max_exports.unwrap_or(10),
            },
        }
    }

    fn to_upward_import_config(&self) -> UpwardImportRuleConfig {
        match self {
            Self::Severity(severity) => UpwardImportRuleConfig {
                severity: Severity::from_str(severity),
                max_depth: 0,
            },
            Self::Options(options) => UpwardImportRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_depth: options.max_depth.unwrap_or(0),
            },
        }
    }

    fn to_no_empty_directories_config(&self) -> NoEmptyDirectoriesRuleConfig {
        match self {
            Self::Severity(severity) => NoEmptyDirectoriesRuleConfig {
                severity: Severity::from_str(severity),
                ignore_dirs: vec![],
            },
            Self::Options(options) => NoEmptyDirectoriesRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                ignore_dirs: options.ignore_dirs.clone().unwrap_or_default(),
            },
        }
    }

    fn to_no_logic_in_domain_config(&self) -> NoLogicInDomainRuleConfig {
        match self {
            Self::Severity(severity) => NoLogicInDomainRuleConfig {
                severity: Severity::from_str(severity),
                extra_folders: vec![],
                extra_file_suffixes: vec![],
            },
            Self::Options(options) => NoLogicInDomainRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                extra_folders: options.extra_folders.clone().unwrap_or_default(),
                extra_file_suffixes: options.extra_file_suffixes.clone().unwrap_or_default(),
            },
        }
    }

    fn to_no_console_config(&self) -> NoConsoleRuleConfig {
        match self {
            Self::Severity(severity) => NoConsoleRuleConfig {
                severity: Severity::from_str(severity),
                allow_patterns: vec![],
            },
            Self::Options(options) => NoConsoleRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                allow_patterns: options.allow_patterns.clone().unwrap_or_default(),
            },
        }
    }

    fn to_no_duplicate_file_names_config(&self) -> NoDuplicateFileNamesRuleConfig {
        match self {
            Self::Severity(severity) => NoDuplicateFileNamesRuleConfig {
                severity: Severity::from_str(severity),
                ignore_names: vec![],
            },
            Self::Options(options) => NoDuplicateFileNamesRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                ignore_names: options.ignore_names.clone().unwrap_or_default(),
            },
        }
    }

    fn to_max_files_per_directory_config(&self) -> MaxFilesPerDirectoryRuleConfig {
        match self {
            Self::Severity(severity) => MaxFilesPerDirectoryRuleConfig {
                severity: Severity::from_str(severity),
                max_files: 20,
                ignore_dirs: vec![],
            },
            Self::Options(options) => MaxFilesPerDirectoryRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_files: options.max_files.unwrap_or(20),
                ignore_dirs: options.ignore_dirs.clone().unwrap_or_default(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRuleOptions {
    severity: Option<String>,
    #[serde(rename = "allow-doc-comments")]
    allow_doc_comments: Option<bool>,
    #[serde(rename = "max-lines")]
    max_lines: Option<usize>,
    #[serde(rename = "max-exports")]
    max_exports: Option<usize>,
    #[serde(rename = "max-depth")]
    max_depth: Option<usize>,
    #[serde(rename = "allow-patterns")]
    allow_patterns: Option<Vec<String>>,
    #[serde(rename = "extra-folders")]
    extra_folders: Option<Vec<String>>,
    #[serde(rename = "extra-file-suffixes")]
    extra_file_suffixes: Option<Vec<String>>,
    #[serde(rename = "ignore-dirs")]
    ignore_dirs: Option<Vec<String>>,
    #[serde(rename = "ignore-names")]
    ignore_names: Option<Vec<String>>,
    #[serde(rename = "max-files")]
    max_files: Option<usize>,
}
