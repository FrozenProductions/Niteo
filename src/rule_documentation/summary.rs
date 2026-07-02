use crate::config::{ProjectConfig, Severity};

pub(crate) type RuleSummaryFn = fn(&ProjectConfig) -> RuleConfigSummary;

#[derive(Debug, Clone)]
pub(crate) struct RuleConfigSummary {
    pub(crate) severity: Severity,
    pub(crate) options: Vec<String>,
}

pub(crate) fn boolean_prefix_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.boolean_prefix.severity,
        options: vec![
            format!("prefixes: {:?}", config.rules.boolean_prefix.prefixes),
            format!(
                "ignore-constants: {}",
                config.rules.boolean_prefix.ignore_constants
            ),
        ],
    }
}

pub(crate) fn component_file_only_components_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.component_file_only_components.severity,
        options: vec![
            format!("folders: {:?}", config.structure.components.folders),
            format!(
                "file-suffixes: {:?}",
                config.structure.components.file_suffixes
            ),
        ],
    }
}

pub(crate) fn directory_must_have_barrel_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.directory_must_have_barrel.severity)
}

pub(crate) fn hook_no_jsx_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.hook_no_jsx.severity,
        options: vec![
            format!("folders: {:?}", config.structure.hooks.folders),
            format!("file-suffixes: {:?}", config.structure.hooks.file_suffixes),
        ],
    }
}

pub(crate) fn layer_boundaries_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.layer_boundaries.severity,
        options: vec![format!("layers: {:?}", config.architecture.layers.order)],
    }
}

pub(crate) fn hook_prefix_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.hook_prefix.severity,
        options: vec![
            format!("prefixes: {:?}", config.rules.hook_prefix.prefixes),
            format!("folders: {:?}", config.structure.hooks.folders),
            format!("file-suffixes: {:?}", config.structure.hooks.file_suffixes),
        ],
    }
}

pub(crate) fn max_directory_depth_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.max_directory_depth.severity,
        options: vec![
            format!("max-depth: {}", config.rules.max_directory_depth.max_depth),
            format!(
                "ignore-dirs: {:?}",
                config.rules.max_directory_depth.ignore_dirs
            ),
        ],
    }
}

pub(crate) fn max_file_exports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.max_file_exports.severity,
        options: vec![format!(
            "max-exports: {}",
            config.rules.max_file_exports.max_exports
        )],
    }
}

pub(crate) fn max_function_params_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.max_function_params.severity,
        options: vec![format!(
            "max-params: {}",
            config.rules.max_function_params.max_params
        )],
    }
}

pub(crate) fn max_items_per_directory_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
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
    }
}

pub(crate) fn min_items_per_directory_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
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
    }
}

pub(crate) fn no_barrel_chain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_barrel_chain.severity)
}

pub(crate) fn no_circular_import_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_circular_import.severity)
}

pub(crate) fn no_barrel_files_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_barrel_files.severity)
}

pub(crate) fn no_comments_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_comments.severity,
        options: vec![format!(
            "allow-doc-comments: {}",
            config.rules.no_comments.allow_doc_comments
        )],
    }
}

pub(crate) fn no_console_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_console.severity,
        options: vec![format!(
            "allow-patterns: {:?}",
            config.rules.no_console.allow_patterns
        )],
    }
}

pub(crate) fn no_debugger_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_debugger.severity)
}

pub(crate) fn no_side_effect_imports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_side_effect_imports.severity)
}

pub(crate) fn sort_imports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.sort_imports.severity)
}

pub(crate) fn no_default_export_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_default_export.severity,
        options: vec![format!(
            "components-only: {}",
            config.rules.no_default_export.components_only
        )],
    }
}

pub(crate) fn no_duplicate_file_names_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_duplicate_file_names.severity,
        options: vec![format!(
            "ignore-names: {:?}",
            config.rules.no_duplicate_file_names.ignore_names
        )],
    }
}

pub(crate) fn no_dump_files_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_dump_files.severity,
        options: vec![format!(
            "extra-names: {:?}",
            config.rules.no_dump_files.extra_names
        )],
    }
}

pub(crate) fn no_empty_directories_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_empty_directories.severity,
        options: vec![format!(
            "ignore-dirs: {:?}",
            config.rules.no_empty_directories.ignore_dirs
        )],
    }
}

pub(crate) fn no_empty_interface_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_empty_interface.severity)
}

pub(crate) fn no_enums_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_enums.severity)
}

pub(crate) fn no_eval_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_eval.severity)
}

pub(crate) fn no_export_star_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_export_star.severity)
}

pub(crate) fn no_focused_test_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_focused_test.severity)
}

pub(crate) fn no_inline_types_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_inline_types.severity,
        options: vec![
            format!("folders: {:?}", config.structure.types.folders),
            format!("file-suffixes: {:?}", config.structure.types.file_suffixes),
        ],
    }
}

pub(crate) fn no_interface_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_interface.severity,
        options: vec![format!(
            "allow-declaration-merging: {}",
            config.rules.no_interface.allow_declaration_merging
        )],
    }
}

pub(crate) fn no_large_file_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_large_file.severity,
        options: vec![format!(
            "max-lines: {}",
            config.rules.no_large_file.max_lines
        )],
    }
}

pub(crate) fn no_logic_in_barrel_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_logic_in_barrel.severity)
}

pub(crate) fn no_logic_in_domain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
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
    }
}

pub(crate) fn no_mutable_exports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_mutable_exports.severity)
}

pub(crate) fn sort_exports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.sort_exports.severity)
}

pub(crate) fn no_nested_functions_summary(config: &ProjectConfig) -> RuleConfigSummary {
    let contexts: Vec<String> = config
        .rules
        .no_nested_functions
        .contexts
        .iter()
        .map(|c| c.to_string())
        .collect();
    RuleConfigSummary {
        severity: config.rules.no_nested_functions.severity,
        options: vec![
            format!("max-depth: {}", config.rules.no_nested_functions.max_depth),
            format!("contexts: [{}]", contexts.join(", ")),
        ],
    }
}

pub(crate) fn no_orphan_files_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_orphan_files.severity,
        options: vec![format!(
            "entry-files: {:?}",
            config.rules.no_orphan_files.entry_files
        )],
    }
}

pub(crate) fn no_package_cycle_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_package_cycle.severity)
}

pub(crate) fn no_private_package_import_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_private_package_import.severity)
}

pub(crate) fn no_namespace_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_namespace.severity)
}

pub(crate) fn no_restricted_imports_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_restricted_imports.severity,
        options: vec![format!(
            "restricted: {:?}",
            config.rules.no_restricted_imports.restricted
        )],
    }
}

pub(crate) fn no_silent_catch_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_silent_catch.severity)
}

pub(crate) fn no_skipped_test_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_skipped_test.severity)
}

pub(crate) fn no_test_code_in_production_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_test_code_in_production.severity,
        options: vec![
            format!("folders: {:?}", config.structure.tests.folders),
            format!("file-suffixes: {:?}", config.structure.tests.file_suffixes),
        ],
    }
}

pub(crate) fn no_then_chain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_then_chain.severity)
}

pub(crate) fn no_upward_import_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_upward_import.severity,
        options: vec![format!(
            "max-depth: {}",
            config.rules.no_upward_import.max_depth
        )],
    }
}

pub(crate) fn prefer_satisfies_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.prefer_satisfies.severity)
}

pub(crate) fn prefer_readonly_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.prefer_readonly.severity)
}

pub(crate) fn no_test_import_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_test_import.severity,
        options: vec![
            format!("folders: {:?}", config.structure.tests.folders),
            format!("file-suffixes: {:?}", config.structure.tests.file_suffixes),
        ],
    }
}

pub(crate) fn entry_file_no_logic_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.entry_file_no_logic.severity,
        options: vec![format!(
            "entry-files: {:?}",
            config.rules.entry_file_no_logic.entry_files
        )],
    }
}

pub(crate) fn explicit_return_type_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.explicit_return_type.severity)
}

pub(crate) fn no_non_null_assertion_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_non_null_assertion.severity)
}

pub(crate) fn no_unsafe_optional_chaining_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_unsafe_optional_chaining.severity)
}

pub(crate) fn no_await_in_loop_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_await_in_loop.severity)
}

pub(crate) fn no_promise_executor_return_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_promise_executor_return.severity)
}

pub(crate) fn no_magic_numbers_summary(config: &ProjectConfig) -> RuleConfigSummary {
    let mut options = vec![format!(
        "allowed-numbers: {:?}",
        config.rules.no_magic_numbers.allowed_numbers
    )];
    if config.rules.no_magic_numbers.enforce_strings {
        options.push("enforce-strings: true".to_string());
    }
    RuleConfigSummary {
        severity: config.rules.no_magic_numbers.severity,
        options,
    }
}

pub(crate) fn no_process_env_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_process_env.severity)
}

pub(crate) fn no_abbreviations_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_abbreviations.severity,
        options: vec![format!(
            "extra-abbreviations: {:?}",
            config.rules.no_abbreviations.extra_abbreviations
        )],
    }
}

pub(crate) fn no_any_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
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
    }
}

pub(crate) fn no_type_assertion_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_type_assertion.severity)
}

pub(crate) fn no_unnecessary_type_assertion_summary(config: &ProjectConfig) -> RuleConfigSummary {
    simple_summary(config.rules.no_unnecessary_type_assertion.severity)
}

pub(crate) fn no_empty_domain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_empty_domain.severity,
        options: vec![format!(
            "ignore-dirs: {:?}",
            config.rules.no_empty_domain.ignore_dirs
        )],
    }
}

pub(crate) fn no_anemic_domain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_anemic_domain.severity,
        options: vec![
            format!("max-files: {}", config.rules.no_anemic_domain.max_files),
            format!(
                "ignore-dirs: {:?}",
                config.rules.no_anemic_domain.ignore_dirs
            ),
        ],
    }
}

pub(crate) fn no_god_domain_summary(config: &ProjectConfig) -> RuleConfigSummary {
    RuleConfigSummary {
        severity: config.rules.no_god_domain.severity,
        options: vec![
            format!("max-files: {}", config.rules.no_god_domain.max_files),
            format!("ignore-dirs: {:?}", config.rules.no_god_domain.ignore_dirs),
        ],
    }
}

fn simple_summary(severity: Severity) -> RuleConfigSummary {
    RuleConfigSummary {
        severity,
        options: Vec::new(),
    }
}
