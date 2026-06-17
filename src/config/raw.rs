use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::architecture::{ArchitectureConfig, LayerBoundaryConfig};
use super::rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, EntryFileNoLogicRuleConfig, FileExportsRuleConfig,
    FileLengthRuleConfig, GitignoreConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxFunctionParamsRuleConfig, MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig,
    NoAbbreviationsRuleConfig, NoAnemicDomainRuleConfig, NoAnyRuleConfig, NoConsoleRuleConfig,
    NoDefaultExportRuleConfig, NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig,
    NoEmptyDirectoriesRuleConfig, NoEmptyDomainRuleConfig, NoGodDomainRuleConfig,
    NoInterfaceRuleConfig, NoMagicNumbersRuleConfig, NoNestedFunctionsRuleConfig,
    NoOrphanFilesRuleConfig, NoRestrictedImportsRuleConfig, RuleConfig, Severity,
    UpwardImportRuleConfig,
};
use super::structure::{DomainConfig, ProjectStructureConfig};
use crate::rules::RulesConfig;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RawConfig {
    pub project: Option<RawProjectConfig>,
    pub architecture: Option<RawArchitectureConfig>,
    pub rules: Option<HashMap<String, RawRuleConfig>>,
}

impl RawConfig {
    pub fn merge(parent: &RawConfig, child: &RawConfig) -> RawConfig {
        RawConfig {
            project: Self::merge_project(parent.project.as_ref(), child.project.as_ref()),
            architecture: Self::merge_architecture(
                parent.architecture.as_ref(),
                child.architecture.as_ref(),
            ),
            rules: Self::merge_rules(parent.rules.as_ref(), child.rules.as_ref()),
        }
    }

    fn merge_project(
        parent: Option<&RawProjectConfig>,
        child: Option<&RawProjectConfig>,
    ) -> Option<RawProjectConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawProjectConfig {
                root: parent.root.clone(),
                respect_gitignore: child.respect_gitignore.or(parent.respect_gitignore),
                structure: Self::merge_structure(
                    parent.structure.as_ref(),
                    child.structure.as_ref(),
                ),
            }),
        }
    }

    fn merge_architecture(
        parent: Option<&RawArchitectureConfig>,
        child: Option<&RawArchitectureConfig>,
    ) -> Option<RawArchitectureConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawArchitectureConfig {
                layers: RawLayerBoundaryConfig::merge(
                    parent.layers.as_ref(),
                    child.layers.as_ref(),
                ),
            }),
        }
    }

    fn merge_structure(
        parent: Option<&RawProjectStructure>,
        child: Option<&RawProjectStructure>,
    ) -> Option<RawProjectStructure> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawProjectStructure {
                hooks: child.hooks.clone().or_else(|| parent.hooks.clone()),
                components: child
                    .components
                    .clone()
                    .or_else(|| parent.components.clone()),
                types: child.types.clone().or_else(|| parent.types.clone()),
                constants: child.constants.clone().or_else(|| parent.constants.clone()),
                tests: child.tests.clone().or_else(|| parent.tests.clone()),
                generated: child.generated.clone().or_else(|| parent.generated.clone()),
            }),
        }
    }

    fn merge_rules(
        parent: Option<&HashMap<String, RawRuleConfig>>,
        child: Option<&HashMap<String, RawRuleConfig>>,
    ) -> Option<HashMap<String, RawRuleConfig>> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => {
                let mut merged = parent.clone();
                for (name, child_rule) in child {
                    match merged.get(name) {
                        Some(parent_rule) => {
                            merged.insert(
                                name.clone(),
                                RawRuleConfig::merge(parent_rule, child_rule),
                            );
                        }
                        None => {
                            merged.insert(name.clone(), child_rule.clone());
                        }
                    }
                }
                Some(merged)
            }
        }
    }
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

declare_raw_rules! {
    simple {
        component_file_only_components => "component-file-only-components",
        directory_must_have_barrel => "directory-must-have-barrel",
        hook_no_jsx => "hook-no-jsx",
        no_barrel_chain => "no-barrel-chain",
        no_circular_import => "no-circular-import",
        no_barrel_files => "no-barrel-files",
        no_debugger => "no-debugger",
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

impl RawConfig {
    pub fn architecture(&self) -> ArchitectureConfig {
        let raw_arch = self.architecture.as_ref();
        let layers = raw_arch
            .and_then(|arch| arch.layers.as_ref())
            .map(|raw| raw.to_layer_boundary_config())
            .unwrap_or_default();

        ArchitectureConfig { layers }
    }

    pub fn gitignore(&self) -> GitignoreConfig {
        let project = self.project.as_ref();
        GitignoreConfig {
            enabled: project
                .and_then(|project| project.respect_gitignore)
                .unwrap_or_default(),
        }
    }

    pub fn structure(&self) -> ProjectStructureConfig {
        let raw_structure = self
            .project
            .as_ref()
            .and_then(|project| project.structure.as_ref());

        let defaults = ProjectStructureConfig::default();

        ProjectStructureConfig {
            hooks: raw_structure
                .and_then(|structure| structure.hooks.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.hooks))
                .unwrap_or(defaults.hooks),
            components: raw_structure
                .and_then(|structure| structure.components.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.components))
                .unwrap_or(defaults.components),
            types: raw_structure
                .and_then(|structure| structure.types.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.types))
                .unwrap_or(defaults.types),
            constants: raw_structure
                .and_then(|structure| structure.constants.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.constants))
                .unwrap_or(defaults.constants),
            tests: raw_structure
                .and_then(|structure| structure.tests.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.tests))
                .unwrap_or(defaults.tests),
            generated: raw_structure
                .and_then(|structure| structure.generated.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.generated))
                .unwrap_or(defaults.generated),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProjectConfig {
    pub root: Option<PathBuf>,
    #[serde(rename = "respect-gitignore")]
    pub respect_gitignore: Option<bool>,
    pub structure: Option<RawProjectStructure>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProjectStructure {
    pub hooks: Option<RawDomainConfig>,
    pub components: Option<RawDomainConfig>,
    pub types: Option<RawDomainConfig>,
    pub constants: Option<RawDomainConfig>,
    pub tests: Option<RawDomainConfig>,
    pub generated: Option<RawDomainConfig>,
}

#[derive(Debug, Clone, Deserialize)]
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
            pub fn $method(&self) -> Result<$config_type, String> {
                match self {
                    Self::Severity(severity) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = Severity::from_str(severity)?;
                        Ok(cfg)
                    }
                    Self::Options(options) => {
                        let mut cfg = <$config_type>::default();
                        cfg.severity = match options.severity.as_deref() {
                            Some(severity) => Severity::from_str(severity)?,
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

// Enables TOML shorthand: `[rules.no-console]` can be `"warn"` or `{ severity = "warn", allow-patterns = [...] }`
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawRuleConfig {
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
                severity: Severity::from_str(severity)?,
            }),
            Self::Options(options) => Ok(RuleConfig {
                severity: match options.severity.as_deref() {
                    Some(severity) => Severity::from_str(severity)?,
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
            max_exports: default(10)
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
            extra_abbreviations: clone_default
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
        to_no_nested_functions_config => (NoNestedFunctionsRuleConfig) {
            max_depth: default(2)
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
            ;
            allowed_numbers: clone_default
        },
        to_upward_import_config => (UpwardImportRuleConfig) {
            max_depth: default(0)
            ;
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawRuleOptions {
    pub severity: Option<String>,
    #[serde(rename = "allow-doc-comments")]
    pub allow_doc_comments: Option<bool>,
    #[serde(rename = "max-lines")]
    pub max_lines: Option<usize>,
    #[serde(rename = "max-exports")]
    pub max_exports: Option<usize>,
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
    pub restricted: Option<Vec<String>>,
    #[serde(rename = "allowed-numbers")]
    pub allowed_numbers: Option<Vec<String>>,
    #[serde(rename = "max-files")]
    pub max_files: Option<usize>,
}

impl RawRuleOptions {
    pub fn merge(parent: &RawRuleOptions, child: &RawRuleOptions) -> RawRuleOptions {
        RawRuleOptions {
            severity: child.severity.clone().or_else(|| parent.severity.clone()),
            allow_doc_comments: child.allow_doc_comments.or(parent.allow_doc_comments),
            max_lines: child.max_lines.or(parent.max_lines),
            max_exports: child.max_exports.or(parent.max_exports),
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
            restricted: child
                .restricted
                .clone()
                .or_else(|| parent.restricted.clone()),
            allowed_numbers: child
                .allowed_numbers
                .clone()
                .or_else(|| parent.allowed_numbers.clone()),
            max_files: child.max_files.or(parent.max_files),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawArchitectureConfig {
    pub layers: Option<RawLayerBoundaryConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawLayerBoundaryConfig {
    pub order: Option<Vec<String>>,
    #[serde(flatten)]
    pub definitions: HashMap<String, RawDomainConfig>,
}

impl RawLayerBoundaryConfig {
    fn merge(
        parent: Option<&RawLayerBoundaryConfig>,
        child: Option<&RawLayerBoundaryConfig>,
    ) -> Option<RawLayerBoundaryConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => {
                let order = child.order.clone().or_else(|| parent.order.clone());
                let mut definitions = parent.definitions.clone();
                for (name, child_def) in &child.definitions {
                    definitions.insert(name.clone(), child_def.clone());
                }
                Some(RawLayerBoundaryConfig { order, definitions })
            }
        }
    }

    fn to_layer_boundary_config(&self) -> LayerBoundaryConfig {
        let order = self.order.clone().unwrap_or_default();
        let mut definitions: HashMap<String, DomainConfig> = HashMap::new();

        for name in &order {
            if let Some(raw_domain) = self.definitions.get(name) {
                let domain = raw_domain.to_domain_config(&DomainConfig {
                    folders: vec![],
                    file_suffixes: vec![],
                });
                definitions.insert(name.clone(), domain);
            }
        }

        LayerBoundaryConfig { order, definitions }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::{Context, Result};

    #[test]
    fn default_config_parses_architecture() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["app", "features", "entities", "shared"]

[architecture.layers.app]
folders = ["app"]

[architecture.layers.features]
folders = ["features"]

[architecture.layers.entities]
folders = ["entities"]

[architecture.layers.shared]
folders = ["shared"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert_eq!(
            arch.layers.order,
            vec!["app", "features", "entities", "shared"]
        );
        assert_eq!(arch.layers.definitions.len(), 4);
        assert_eq!(
            arch.layers
                .definitions
                .get("shared")
                .context("expected shared layer")?
                .folders,
            vec!["shared"]
        );
        Ok(())
    }

    #[test]
    fn missing_architecture_uses_defaults() -> Result<()> {
        let source = r#"
[project]
root = "src"
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert!(arch.layers.order.is_empty());
        Ok(())
    }

    #[test]
    fn architecture_layer_order_only_defines_layers() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["shared", "core"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert_eq!(arch.layers.order, vec!["shared", "core"]);
        assert!(arch.layers.definitions.is_empty());
        Ok(())
    }

    #[test]
    fn architecture_layer_with_suffixes() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["features"]

[architecture.layers.features]
folders = ["features"]
file-suffixes = [".feature.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        let def = arch
            .layers
            .definitions
            .get("features")
            .context("expected features layer")?;
        assert_eq!(def.folders, vec!["features"]);
        assert_eq!(def.file_suffixes, vec![".feature.ts"]);
        Ok(())
    }

    #[test]
    fn default_config_parses_structure() -> Result<()> {
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
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".hook.ts", ".hooks.ts"]);
        assert_eq!(structure.components.folders, vec!["components"]);
        assert_eq!(structure.types.folders, vec!["types"]);
        assert_eq!(structure.constants.folders, vec!["constants"]);
        assert_eq!(structure.tests.folders, vec!["tests"]);
        assert_eq!(structure.tests.file_suffixes, vec![".test.ts", ".tests.ts"]);
        Ok(())
    }

    #[test]
    fn missing_structure_uses_defaults() -> Result<()> {
        let source = r#"
[project]
root = "src"
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let structure = raw.structure();
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.hooks.folders, defaults.hooks.folders);
        assert_eq!(structure.hooks.file_suffixes, defaults.hooks.file_suffixes);
        assert_eq!(structure.components.folders, defaults.components.folders);
        assert_eq!(structure.types.folders, defaults.types.folders);
        assert_eq!(structure.constants.folders, defaults.constants.folders);
        assert_eq!(structure.tests.folders, defaults.tests.folders);
        assert_eq!(structure.tests.file_suffixes, defaults.tests.file_suffixes);
        Ok(())
    }

    #[test]
    fn custom_structure_overrides_defaults() -> Result<()> {
        let source = r#"
[project.structure.hooks]
folders = ["custom-hooks"]
file-suffixes = [".custom.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["custom-hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".custom.ts"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.components.folders, defaults.components.folders);
        Ok(())
    }

    #[test]
    fn partial_domain_config_preserves_defaults() -> Result<()> {
        let source = r#"
[project.structure.types]
folders = ["typings"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let structure = raw.structure();
        assert_eq!(structure.types.folders, vec!["typings"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.types.file_suffixes, defaults.types.file_suffixes);
        Ok(())
    }

    #[test]
    fn merge_child_severity_overrides_parent_options() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "warn"
allow-patterns = ["debug"]
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "error"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_console.severity, Severity::Error);
        Ok(())
    }

    #[test]
    fn merge_child_options_partial_override() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-large-file]
severity = "warn"
max-lines = 300
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-large-file]
severity = "error"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_large_file.severity, Severity::Error);
        assert_eq!(rules_config.no_large_file.max_lines, 300);
        Ok(())
    }

    #[test]
    fn merge_inherits_parent_rules_not_in_child() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "warn"
[rules.no-debugger]
severity = "error"
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "off"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_console.severity, Severity::Off);
        assert_eq!(rules_config.no_debugger.severity, Severity::Error);
        Ok(())
    }

    #[test]
    fn merge_structure_child_overrides_parent_domain() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts"]
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
folders = ["__tests__"]
file-suffixes = [".spechild.ts"]
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let structure = merged.structure();
        assert_eq!(structure.tests.folders, vec!["__tests__"]);
        assert_eq!(structure.tests.file_suffixes, vec![".spechild.ts"]);
        Ok(())
    }

    #[test]
    fn merge_structure_preserves_parent_root() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[project]
root = "src"
respect-gitignore = true
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[project]
respect-gitignore = false
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let project = merged.project.as_ref().context("expected project")?;
        assert_eq!(project.root, Some(PathBuf::from("src")));
        assert_eq!(project.respect_gitignore, Some(false));
        Ok(())
    }

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
