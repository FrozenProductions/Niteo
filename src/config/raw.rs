use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig,
    GitignoreConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig, NoConsoleRuleConfig,
    NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig,
    NoInterfaceRuleConfig, RuleConfig, RulesConfig, Severity, UpwardImportRuleConfig,
};
use super::structure::{DomainConfig, ProjectStructureConfig};

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
            component_file_only_components: self.component_file_only_components(),
            hook_no_jsx: self.hook_no_jsx(),
            hook_prefix: self.hook_prefix(),
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
            no_namespace: self.no_namespace(),
            no_silent_catch: self.no_silent_catch(),
            no_then_chain: self.no_then_chain(),
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

    pub fn structure(&self) -> ProjectStructureConfig {
        let raw_structure = self.project.as_ref().and_then(|p| p.structure.as_ref());

        let defaults = ProjectStructureConfig::default();

        ProjectStructureConfig {
            hooks: raw_structure
                .and_then(|s| s.hooks.as_ref())
                .map(|d| d.to_domain_config(&defaults.hooks))
                .unwrap_or(defaults.hooks),
            components: raw_structure
                .and_then(|s| s.components.as_ref())
                .map(|d| d.to_domain_config(&defaults.components))
                .unwrap_or(defaults.components),
            types: raw_structure
                .and_then(|s| s.types.as_ref())
                .map(|d| d.to_domain_config(&defaults.types))
                .unwrap_or(defaults.types),
            constants: raw_structure
                .and_then(|s| s.constants.as_ref())
                .map(|d| d.to_domain_config(&defaults.constants))
                .unwrap_or(defaults.constants),
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

    fn no_logic_in_domain(&self) -> RuleConfig {
        self.rule("no-logic-in-domain")
            .map(RawRuleConfig::to_rule_config)
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

    fn no_namespace(&self) -> RuleConfig {
        self.rule("no-namespace")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_silent_catch(&self) -> RuleConfig {
        self.rule("no-silent-catch")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn no_then_chain(&self) -> RuleConfig {
        self.rule("no-then-chain")
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

    fn component_file_only_components(&self) -> RuleConfig {
        self.rule("component-file-only-components")
            .map(RawRuleConfig::to_rule_config)
            .unwrap_or_default()
    }

    fn hook_prefix(&self) -> HookPrefixRuleConfig {
        self.rule("hook-prefix")
            .map(RawRuleConfig::to_hook_prefix_config)
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
    pub structure: Option<RawProjectStructure>,
}

#[derive(Debug, Deserialize)]
pub struct RawProjectStructure {
    pub hooks: Option<RawDomainConfig>,
    pub components: Option<RawDomainConfig>,
    pub types: Option<RawDomainConfig>,
    pub constants: Option<RawDomainConfig>,
}

#[derive(Debug, Deserialize)]
pub struct RawDomainConfig {
    pub folders: Option<Vec<String>>,
    #[serde(rename = "file-suffixes")]
    pub file_suffixes: Option<Vec<String>>,
}

impl RawDomainConfig {
    fn to_domain_config(&self, defaults: &DomainConfig) -> DomainConfig {
        DomainConfig {
            folders: self
                .folders
                .clone()
                .unwrap_or_else(|| defaults.folders.clone()),
            file_suffixes: self
                .file_suffixes
                .clone()
                .unwrap_or_else(|| defaults.file_suffixes.clone()),
        }
    }
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

    pub fn to_hook_prefix_config(&self) -> HookPrefixRuleConfig {
        match self {
            Self::Severity(severity) => HookPrefixRuleConfig {
                severity: Severity::from_str(severity),
                prefixes: vec![],
            },
            Self::Options(options) => HookPrefixRuleConfig {
                severity: Severity::from_str(options.severity.as_deref().unwrap_or("warn")),
                prefixes: options.prefixes.clone().unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses_structure() {
        let source = r#"
[project]
root = "src"

[project.structure.hooks]
folders = ["hooks"]
file-suffixes = [".hook.ts", ".hooks.ts"]

[project.structure.components]
folders = ["components"]
file-suffixes = [".component.tsx", ".components.tsx"]

[project.structure.types]
folders = ["types"]
file-suffixes = [".type.ts", ".types.ts"]

[project.structure.constants]
folders = ["constants"]
file-suffixes = [".constant.ts", ".constants.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).expect("valid config");
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".hook.ts", ".hooks.ts"]);
        assert_eq!(structure.components.folders, vec!["components"]);
        assert_eq!(structure.types.folders, vec!["types"]);
        assert_eq!(structure.constants.folders, vec!["constants"]);
    }

    #[test]
    fn missing_structure_uses_defaults() {
        let source = r#"
[project]
root = "src"
"#;
        let raw: RawConfig = toml::from_str(source).expect("valid config");
        let structure = raw.structure();
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.hooks.folders, defaults.hooks.folders);
        assert_eq!(structure.hooks.file_suffixes, defaults.hooks.file_suffixes);
        assert_eq!(structure.components.folders, defaults.components.folders);
        assert_eq!(structure.types.folders, defaults.types.folders);
        assert_eq!(structure.constants.folders, defaults.constants.folders);
    }

    #[test]
    fn custom_structure_overrides_defaults() {
        let source = r#"
[project.structure.hooks]
folders = ["custom-hooks"]
file-suffixes = [".custom.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).expect("valid config");
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["custom-hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".custom.ts"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.components.folders, defaults.components.folders);
    }

    #[test]
    fn partial_domain_config_preserves_defaults() {
        let source = r#"
[project.structure.types]
folders = ["typings"]
"#;
        let raw: RawConfig = toml::from_str(source).expect("valid config");
        let structure = raw.structure();
        assert_eq!(structure.types.folders, vec!["typings"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.types.file_suffixes, defaults.types.file_suffixes);
    }
}
