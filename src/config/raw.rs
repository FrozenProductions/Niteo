use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, EntryFileNoLogicRuleConfig, FileExportsRuleConfig,
    FileLengthRuleConfig, GitignoreConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig, NoAnyRuleConfig,
    NoConsoleRuleConfig, NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig,
    NoEmptyDirectoriesRuleConfig, NoInterfaceRuleConfig, RuleConfig, Severity,
    UpwardImportRuleConfig,
};
use super::structure::{DomainConfig, ProjectStructureConfig};
use crate::rules::RulesConfig;

#[derive(Debug, Default, Deserialize)]
pub struct RawConfig {
    pub project: Option<RawProjectConfig>,
    pub rules: Option<HashMap<String, RawRuleConfig>>,
}

macro_rules! declare_raw_rules {
    (
        simple {
            $( $simple_method:ident => $simple_name:literal ),* $(,)?
        }
        custom_default {
            $( $cd_method:ident => ($cd_name:literal, $cd_sev:expr) ),* $(,)?
        }
        custom {
            $( $custom_method:ident => ($custom_name:literal, $custom_converter:ident, $custom_type:ty) ),* $(,)?
        }
    ) => {
        impl RawConfig {
            pub fn rule(&self, name: &str) -> Option<&RawRuleConfig> {
                self.rules.as_ref().and_then(|rules| rules.get(name))
            }

            pub fn rules_config(&self) -> RulesConfig {
                RulesConfig {
                    $( $simple_method: self.$simple_method(), )*
                    $( $cd_method: self.$cd_method(), )*
                    $( $custom_method: self.$custom_method(), )*
                }
            }

            $(
                fn $simple_method(&self) -> RuleConfig {
                    self.rule($simple_name)
                        .map(RawRuleConfig::to_rule_config)
                        .unwrap_or_default()
                }
            )*

            $(
                fn $cd_method(&self) -> RuleConfig {
                    self.rule($cd_name)
                        .map(|rule| rule.to_rule_config_with_default($cd_sev))
                        .unwrap_or(RuleConfig { severity: $cd_sev })
                }
            )*

            $(
                fn $custom_method(&self) -> $custom_type {
                    self.rule($custom_name)
                        .map(RawRuleConfig::$custom_converter)
                        .unwrap_or_default()
                }
            )*
        }
    };
}

declare_raw_rules! {
    simple {
        component_file_only_components => "component-file-only-components",
        hook_no_jsx => "hook-no-jsx",
        no_barrel_chain => "no-barrel-chain",
        no_barrel_files => "no-barrel-files",
        no_component_default_export => "no-component-default-export",
        no_debugger => "no-debugger",
        no_default_export => "no-default-export",
        no_enums => "no-enums",
        no_eval => "no-eval",
        no_export_star => "no-export-star",
        no_inline_types => "no-inline-types",
        no_logic_in_barrel => "no-logic-in-barrel",
        no_logic_in_domain => "no-logic-in-domain",
        no_mutable_exports => "no-mutable-exports",
        no_namespace => "no-namespace",
        no_non_null_assertion => "no-non-null-assertion",
        no_silent_catch => "no-silent-catch",
        no_test_code_in_production => "no-test-code-in-production",
        no_test_import => "no-test-import",
        no_then_chain => "no-then-chain",
    }
    custom_default {
        no_empty_interface => ("no-empty-interface", Severity::Error),
        prefer_satisfies => ("prefer-satisfies", Severity::Info),
    }
    custom {
        boolean_prefix => ("boolean-prefix", to_boolean_prefix_config, BooleanPrefixRuleConfig),
        entry_file_no_logic => ("entry-file-no-logic", to_entry_file_no_logic_config, EntryFileNoLogicRuleConfig),
        hook_prefix => ("hook-prefix", to_hook_prefix_config, HookPrefixRuleConfig),
        max_directory_depth => ("max-directory-depth", to_max_directory_depth_config, MaxDirectoryDepthRuleConfig),
        max_file_exports => ("max-file-exports", to_file_exports_config, FileExportsRuleConfig),
        max_items_per_directory => ("max-items-per-directory", to_max_items_per_directory_config, MaxItemsPerDirectoryRuleConfig),
        min_items_per_directory => ("min-items-per-directory", to_min_items_per_directory_config, MinItemsPerDirectoryRuleConfig),
        no_any => ("no-any", to_no_any_config, NoAnyRuleConfig),
        no_comments => ("no-comments", to_comments_config, CommentsRuleConfig),
        no_console => ("no-console", to_no_console_config, NoConsoleRuleConfig),
        no_dump_files => ("no-dump-files", to_no_dump_files_config, NoDumpFilesRuleConfig),
        no_duplicate_file_names => ("no-duplicate-file-names", to_no_duplicate_file_names_config, NoDuplicateFileNamesRuleConfig),
        no_empty_directories => ("no-empty-directories", to_no_empty_directories_config, NoEmptyDirectoriesRuleConfig),
        no_interface => ("no-interface", to_no_interface_config, NoInterfaceRuleConfig),
        no_large_file => ("no-large-file", to_file_length_config, FileLengthRuleConfig),
        no_upward_import => ("no-upward-import", to_upward_import_config, UpwardImportRuleConfig),
    }
}

impl RawConfig {
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
            tests: raw_structure
                .and_then(|s| s.tests.as_ref())
                .map(|d| d.to_domain_config(&defaults.tests))
                .unwrap_or(defaults.tests),
            generated: raw_structure
                .and_then(|s| s.generated.as_ref())
                .map(|d| d.to_domain_config(&defaults.generated))
                .unwrap_or(defaults.generated),
        }
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
    pub tests: Option<RawDomainConfig>,
    pub generated: Option<RawDomainConfig>,
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

macro_rules! declare_option_converters {
    (
        $(
            $method:ident => ($config_type:ty) {
                $( $field:ident: default($default:expr) ),* $(,)?
                ;
                $( $clone_field:ident: clone_default ),* $(,)?
            }
        ),* $(,)?
    ) => {
        $(
            pub fn $method(&self) -> $config_type {
                match self {
                    Self::Severity(severity) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = Severity::from_str(severity);
                        cfg
                    }
                    Self::Options(options) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = Severity::from_str(
                            options.severity.as_deref().unwrap_or("warn"),
                        );
                        $( cfg.$field = options.$field.unwrap_or($default); )*
                        $( cfg.$clone_field = options.$clone_field.clone().unwrap_or_else(|| <$config_type>::default().$clone_field); )*
                        cfg
                    }
                }
            }
        )*
    };
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawRuleConfig {
    Severity(String),
    Options(Box<RawRuleOptions>),
}

impl RawRuleConfig {
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

    declare_option_converters! {
        to_boolean_prefix_config => (BooleanPrefixRuleConfig) {
            ignore_constants: default(false)
            ;
            prefixes: clone_default
        },
        to_comments_config => (CommentsRuleConfig) {
            allow_doc_comments: default(true)
            ;
        },
        to_entry_file_no_logic_config => (EntryFileNoLogicRuleConfig) {
            ;
            entry_files: clone_default
        },
        to_file_exports_config => (FileExportsRuleConfig) {
            max_exports: default(10)
            ;
        },
        to_file_length_config => (FileLengthRuleConfig) {
            max_lines: default(FileLengthRuleConfig::default().max_lines)
            ;
        },
        to_hook_prefix_config => (HookPrefixRuleConfig) {
            ;
            prefixes: clone_default
        },
        to_max_directory_depth_config => (MaxDirectoryDepthRuleConfig) {
            max_depth: default(5)
            ;
            ignore_dirs: clone_default
        },
        to_max_items_per_directory_config => (MaxItemsPerDirectoryRuleConfig) {
            max_items: default(20),
            count_folders: default(false)
            ;
            ignore_dirs: clone_default
        },
        to_min_items_per_directory_config => (MinItemsPerDirectoryRuleConfig) {
            min_items: default(3),
            count_folders: default(false)
            ;
            ignore_dirs: clone_default
        },
        to_no_any_config => (NoAnyRuleConfig) {
            ;
            allowed_folders: clone_default
        },
        to_no_console_config => (NoConsoleRuleConfig) {
            ;
            allow_patterns: clone_default
        },
        to_no_dump_files_config => (NoDumpFilesRuleConfig) {
            ;
            extra_names: clone_default
        },
        to_no_duplicate_file_names_config => (NoDuplicateFileNamesRuleConfig) {
            ;
            ignore_names: clone_default
        },
        to_no_empty_directories_config => (NoEmptyDirectoriesRuleConfig) {
            ;
            ignore_dirs: clone_default
        },
        to_no_interface_config => (NoInterfaceRuleConfig) {
            allow_declaration_merging: default(true)
            ;
        },
        to_upward_import_config => (UpwardImportRuleConfig) {
            max_depth: default(0)
            ;
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
    #[serde(rename = "entry-files")]
    pub entry_files: Option<Vec<String>>,
    #[serde(rename = "allowed-folders")]
    pub allowed_folders: Option<Vec<String>>,
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

[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts", ".tests.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).expect("valid config");
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".hook.ts", ".hooks.ts"]);
        assert_eq!(structure.components.folders, vec!["components"]);
        assert_eq!(structure.types.folders, vec!["types"]);
        assert_eq!(structure.constants.folders, vec!["constants"]);
        assert_eq!(structure.tests.folders, vec!["tests"]);
        assert_eq!(structure.tests.file_suffixes, vec![".test.ts", ".tests.ts"]);
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
        assert_eq!(structure.tests.folders, defaults.tests.folders);
        assert_eq!(structure.tests.file_suffixes, defaults.tests.file_suffixes);
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
