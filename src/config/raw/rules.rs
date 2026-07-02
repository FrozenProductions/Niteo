use serde::Deserialize;

use super::super::rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, EntryFileNoLogicRuleConfig, FileExportsRuleConfig,
    FileLengthRuleConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxFunctionParamsRuleConfig, MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig,
    NestingContext, NoAbbreviationsRuleConfig, NoAnemicDomainRuleConfig, NoAnyRuleConfig,
    NoConsoleRuleConfig, NoDefaultExportRuleConfig, NoDumpFilesRuleConfig,
    NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig, NoEmptyDomainRuleConfig,
    NoGodDomainRuleConfig, NoInterfaceRuleConfig, NoMagicNumbersRuleConfig,
    NoNestedFunctionsRuleConfig, NoOrphanFilesRuleConfig, NoRestrictedImportsRuleConfig,
    RuleConfig, Severity, UpwardImportRuleConfig,
};
use super::config::RawConfig;
use crate::rules::RulesConfig;

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
            pub fn $method(&self) -> Result<$config_type, String> {
                match self {
                    Self::Severity(severity) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = severity.parse::<Severity>()?;
                        Ok(cfg)
                    }
                    Self::Options(options) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = match options.severity.as_deref() {
                            Some(severity) => severity.parse::<Severity>()?,
                            None => Severity::Warn,
                        };
                        $( cfg.$field = options.$field.unwrap_or($default); )*
                        $( cfg.$clone_field = options.$clone_field.clone().unwrap_or_else(|| <$config_type>::default().$clone_field); )*
                        Ok(cfg)
                    }
                }
            }
        )*
    };
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

            pub fn rules_config(&self) -> Result<RulesConfig, String> {
                Ok(RulesConfig {
                    $( $simple_method: self.$simple_method()?, )*
                    $( $cd_method: self.$cd_method()?, )*
                    $( $custom_method: self.$custom_method()?, )*
                })
            }

            $(
                fn $simple_method(&self) -> Result<RuleConfig, String> {
                    match self.rule($simple_name) {
                        Some(rule) => rule.to_rule_config().map_err(|error| {
                            format!("in rule '{}': {}", $simple_name, error)
                        }),
                        None => Ok(RuleConfig::default()),
                    }
                }
            )*

            $(
                fn $cd_method(&self) -> Result<RuleConfig, String> {
                    match self.rule($cd_name) {
                        Some(rule) => rule.to_rule_config_with_default($cd_sev).map_err(|error| {
                            format!("in rule '{}': {}", $cd_name, error)
                        }),
                        None => Ok(RuleConfig { severity: $cd_sev }),
                    }
                }
            )*

            $(
                fn $custom_method(&self) -> Result<$custom_type, String> {
                    match self.rule($custom_name) {
                        Some(rule) => RawRuleConfig::$custom_converter(rule).map_err(|error| {
                            format!("in rule '{}': {}", $custom_name, error)
                        }),
                        None => Ok(<$custom_type>::default()),
                    }
                }
            )*
        }
    };
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawRuleConfig {
    Severity(String),
    Options(Box<RawRuleOptions>),
}

impl RawRuleConfig {
    pub fn merge(parent: &RawRuleConfig, child: &RawRuleConfig) -> RawRuleConfig {
        match (parent, child) {
            (_, RawRuleConfig::Severity(sev)) => RawRuleConfig::Severity(sev.clone()),
            (RawRuleConfig::Severity(parent_sev), RawRuleConfig::Options(child_opts)) => {
                let mut merged = (**child_opts).clone();
                if merged.severity.is_none() {
                    merged.severity = Some(parent_sev.clone());
                }
                RawRuleConfig::Options(Box::new(merged))
            }
            (RawRuleConfig::Options(parent_opts), RawRuleConfig::Options(child_opts)) => {
                RawRuleConfig::Options(Box::new(RawRuleOptions::merge(parent_opts, child_opts)))
            }
        }
    }
}

impl RawRuleConfig {
    pub fn to_rule_config(&self) -> Result<RuleConfig, String> {
        self.to_rule_config_with_default(Severity::Warn)
    }

    pub fn to_rule_config_with_default(
        &self,
        default_severity: Severity,
    ) -> Result<RuleConfig, String> {
        match self {
            Self::Severity(severity) => Ok(RuleConfig {
                severity: severity.parse::<Severity>()?,
            }),
            Self::Options(options) => Ok(RuleConfig {
                severity: match options.severity.as_deref() {
                    Some(severity) => severity.parse::<Severity>()?,
                    None => default_severity,
                },
            }),
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
            max_exports: default(10),
            count_default: default(true)
            ;
        },
        to_max_function_params_config => (MaxFunctionParamsRuleConfig) {
            max_params: default(3)
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
        to_no_abbreviations_config => (NoAbbreviationsRuleConfig) {
            ;
            extra_abbreviations: clone_default,
            allow_abbreviations: clone_default
        },
        to_no_any_config => (NoAnyRuleConfig) {
            ;
            allowed_folders: clone_default
        },
        to_no_default_export_config => (NoDefaultExportRuleConfig) {
            components_only: default(false)
            ;
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
        to_no_orphan_files_config => (NoOrphanFilesRuleConfig) {
            ;
            entry_files: clone_default
        },
        to_no_restricted_imports_config => (NoRestrictedImportsRuleConfig) {
            ;
            restricted: clone_default
        },
        to_no_magic_numbers_config => (NoMagicNumbersRuleConfig) {
            enforce_strings: default(false)
            ;
            allowed_numbers: clone_default
        },
        to_upward_import_config => (UpwardImportRuleConfig) {
            max_depth: default(0)
            ;
            allow_patterns: clone_default
        },
        to_no_empty_domain_config => (NoEmptyDomainRuleConfig) {
            ;
            ignore_dirs: clone_default
        },
        to_no_anemic_domain_config => (NoAnemicDomainRuleConfig) {
            max_files: default(1)
            ;
            ignore_dirs: clone_default
        },
        to_no_god_domain_config => (NoGodDomainRuleConfig) {
            max_files: default(20)
            ;
            ignore_dirs: clone_default
        }
    }

    pub fn to_no_nested_functions_config(&self) -> Result<NoNestedFunctionsRuleConfig, String> {
        match self {
            Self::Severity(severity) => Ok(NoNestedFunctionsRuleConfig {
                severity: severity.parse::<Severity>()?,
                ..NoNestedFunctionsRuleConfig::default()
            }),
            Self::Options(options) => {
                let contexts = match &options.contexts {
                    Some(contexts) => contexts
                        .iter()
                        .map(|c| {
                            c.parse::<NestingContext>()
                                .map_err(|e| format!("in field 'contexts': {e}"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    None => NoNestedFunctionsRuleConfig::default().contexts,
                };
                Ok(NoNestedFunctionsRuleConfig {
                    severity: match options.severity.as_deref() {
                        Some(severity) => severity.parse::<Severity>()?,
                        None => Severity::Warn,
                    },
                    max_depth: options.max_depth.unwrap_or(2),
                    contexts,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawRuleOptions {
    pub severity: Option<String>,
    #[serde(rename = "allow-doc-comments")]
    pub allow_doc_comments: Option<bool>,
    #[serde(rename = "max-lines")]
    pub max_lines: Option<usize>,
    #[serde(rename = "max-exports")]
    pub max_exports: Option<usize>,
    #[serde(rename = "count-default")]
    pub count_default: Option<bool>,
    #[serde(rename = "max-params")]
    pub max_params: Option<usize>,
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
    #[serde(rename = "components-only")]
    pub components_only: Option<bool>,
    #[serde(rename = "entry-files")]
    pub entry_files: Option<Vec<String>>,
    #[serde(rename = "allowed-folders")]
    pub allowed_folders: Option<Vec<String>>,
    #[serde(rename = "extra-abbreviations")]
    pub extra_abbreviations: Option<Vec<String>>,
    #[serde(rename = "allow-abbreviations")]
    pub allow_abbreviations: Option<Vec<String>>,
    pub restricted: Option<Vec<String>>,
    #[serde(rename = "allowed-numbers")]
    pub allowed_numbers: Option<Vec<String>>,
    #[serde(rename = "enforce-strings")]
    pub enforce_strings: Option<bool>,
    #[serde(rename = "max-files")]
    pub max_files: Option<usize>,
    pub contexts: Option<Vec<String>>,
}

impl RawRuleOptions {
    pub fn merge(parent: &RawRuleOptions, child: &RawRuleOptions) -> RawRuleOptions {
        RawRuleOptions {
            severity: child.severity.clone().or_else(|| parent.severity.clone()),
            allow_doc_comments: child.allow_doc_comments.or(parent.allow_doc_comments),
            max_lines: child.max_lines.or(parent.max_lines),
            max_exports: child.max_exports.or(parent.max_exports),
            count_default: child.count_default.or(parent.count_default),
            max_params: child.max_params.or(parent.max_params),
            max_depth: child.max_depth.or(parent.max_depth),
            allow_patterns: child
                .allow_patterns
                .clone()
                .or_else(|| parent.allow_patterns.clone()),
            ignore_dirs: child
                .ignore_dirs
                .clone()
                .or_else(|| parent.ignore_dirs.clone()),
            ignore_names: child
                .ignore_names
                .clone()
                .or_else(|| parent.ignore_names.clone()),
            max_items: child.max_items.or(parent.max_items),
            min_items: child.min_items.or(parent.min_items),
            count_folders: child.count_folders.or(parent.count_folders),
            allow_declaration_merging: child
                .allow_declaration_merging
                .or(parent.allow_declaration_merging),
            extra_names: child
                .extra_names
                .clone()
                .or_else(|| parent.extra_names.clone()),
            prefixes: child.prefixes.clone().or_else(|| parent.prefixes.clone()),
            ignore_constants: child.ignore_constants.or(parent.ignore_constants),
            components_only: child.components_only.or(parent.components_only),
            entry_files: child
                .entry_files
                .clone()
                .or_else(|| parent.entry_files.clone()),
            allowed_folders: child
                .allowed_folders
                .clone()
                .or_else(|| parent.allowed_folders.clone()),
            extra_abbreviations: child
                .extra_abbreviations
                .clone()
                .or_else(|| parent.extra_abbreviations.clone()),
            allow_abbreviations: child
                .allow_abbreviations
                .clone()
                .or_else(|| parent.allow_abbreviations.clone()),
            restricted: child
                .restricted
                .clone()
                .or_else(|| parent.restricted.clone()),
            allowed_numbers: child
                .allowed_numbers
                .clone()
                .or_else(|| parent.allowed_numbers.clone()),
            enforce_strings: child.enforce_strings.or(parent.enforce_strings),
            max_files: child.max_files.or(parent.max_files),
            contexts: child.contexts.clone().or_else(|| parent.contexts.clone()),
        }
    }
}

declare_raw_rules! {
    simple {
        component_file_only_components => "component-file-only-components",
        directory_must_have_barrel => "directory-must-have-barrel",
        hook_no_jsx => "hook-no-jsx",
        no_barrel_chain => "no-barrel-chain",
        no_circular_import => "no-circular-import",
        no_barrel_files => "no-barrel-files",
        no_debugger => "no-debugger",
        no_side_effect_imports => "no-side-effect-imports",
        no_enums => "no-enums",
        explicit_return_type => "explicit-return-type",
        no_eval => "no-eval",
        no_export_star => "no-export-star",
        no_focused_test => "no-focused-test",
        no_inline_types => "no-inline-types",
        no_logic_in_barrel => "no-logic-in-barrel",
        no_logic_in_domain => "no-logic-in-domain",
        no_mutable_exports => "no-mutable-exports",
        no_namespace => "no-namespace",
        no_non_null_assertion => "no-non-null-assertion",
        no_unsafe_optional_chaining => "no-unsafe-optional-chaining",
        no_process_env => "no-process-env",
        no_silent_catch => "no-silent-catch",
        no_skipped_test => "no-skipped-test",
        no_test_code_in_production => "no-test-code-in-production",
        no_package_cycle => "no-package-cycle",
        no_private_package_import => "no-private-package-import",
        no_test_import => "no-test-import",
        no_then_chain => "no-then-chain",
        no_type_assertion => "no-type-assertion",
        prefer_readonly => "prefer-readonly",
    }
    custom_default {
        layer_boundaries => ("layer-boundaries", Severity::Off),
        no_empty_interface => ("no-empty-interface", Severity::Error),
        prefer_satisfies => ("prefer-satisfies", Severity::Info),
    }
    custom {
        boolean_prefix => ("boolean-prefix", to_boolean_prefix_config, BooleanPrefixRuleConfig),
        entry_file_no_logic => ("entry-file-no-logic", to_entry_file_no_logic_config, EntryFileNoLogicRuleConfig),
        hook_prefix => ("hook-prefix", to_hook_prefix_config, HookPrefixRuleConfig),
        max_directory_depth => ("max-directory-depth", to_max_directory_depth_config, MaxDirectoryDepthRuleConfig),
        max_file_exports => ("max-file-exports", to_file_exports_config, FileExportsRuleConfig),
        max_function_params => ("max-function-params", to_max_function_params_config, MaxFunctionParamsRuleConfig),
        max_items_per_directory => ("max-items-per-directory", to_max_items_per_directory_config, MaxItemsPerDirectoryRuleConfig),
        min_items_per_directory => ("min-items-per-directory", to_min_items_per_directory_config, MinItemsPerDirectoryRuleConfig),
        no_abbreviations => ("no-abbreviations", to_no_abbreviations_config, NoAbbreviationsRuleConfig),
        no_any => ("no-any", to_no_any_config, NoAnyRuleConfig),
        no_comments => ("no-comments", to_comments_config, CommentsRuleConfig),
        no_console => ("no-console", to_no_console_config, NoConsoleRuleConfig),
        no_default_export => ("no-default-export", to_no_default_export_config, NoDefaultExportRuleConfig),
        no_dump_files => ("no-dump-files", to_no_dump_files_config, NoDumpFilesRuleConfig),
        no_duplicate_file_names => ("no-duplicate-file-names", to_no_duplicate_file_names_config, NoDuplicateFileNamesRuleConfig),
        no_empty_directories => ("no-empty-directories", to_no_empty_directories_config, NoEmptyDirectoriesRuleConfig),
        no_interface => ("no-interface", to_no_interface_config, NoInterfaceRuleConfig),
        no_large_file => ("no-large-file", to_file_length_config, FileLengthRuleConfig),
        no_magic_numbers => ("no-magic-numbers", to_no_magic_numbers_config, NoMagicNumbersRuleConfig),
        no_nested_functions => ("no-nested-functions", to_no_nested_functions_config, NoNestedFunctionsRuleConfig),
        no_orphan_files => ("no-orphan-files", to_no_orphan_files_config, NoOrphanFilesRuleConfig),
        no_restricted_imports => ("no-restricted-imports", to_no_restricted_imports_config, NoRestrictedImportsRuleConfig),
        no_upward_import => ("no-upward-import", to_upward_import_config, UpwardImportRuleConfig),
        no_empty_domain => ("no-empty-domain", to_no_empty_domain_config, NoEmptyDomainRuleConfig),
        no_anemic_domain => ("no-anemic-domain", to_no_anemic_domain_config, NoAnemicDomainRuleConfig),
        no_god_domain => ("no-god-domain", to_no_god_domain_config, NoGodDomainRuleConfig),
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::RawConfig;

    #[test]
    fn rules_config_rejects_invalid_severity() {
        let raw: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "warning"
"#,
        )
        .expect("valid toml");

        let error = raw
            .rules_config()
            .expect_err("invalid severity should fail");
        assert!(error.contains("'warning'"));
        assert!(error.contains("no-console"));
        assert!(error.contains("off, info, warn, error"));
    }
}
