use crate::config::{ProjectConfig, Severity};

use super::catalog::RuleKind;

#[derive(Debug, Clone)]
pub(crate) struct RuleConfigSummary {
    pub(crate) severity: Severity,
    pub(crate) options: Vec<String>,
}

pub(crate) fn summarize_rule(config: &ProjectConfig, kind: RuleKind) -> RuleConfigSummary {
    match kind {
        RuleKind::BooleanPrefix => RuleConfigSummary {
            severity: config.rules.boolean_prefix.severity,
            options: vec![
                format!("prefixes: {:?}", config.rules.boolean_prefix.prefixes),
                format!(
                    "ignore-constants: {}",
                    config.rules.boolean_prefix.ignore_constants
                ),
            ],
        },
        RuleKind::ComponentFileOnlyComponents => RuleConfigSummary {
            severity: config.rules.component_file_only_components.severity,
            options: vec![
                format!("folders: {:?}", config.structure.components.folders),
                format!(
                    "file-suffixes: {:?}",
                    config.structure.components.file_suffixes
                ),
            ],
        },
        RuleKind::HookNoJsx => RuleConfigSummary {
            severity: config.rules.hook_no_jsx.severity,
            options: vec![
                format!("folders: {:?}", config.structure.hooks.folders),
                format!("file-suffixes: {:?}", config.structure.hooks.file_suffixes),
            ],
        },
        RuleKind::HookPrefix => RuleConfigSummary {
            severity: config.rules.hook_prefix.severity,
            options: vec![
                format!("prefixes: {:?}", config.rules.hook_prefix.prefixes),
                format!("folders: {:?}", config.structure.hooks.folders),
                format!("file-suffixes: {:?}", config.structure.hooks.file_suffixes),
            ],
        },
        RuleKind::MaxDirectoryDepth => RuleConfigSummary {
            severity: config.rules.max_directory_depth.severity,
            options: vec![
                format!("max-depth: {}", config.rules.max_directory_depth.max_depth),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.max_directory_depth.ignore_dirs
                ),
            ],
        },
        RuleKind::MaxFileExports => RuleConfigSummary {
            severity: config.rules.max_file_exports.severity,
            options: vec![format!(
                "max-exports: {}",
                config.rules.max_file_exports.max_exports
            )],
        },
        RuleKind::MaxFunctionParams => RuleConfigSummary {
            severity: config.rules.max_function_params.severity,
            options: vec![format!(
                "max-params: {}",
                config.rules.max_function_params.max_params
            )],
        },
        RuleKind::MaxItemsPerDirectory => RuleConfigSummary {
            severity: config.rules.max_items_per_directory.severity,
            options: vec![
                format!(
                    "max-items: {}",
                    config.rules.max_items_per_directory.max_items
                ),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.max_items_per_directory.ignore_dirs
                ),
                format!(
                    "count-folders: {}",
                    config.rules.max_items_per_directory.count_folders
                ),
            ],
        },
        RuleKind::MinItemsPerDirectory => RuleConfigSummary {
            severity: config.rules.min_items_per_directory.severity,
            options: vec![
                format!(
                    "min-items: {}",
                    config.rules.min_items_per_directory.min_items
                ),
                format!(
                    "ignore-dirs: {:?}",
                    config.rules.min_items_per_directory.ignore_dirs
                ),
                format!(
                    "count-folders: {}",
                    config.rules.min_items_per_directory.count_folders
                ),
            ],
        },
        RuleKind::NoBarrelChain => simple_summary(config.rules.no_barrel_chain.severity),
        RuleKind::NoCircularImport => simple_summary(config.rules.no_circular_import.severity),
        RuleKind::NoBarrelFiles => simple_summary(config.rules.no_barrel_files.severity),
        RuleKind::NoComments => RuleConfigSummary {
            severity: config.rules.no_comments.severity,
            options: vec![format!(
                "allow-doc-comments: {}",
                config.rules.no_comments.allow_doc_comments
            )],
        },
        RuleKind::NoConsole => RuleConfigSummary {
            severity: config.rules.no_console.severity,
            options: vec![format!(
                "allow-patterns: {:?}",
                config.rules.no_console.allow_patterns
            )],
        },
        RuleKind::NoDebugger => simple_summary(config.rules.no_debugger.severity),
        RuleKind::NoDefaultExport => RuleConfigSummary {
            severity: config.rules.no_default_export.severity,
            options: vec![format!(
                "components-only: {}",
                config.rules.no_default_export.components_only
            )],
        },
        RuleKind::NoDuplicateFileNames => RuleConfigSummary {
            severity: config.rules.no_duplicate_file_names.severity,
            options: vec![format!(
                "ignore-names: {:?}",
                config.rules.no_duplicate_file_names.ignore_names
            )],
        },
        RuleKind::NoDumpFiles => RuleConfigSummary {
            severity: config.rules.no_dump_files.severity,
            options: vec![format!(
                "extra-names: {:?}",
                config.rules.no_dump_files.extra_names
            )],
        },
        RuleKind::NoEmptyDirectories => RuleConfigSummary {
            severity: config.rules.no_empty_directories.severity,
            options: vec![format!(
                "ignore-dirs: {:?}",
                config.rules.no_empty_directories.ignore_dirs
            )],
        },
        RuleKind::NoEmptyInterface => simple_summary(config.rules.no_empty_interface.severity),
        RuleKind::NoEnums => simple_summary(config.rules.no_enums.severity),
        RuleKind::NoEval => simple_summary(config.rules.no_eval.severity),
        RuleKind::NoExportStar => simple_summary(config.rules.no_export_star.severity),
        RuleKind::NoFocusedTest => simple_summary(config.rules.no_focused_test.severity),
        RuleKind::NoInlineTypes => RuleConfigSummary {
            severity: config.rules.no_inline_types.severity,
            options: vec![
                format!("folders: {:?}", config.structure.types.folders),
                format!("file-suffixes: {:?}", config.structure.types.file_suffixes),
            ],
        },
        RuleKind::NoInterface => RuleConfigSummary {
            severity: config.rules.no_interface.severity,
            options: vec![format!(
                "allow-declaration-merging: {}",
                config.rules.no_interface.allow_declaration_merging
            )],
        },
        RuleKind::NoLargeFile => RuleConfigSummary {
            severity: config.rules.no_large_file.severity,
            options: vec![format!(
                "max-lines: {}",
                config.rules.no_large_file.max_lines
            )],
        },
        RuleKind::NoLogicInBarrel => simple_summary(config.rules.no_logic_in_barrel.severity),
        RuleKind::NoLogicInDomain => RuleConfigSummary {
            severity: config.rules.no_logic_in_domain.severity,
            options: vec![
                format!("types-folders: {:?}", config.structure.types.folders),
                format!(
                    "types-file-suffixes: {:?}",
                    config.structure.types.file_suffixes
                ),
                format!(
                    "constants-folders: {:?}",
                    config.structure.constants.folders
                ),
                format!(
                    "constants-file-suffixes: {:?}",
                    config.structure.constants.file_suffixes
                ),
            ],
        },
        RuleKind::NoMutableExports => simple_summary(config.rules.no_mutable_exports.severity),
        RuleKind::NoNestedFunctions => RuleConfigSummary {
            severity: config.rules.no_nested_functions.severity,
            options: vec![format!(
                "max-depth: {}",
                config.rules.no_nested_functions.max_depth
            )],
        },
        RuleKind::NoOrphanFiles => RuleConfigSummary {
            severity: config.rules.no_orphan_files.severity,
            options: vec![format!(
                "entry-files: {:?}",
                config.rules.no_orphan_files.entry_files
            )],
        },
        RuleKind::NoNamespace => simple_summary(config.rules.no_namespace.severity),
        RuleKind::NoRestrictedImports => RuleConfigSummary {
            severity: config.rules.no_restricted_imports.severity,
            options: vec![format!(
                "restricted: {:?}",
                config.rules.no_restricted_imports.restricted
            )],
        },
        RuleKind::NoSilentCatch => simple_summary(config.rules.no_silent_catch.severity),
        RuleKind::NoSkippedTest => simple_summary(config.rules.no_skipped_test.severity),
        RuleKind::NoTestCodeInProduction => RuleConfigSummary {
            severity: config.rules.no_test_code_in_production.severity,
            options: vec![
                format!("folders: {:?}", config.structure.tests.folders),
                format!("file-suffixes: {:?}", config.structure.tests.file_suffixes),
            ],
        },
        RuleKind::NoThenChain => simple_summary(config.rules.no_then_chain.severity),
        RuleKind::NoUpwardImport => RuleConfigSummary {
            severity: config.rules.no_upward_import.severity,
            options: vec![format!(
                "max-depth: {}",
                config.rules.no_upward_import.max_depth
            )],
        },
        RuleKind::PreferSatisfies => simple_summary(config.rules.prefer_satisfies.severity),
        RuleKind::NoTestImport => RuleConfigSummary {
            severity: config.rules.no_test_import.severity,
            options: vec![
                format!("folders: {:?}", config.structure.tests.folders),
                format!("file-suffixes: {:?}", config.structure.tests.file_suffixes),
            ],
        },
        RuleKind::EntryFileNoLogic => RuleConfigSummary {
            severity: config.rules.entry_file_no_logic.severity,
            options: vec![format!(
                "entry-files: {:?}",
                config.rules.entry_file_no_logic.entry_files
            )],
        },
        RuleKind::ExplicitReturnType => simple_summary(config.rules.explicit_return_type.severity),
        RuleKind::NoNonNullAssertion => simple_summary(config.rules.no_non_null_assertion.severity),
        RuleKind::NoProcessEnv => simple_summary(config.rules.no_process_env.severity),
        RuleKind::NoAbbreviations => RuleConfigSummary {
            severity: config.rules.no_abbreviations.severity,
            options: vec![format!(
                "extra-abbreviations: {:?}",
                config.rules.no_abbreviations.extra_abbreviations
            )],
        },
        RuleKind::NoAny => RuleConfigSummary {
            severity: config.rules.no_any.severity,
            options: vec![
                format!("allowed-folders: {:?}", config.rules.no_any.allowed_folders),
                format!(
                    "generated-folders: {:?}",
                    config.structure.generated.folders
                ),
                format!(
                    "generated-file-suffixes: {:?}",
                    config.structure.generated.file_suffixes
                ),
            ],
        },
        RuleKind::NoTypeAssertion => simple_summary(config.rules.no_type_assertion.severity),
    }
}

fn simple_summary(severity: Severity) -> RuleConfigSummary {
    RuleConfigSummary {
        severity,
        options: Vec::new(),
    }
}
