use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig,
    GitignoreConfig, MaxDirectoryDepthRuleConfig, MaxItemsPerDirectoryRuleConfig,
    MinItemsPerDirectoryRuleConfig, NoConsoleRuleConfig, NoDumpFilesRuleConfig,
    NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig, NoInterfaceRuleConfig,
    NoLogicInDomainRuleConfig, RuleConfig, RulesConfig, Severity, UpwardImportRuleConfig,
};

#[derive(Debug, Default, Deserialize)]
pub struct RawConfig {
    pub project: Option<RawProjectConfig>,
    pub rules: Option<HashMap<String, RawRuleConfig>>,
}

impl RawConfig {
    pub fn rule(&self, name: &str) -> Option<&RawRuleConfig> {
        self.rules.as_ref().and_then(|rules| rules.get(name))
    }

    pub fn rules_config(&self) -> RulesConfig {
        RulesConfig {
            boolean_prefix: self.boolean_prefix(),
            hook_no_jsx: self.hook_no_jsx(),
            max_directory_depth: self.max_directory_depth(),
            max_file_exports: self.max_file_exports(),
            max_items_per_directory: self.max_items_per_directory(),
            min_items_per_directory: self.min_items_per_directory(),
            no_barrel_chain: self.no_barrel_chain(),
            no_barrel_files: self.no_barrel_files(),
            no_comments: self.no_comments(),
            no_console: self.no_console(),
            no_debugger: self.no_debugger(),
            no_default_export: self.no_default_export(),
            no_duplicate_file_names: self.no_duplicate_file_names(),
            no_dump_files: self.no_dump_files(),
            no_empty_directories: self.no_empty_directories(),
            no_empty_interface: self.no_empty_interface(),
            no_enums: self.no_enums(),
            no_eval: self.no_eval(),
            no_export_star: self.no_export_star(),
            no_inline_types: self.no_inline_types(),
            no_interface: self.no_interface(),
            no_large_file: self.no_large_file(),
            no_logic_in_barrel: self.no_logic_in_barrel(),
            no_logic_in_domain: self.no_logic_in_domain(),
            no_mutable_exports: self.no_mutable_exports(),
            no_silent_catch: self.no_silent_catch(),
            no_upward_import: self.no_upward_import(),
            prefer_satisfies: self.prefer_satisfies(),
        }
    }

    pub fn gitignore(&self) -> GitignoreConfig {
        let project = self.project.as_ref();
        GitignoreConfig {
            enabled: project
                .and_then(|p| p.respect_gitignore)
                .unwrap_or_default(),
        }
    }

    fn no_comments(&self) -> CommentsRuleConfig {
        self.rule("no-comments")
            .map(RawRuleConfig::to_comments_config)
            .unwrap_or_default()
    }

    fn no_logic_in_barrel(&self) -> RuleConfig {
        self.rule("no-logic-in-barrel")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_default_export(&self) -> RuleConfig {
        self.rule("no-default-export")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_export_star(&self) -> RuleConfig {
        self.rule("no-export-star")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_inline_types(&self) -> RuleConfig {
        self.rule("no-inline-types")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn max_file_exports(&self) -> FileExportsRuleConfig {
        self.rule("max-file-exports")
            .map(RawRuleConfig::to_file_exports_config)
            .unwrap_or_default()
    }

    fn no_upward_import(&self) -> UpwardImportRuleConfig {
        self.rule("no-upward-import")
            .map(RawRuleConfig::to_upward_import_config)
            .unwrap_or_default()
    }

    fn no_large_file(&self) -> FileLengthRuleConfig {
        self.rule("no-large-file")
            .map(RawRuleConfig::to_file_length_config)
            .unwrap_or_default()
    }

    fn no_enums(&self) -> RuleConfig {
        self.rule("no-enums")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_barrel_files(&self) -> RuleConfig {
        self.rule("no-barrel-files")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_barrel_chain(&self) -> RuleConfig {
        self.rule("no-barrel-chain")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn boolean_prefix(&self) -> BooleanPrefixRuleConfig {
        self.rule("boolean-prefix")
            .map(RawRuleConfig::to_boolean_prefix_config)
            .unwrap_or_default()
    }

    fn no_console(&self) -> NoConsoleRuleConfig {
        self.rule("no-console")
            .map(RawRuleConfig::to_no_console_config)
            .unwrap_or_default()
    }

    fn no_debugger(&self) -> RuleConfig {
        self.rule("no-debugger")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_eval(&self) -> RuleConfig {
        self.rule("no-eval")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_logic_in_domain(&self) -> NoLogicInDomainRuleConfig {
        self.rule("no-logic-in-domain")
            .map(RawRuleConfig::to_no_logic_in_domain_config)
            .unwrap_or_default()
    }

    fn no_empty_directories(&self) -> NoEmptyDirectoriesRuleConfig {
        self.rule("no-empty-directories")
            .map(RawRuleConfig::to_no_empty_directories_config)
            .unwrap_or_default()
    }

    fn no_duplicate_file_names(&self) -> NoDuplicateFileNamesRuleConfig {
        self.rule("no-duplicate-file-names")
            .map(RawRuleConfig::to_no_duplicate_file_names_config)
            .unwrap_or_default()
    }

    fn max_items_per_directory(&self) -> MaxItemsPerDirectoryRuleConfig {
        self.rule("max-items-per-directory")
            .map(RawRuleConfig::to_max_items_per_directory_config)
            .unwrap_or_default()
    }

    fn min_items_per_directory(&self) -> MinItemsPerDirectoryRuleConfig {
        self.rule("min-items-per-directory")
            .map(RawRuleConfig::to_min_items_per_directory_config)
            .unwrap_or_default()
    }

    fn max_directory_depth(&self) -> MaxDirectoryDepthRuleConfig {
        self.rule("max-directory-depth")
            .map(RawRuleConfig::to_max_directory_depth_config)
            .unwrap_or_default()
    }

    fn no_empty_interface(&self) -> RuleConfig {
        self.rule("no-empty-interface")
            .map(|rule| rule.to_rule_config_with_default(Severity::Error))
            .unwrap_or(RuleConfig {
                severity: Severity::Error,
            })
    }

    fn no_interface(&self) -> NoInterfaceRuleConfig {
        self.rule("no-interface")
            .map(RawRuleConfig::to_no_interface_config)
            .unwrap_or_default()
    }

    fn no_mutable_exports(&self) -> RuleConfig {
        self.rule("no-mutable-exports")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_silent_catch(&self) -> RuleConfig {
        self.rule("no-silent-catch")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn prefer_satisfies(&self) -> RuleConfig {
        self.rule("prefer-satisfies")
            .map(|rule| rule.to_rule_config_with_default(Severity::Info))
            .unwrap_or(RuleConfig {
                severity: Severity::Info,
            })
    }

    fn hook_no_jsx(&self) -> RuleConfig {
        self.rule("hook-no-jsx")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_dump_files(&self) -> NoDumpFilesRuleConfig {
        self.rule("no-dump-files")
            .map(RawRuleConfig::to_no_dump_files_config)
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct RawProjectConfig {
    pub root: Option<PathBuf>,
    #[serde(rename = "respect-gitignore")]
    pub respect_gitignore: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawRuleConfig {
    Severity(String),
    Options(Box<RawRuleOptions>),
}

impl RawRuleConfig {
    pub fn to_comments_config(&self) -> CommentsRuleConfig {
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

    pub fn to_rule_config(&self) -> RuleConfig {
        self.to_rule_config_with_default(Severity::Warn)
    }

    pub fn to_rule_config_with_default(&self, default_severity: Severity) -> RuleConfig {
        match self {
            Self::Severity(severity) => RuleConfig {
                severity: Severity::from_str(severity),
            },
            Self::Options(options) => RuleConfig {
                severity: options
                    .severity
                    .as_deref()
                    .map(Severity::from_str)
                    .unwrap_or(default_severity),
            },
        }
    }

    pub fn to_file_length_config(&self) -> FileLengthRuleConfig {
        match self {
            Self::Severity(severity) => FileLengthRuleConfig {
                severity: Severity::from_str(severity),
                max_lines: FileLengthRuleConfig::default().max_lines,
            },
            Self::Options(options) => FileLengthRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_lines: options
                    .max_lines
                    .unwrap_or(FileLengthRuleConfig::default().max_lines),
            },
        }
    }

    pub fn to_file_exports_config(&self) -> FileExportsRuleConfig {
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

    pub fn to_upward_import_config(&self) -> UpwardImportRuleConfig {
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

    pub fn to_no_empty_directories_config(&self) -> NoEmptyDirectoriesRuleConfig {
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

    pub fn to_no_logic_in_domain_config(&self) -> NoLogicInDomainRuleConfig {
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

    pub fn to_no_console_config(&self) -> NoConsoleRuleConfig {
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

    pub fn to_no_duplicate_file_names_config(&self) -> NoDuplicateFileNamesRuleConfig {
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

    pub fn to_max_items_per_directory_config(&self) -> MaxItemsPerDirectoryRuleConfig {
        match self {
            Self::Severity(severity) => MaxItemsPerDirectoryRuleConfig {
                severity: Severity::from_str(severity),
                max_items: 20,
                ignore_dirs: vec![],
                count_folders: false,
            },
            Self::Options(options) => MaxItemsPerDirectoryRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_items: options.max_items.unwrap_or(20),
                ignore_dirs: options.ignore_dirs.clone().unwrap_or_default(),
                count_folders: options.count_folders.unwrap_or(false),
            },
        }
    }

    pub fn to_min_items_per_directory_config(&self) -> MinItemsPerDirectoryRuleConfig {
        match self {
            Self::Severity(severity) => MinItemsPerDirectoryRuleConfig {
                severity: Severity::from_str(severity),
                min_items: 3,
                ignore_dirs: vec![],
                count_folders: false,
            },
            Self::Options(options) => MinItemsPerDirectoryRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                min_items: options.min_items.unwrap_or(3),
                ignore_dirs: options.ignore_dirs.clone().unwrap_or_default(),
                count_folders: options.count_folders.unwrap_or(false),
            },
        }
    }

    pub fn to_max_directory_depth_config(&self) -> MaxDirectoryDepthRuleConfig {
        match self {
            Self::Severity(severity) => MaxDirectoryDepthRuleConfig {
                severity: Severity::from_str(severity),
                max_depth: 5,
                ignore_dirs: vec![],
            },
            Self::Options(options) => MaxDirectoryDepthRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                max_depth: options.max_depth.unwrap_or(5),
                ignore_dirs: options.ignore_dirs.clone().unwrap_or_default(),
            },
        }
    }

    pub fn to_no_interface_config(&self) -> NoInterfaceRuleConfig {
        match self {
            Self::Severity(severity) => NoInterfaceRuleConfig {
                severity: Severity::from_str(severity),
                allow_declaration_merging: true,
            },
            Self::Options(options) => NoInterfaceRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                allow_declaration_merging: options.allow_declaration_merging.unwrap_or(true),
            },
        }
    }

    pub fn to_no_dump_files_config(&self) -> NoDumpFilesRuleConfig {
        match self {
            Self::Severity(severity) => NoDumpFilesRuleConfig {
                severity: Severity::from_str(severity),
                extra_names: vec![],
            },
            Self::Options(options) => NoDumpFilesRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                extra_names: options.extra_names.clone().unwrap_or_default(),
            },
        }
    }

    pub fn to_boolean_prefix_config(&self) -> BooleanPrefixRuleConfig {
        match self {
            Self::Severity(severity) => BooleanPrefixRuleConfig {
                severity: Severity::from_str(severity),
                prefixes: vec![],
                ignore_constants: false,
            },
            Self::Options(options) => BooleanPrefixRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                prefixes: options.prefixes.clone().unwrap_or_default(),
                ignore_constants: options.ignore_constants.unwrap_or(false),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RawRuleOptions {
    pub severity: Option<String>,
    #[serde(rename = "allow-doc-comments")]
    pub allow_doc_comments: Option<bool>,
    #[serde(rename = "max-lines")]
    pub max_lines: Option<usize>,
    #[serde(rename = "max-exports")]
    pub max_exports: Option<usize>,
    #[serde(rename = "max-depth")]
    pub max_depth: Option<usize>,
    #[serde(rename = "allow-patterns")]
    pub allow_patterns: Option<Vec<String>>,
    #[serde(rename = "extra-folders")]
    pub extra_folders: Option<Vec<String>>,
    #[serde(rename = "extra-file-suffixes")]
    pub extra_file_suffixes: Option<Vec<String>>,
    #[serde(rename = "ignore-dirs")]
    pub ignore_dirs: Option<Vec<String>>,
    #[serde(rename = "ignore-names")]
    pub ignore_names: Option<Vec<String>>,
    #[serde(rename = "max-items")]
    pub max_items: Option<usize>,
    #[serde(rename = "min-items")]
    pub min_items: Option<usize>,
    #[serde(rename = "count-folders")]
    pub count_folders: Option<bool>,
    #[serde(rename = "allow-declaration-merging")]
    pub allow_declaration_merging: Option<bool>,
    #[serde(rename = "extra-names")]
    pub extra_names: Option<Vec<String>>,
    pub prefixes: Option<Vec<String>>,
    #[serde(rename = "ignore-constants")]
    pub ignore_constants: Option<bool>,
}
