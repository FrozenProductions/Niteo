#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Off,
    Info,
    Warn,
    Error,
}

impl Severity {
    pub fn from_str(value: &str) -> Self {
        match value {
            "off" => Self::Off,
            "info" => Self::Info,
            "error" => Self::Error,
            _ => Self::Warn,
        }
    }

    pub fn is_enabled(self) -> bool {
        self != Self::Off
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
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
pub struct FileLengthRuleConfig {
    pub severity: Severity,
    pub max_lines: usize,
}

impl Default for FileLengthRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_lines: 500,
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
pub struct MaxItemsPerDirectoryRuleConfig {
    pub severity: Severity,
    pub max_items: usize,
    pub ignore_dirs: Vec<String>,
    pub count_folders: bool,
}

impl Default for MaxItemsPerDirectoryRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_items: 20,
            ignore_dirs: vec![],
            count_folders: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinItemsPerDirectoryRuleConfig {
    pub severity: Severity,
    pub min_items: usize,
    pub ignore_dirs: Vec<String>,
    pub count_folders: bool,
}

impl Default for MinItemsPerDirectoryRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            min_items: 3,
            ignore_dirs: vec![],
            count_folders: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxDirectoryDepthRuleConfig {
    pub severity: Severity,
    pub max_depth: usize,
    pub ignore_dirs: Vec<String>,
}

impl Default for MaxDirectoryDepthRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_depth: 5,
            ignore_dirs: vec![],
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

#[derive(Debug, Clone)]
pub struct BooleanPrefixRuleConfig {
    pub severity: Severity,
    pub prefixes: Vec<String>,
    pub ignore_constants: bool,
}

impl Default for BooleanPrefixRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            prefixes: vec![],
            ignore_constants: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HookPrefixRuleConfig {
    pub severity: Severity,
    pub prefixes: Vec<String>,
}

impl Default for HookPrefixRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            prefixes: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoInterfaceRuleConfig {
    pub severity: Severity,
    pub allow_declaration_merging: bool,
}

impl Default for NoInterfaceRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_declaration_merging: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoDumpFilesRuleConfig {
    pub severity: Severity,
    pub extra_names: Vec<String>,
}

impl Default for NoDumpFilesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            extra_names: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct RulesConfig {
    pub boolean_prefix: BooleanPrefixRuleConfig,
    pub component_file_only_components: RuleConfig,
    pub hook_no_jsx: RuleConfig,
    pub hook_prefix: HookPrefixRuleConfig,
    pub max_directory_depth: MaxDirectoryDepthRuleConfig,
    pub max_file_exports: FileExportsRuleConfig,
    pub max_items_per_directory: MaxItemsPerDirectoryRuleConfig,
    pub min_items_per_directory: MinItemsPerDirectoryRuleConfig,
    pub no_barrel_chain: RuleConfig,
    pub no_barrel_files: RuleConfig,
    pub no_comments: CommentsRuleConfig,
    pub no_console: NoConsoleRuleConfig,
    pub no_debugger: RuleConfig,
    pub no_component_default_export: RuleConfig,
    pub no_default_export: RuleConfig,
    pub no_duplicate_file_names: NoDuplicateFileNamesRuleConfig,
    pub no_dump_files: NoDumpFilesRuleConfig,
    pub no_empty_directories: NoEmptyDirectoriesRuleConfig,
    pub no_empty_interface: RuleConfig,
    pub no_enums: RuleConfig,
    pub no_eval: RuleConfig,
    pub no_export_star: RuleConfig,
    pub no_namespace: RuleConfig,
    pub no_inline_types: RuleConfig,
    pub no_interface: NoInterfaceRuleConfig,
    pub no_large_file: FileLengthRuleConfig,
    pub no_logic_in_barrel: RuleConfig,
    pub no_logic_in_domain: RuleConfig,
    pub no_mutable_exports: RuleConfig,
    pub no_silent_catch: RuleConfig,
    pub no_then_chain: RuleConfig,
    pub no_upward_import: UpwardImportRuleConfig,
    pub prefer_satisfies: RuleConfig,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            boolean_prefix: BooleanPrefixRuleConfig::default(),
            component_file_only_components: RuleConfig::default(),
            hook_no_jsx: RuleConfig::default(),
            hook_prefix: HookPrefixRuleConfig::default(),
            max_directory_depth: MaxDirectoryDepthRuleConfig::default(),
            max_file_exports: FileExportsRuleConfig::default(),
            max_items_per_directory: MaxItemsPerDirectoryRuleConfig::default(),
            min_items_per_directory: MinItemsPerDirectoryRuleConfig::default(),
            no_barrel_chain: RuleConfig::default(),
            no_barrel_files: RuleConfig::default(),
            no_comments: CommentsRuleConfig::default(),
            no_console: NoConsoleRuleConfig::default(),
            no_debugger: RuleConfig::default(),
            no_component_default_export: RuleConfig::default(),
            no_default_export: RuleConfig::default(),
            no_duplicate_file_names: NoDuplicateFileNamesRuleConfig::default(),
            no_dump_files: NoDumpFilesRuleConfig::default(),
            no_empty_directories: NoEmptyDirectoriesRuleConfig::default(),
            no_empty_interface: RuleConfig {
                severity: Severity::Error,
            },
            no_enums: RuleConfig::default(),
            no_eval: RuleConfig::default(),
            no_export_star: RuleConfig::default(),
            no_inline_types: RuleConfig::default(),
            no_interface: NoInterfaceRuleConfig::default(),
            no_large_file: FileLengthRuleConfig::default(),
            no_logic_in_barrel: RuleConfig::default(),
            no_logic_in_domain: RuleConfig::default(),
            no_mutable_exports: RuleConfig::default(),
            no_namespace: RuleConfig::default(),
            no_silent_catch: RuleConfig::default(),
            no_then_chain: RuleConfig::default(),
            no_upward_import: UpwardImportRuleConfig::default(),
            prefer_satisfies: RuleConfig {
                severity: Severity::Info,
            },
        }
    }
}
