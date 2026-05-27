macro_rules! declare_rules {
    (
        $( $mod_name:ident => { id: $rule_id:ident, value: $rule_value:literal, config: $config_type:ty $(, default_severity: $default_sev:expr )? } ),* $(,)?
    ) => {
        $( mod $mod_name; )*

        use anyhow::{Context, Result};
        use std::fs;
        use std::path::PathBuf;

        use oxc_allocator::Allocator;
        use oxc_parser::Parser;

        use crate::config::{ProjectConfig, Severity};
        use crate::ignore;

        pub type RuleId = &'static str;

        $( pub const $rule_id: RuleId = $rule_value; )*

        #[derive(Debug, Clone)]
        pub struct Violation {
            pub file: PathBuf,
            pub line: Option<usize>,
            pub column: Option<usize>,
            pub rule: RuleId,
            pub message: &'static str,
            pub severity: Severity,
            pub detail: Option<String>,
            pub subject: Option<String>,
        }

        #[derive(Debug, Clone)]
        pub struct RulesConfig {
            $( pub $mod_name: $config_type, )*
        }

        impl Default for RulesConfig {
            fn default() -> Self {
                Self {
                    $(
                        $mod_name: {
                            let mut cfg = <$config_type>::default();
                            cfg.severity = declare_rules!(@sev $($default_sev)?);
                            cfg
                        },
                    )*
                }
            }
        }
    };

    (@sev) => { Severity::Warn };
    (@sev $sev:expr) => { $sev };
}

declare_rules! {
    boolean_prefix => { id: BOOLEAN_PREFIX_RULE_ID, value: "boolean-prefix", config: crate::config::BooleanPrefixRuleConfig },
    component_file_only_components => { id: COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID, value: "component-file-only-components", config: crate::config::RuleConfig },
    entry_file_no_logic => { id: ENTRY_FILE_NO_LOGIC_RULE_ID, value: "entry-file-no-logic", config: crate::config::EntryFileNoLogicRuleConfig },
    hook_no_jsx => { id: HOOK_NO_JSX_RULE_ID, value: "hook-no-jsx", config: crate::config::RuleConfig },
    hook_prefix => { id: HOOK_PREFIX_RULE_ID, value: "hook-prefix", config: crate::config::HookPrefixRuleConfig },
    max_directory_depth => { id: MAX_DIRECTORY_DEPTH_RULE_ID, value: "max-directory-depth", config: crate::config::MaxDirectoryDepthRuleConfig },
    max_file_exports => { id: MAX_FILE_EXPORTS_RULE_ID, value: "max-file-exports", config: crate::config::FileExportsRuleConfig },
    max_items_per_directory => { id: MAX_ITEMS_PER_DIRECTORY_RULE_ID, value: "max-items-per-directory", config: crate::config::MaxItemsPerDirectoryRuleConfig },
    min_items_per_directory => { id: MIN_ITEMS_PER_DIRECTORY_RULE_ID, value: "min-items-per-directory", config: crate::config::MinItemsPerDirectoryRuleConfig },
    no_any => { id: NO_ANY_RULE_ID, value: "no-any", config: crate::config::NoAnyRuleConfig },
    no_barrel_chain => { id: NO_BARREL_CHAIN_RULE_ID, value: "no-barrel-chain", config: crate::config::RuleConfig },
    no_barrel_files => { id: NO_BARREL_FILES_RULE_ID, value: "no-barrel-files", config: crate::config::RuleConfig },
    no_comments => { id: NO_COMMENTS_RULE_ID, value: "no-comments", config: crate::config::CommentsRuleConfig },
    no_component_default_export => { id: NO_COMPONENT_DEFAULT_EXPORT_RULE_ID, value: "no-component-default-export", config: crate::config::RuleConfig },
    no_console => { id: NO_CONSOLE_RULE_ID, value: "no-console", config: crate::config::NoConsoleRuleConfig },
    no_debugger => { id: NO_DEBUGGER_RULE_ID, value: "no-debugger", config: crate::config::RuleConfig },
    no_default_export => { id: NO_DEFAULT_EXPORT_RULE_ID, value: "no-default-export", config: crate::config::RuleConfig },
    no_dump_files => { id: NO_DUMP_FILES_RULE_ID, value: "no-dump-files", config: crate::config::NoDumpFilesRuleConfig },
    no_duplicate_file_names => { id: NO_DUPLICATE_FILE_NAMES_RULE_ID, value: "no-duplicate-file-names", config: crate::config::NoDuplicateFileNamesRuleConfig },
    no_empty_directories => { id: NO_EMPTY_DIRECTORIES_RULE_ID, value: "no-empty-directories", config: crate::config::NoEmptyDirectoriesRuleConfig },
    no_empty_interface => { id: NO_EMPTY_INTERFACE_RULE_ID, value: "no-empty-interface", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_enums => { id: NO_ENUMS_RULE_ID, value: "no-enums", config: crate::config::RuleConfig },
    no_eval => { id: NO_EVAL_RULE_ID, value: "no-eval", config: crate::config::RuleConfig },
    no_export_star => { id: NO_EXPORT_STAR_RULE_ID, value: "no-export-star", config: crate::config::RuleConfig },
    no_inline_types => { id: NO_INLINE_TYPES_RULE_ID, value: "no-inline-types", config: crate::config::RuleConfig },
    no_interface => { id: NO_INTERFACE_RULE_ID, value: "no-interface", config: crate::config::NoInterfaceRuleConfig },
    no_large_file => { id: NO_LARGE_FILE_RULE_ID, value: "no-large-file", config: crate::config::FileLengthRuleConfig },
    no_logic_in_barrel => { id: NO_LOGIC_IN_BARREL_RULE_ID, value: "no-logic-in-barrel", config: crate::config::RuleConfig },
    no_logic_in_domain => { id: NO_LOGIC_IN_DOMAIN_RULE_ID, value: "no-logic-in-domain", config: crate::config::RuleConfig },
    no_mutable_exports => { id: NO_MUTABLE_EXPORTS_RULE_ID, value: "no-mutable-exports", config: crate::config::RuleConfig },
    no_namespace => { id: NO_NAMESPACE_RULE_ID, value: "no-namespace", config: crate::config::RuleConfig },
    no_non_null_assertion => { id: NO_NON_NULL_ASSERTION_RULE_ID, value: "no-non-null-assertion", config: crate::config::RuleConfig },
    no_silent_catch => { id: NO_SILENT_CATCH_RULE_ID, value: "no-silent-catch", config: crate::config::RuleConfig },
    no_test_code_in_production => { id: NO_TEST_CODE_IN_PRODUCTION_RULE_ID, value: "no-test-code-in-production", config: crate::config::RuleConfig },
    no_test_import => { id: NO_TEST_IMPORT_RULE_ID, value: "no-test-import", config: crate::config::RuleConfig },
    no_then_chain => { id: NO_THEN_CHAIN_RULE_ID, value: "no-then-chain", config: crate::config::RuleConfig },
    no_upward_import => { id: NO_UPWARD_IMPORT_RULE_ID, value: "no-upward-import", config: crate::config::UpwardImportRuleConfig },
    prefer_satisfies => { id: PREFER_SATISFIES_RULE_ID, value: "prefer-satisfies", config: crate::config::RuleConfig, default_severity: Severity::Info },
}

pub fn check_files(
    files: &[PathBuf],
    config: &ProjectConfig,
) -> Result<(Vec<Violation>, ignore::SuppressionReport)> {
    let mut violations = Vec::new();

    let needs_scan = config.rules.no_comments.severity.is_enabled()
        || config.rules.no_logic_in_barrel.severity.is_enabled()
        || config.rules.no_large_file.severity.is_enabled()
        || config.rules.no_barrel_files.severity.is_enabled()
        || config.rules.no_barrel_chain.severity.is_enabled()
        || config.rules.no_logic_in_domain.severity.is_enabled()
        || config.rules.prefer_satisfies.severity.is_enabled();

    let needs_ast = config.rules.no_default_export.severity.is_enabled()
        || config
            .rules
            .no_component_default_export
            .severity
            .is_enabled()
        || config.rules.boolean_prefix.severity.is_enabled()
        || config.rules.no_export_star.severity.is_enabled()
        || config.rules.no_inline_types.severity.is_enabled()
        || config.rules.max_file_exports.severity.is_enabled()
        || config.rules.no_upward_import.severity.is_enabled()
        || config.rules.no_enums.severity.is_enabled()
        || config.rules.no_console.severity.is_enabled()
        || config.rules.no_debugger.severity.is_enabled()
        || config.rules.no_eval.severity.is_enabled()
        || config.rules.no_empty_interface.severity.is_enabled()
        || config.rules.no_interface.severity.is_enabled()
        || config.rules.no_mutable_exports.severity.is_enabled()
        || config.rules.no_namespace.severity.is_enabled()
        || config.rules.no_silent_catch.severity.is_enabled()
        || config
            .rules
            .no_test_code_in_production
            .severity
            .is_enabled()
        || config.rules.no_then_chain.severity.is_enabled()
        || config.rules.hook_no_jsx.severity.is_enabled()
        || config
            .rules
            .component_file_only_components
            .severity
            .is_enabled()
        || config.rules.no_test_import.severity.is_enabled()
        || config.rules.entry_file_no_logic.severity.is_enabled()
        || config.rules.no_non_null_assertion.severity.is_enabled()
        || config.rules.no_any.severity.is_enabled();

    let mut suppression_files = Vec::new();

    if !needs_scan && !needs_ast {
        return Ok((
            violations,
            ignore::SuppressionReport {
                files: suppression_files,
            },
        ));
    }

    let type_location_style =
        no_inline_types::TypeLocationStyle::detect(files, &config.structure.types);

    for file in files {
        let source = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let directives = ignore::parse_ignore_directives(&source);
        let mut file_violations = Vec::new();

        let allocator = Allocator::default();
        let line_index = crate::syntax::LineIndex::new(&source);
        let parse_result: Option<oxc_ast::ast::Program<'_>> = if needs_ast {
            match crate::syntax::source_type_from_path(file) {
                Some(source_type) => {
                    let parser_return = Parser::new(&allocator, &source, source_type).parse();
                    if parser_return.panicked {
                        None
                    } else {
                        Some(parser_return.program)
                    }
                }
                None => None,
            }
        } else {
            None
        };

        if let Some(ref program) = parse_result {
            if config.rules.boolean_prefix.severity.is_enabled() {
                file_violations.extend(boolean_prefix::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.boolean_prefix,
                ));
            }
            if config.rules.no_default_export.severity.is_enabled() {
                file_violations.extend(no_default_export::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_default_export,
                ));
            }
            if config
                .rules
                .no_component_default_export
                .severity
                .is_enabled()
            {
                file_violations.extend(no_component_default_export::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_component_default_export,
                    &config.structure.components,
                ));
            }
            if config.rules.no_export_star.severity.is_enabled() {
                file_violations.extend(no_export_star::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_export_star,
                ));
            }
            if config.rules.no_inline_types.severity.is_enabled() {
                file_violations.extend(no_inline_types::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_inline_types,
                    type_location_style,
                    &config.structure.types,
                ));
            }
            if config.rules.max_file_exports.severity.is_enabled() {
                file_violations.extend(max_file_exports::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.max_file_exports,
                ));
            }
            if config.rules.no_upward_import.severity.is_enabled() {
                file_violations.extend(no_upward_import::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_upward_import,
                ));
            }
            if config.rules.no_enums.severity.is_enabled() {
                file_violations.extend(no_enums::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_enums,
                ));
            }
            if config.rules.no_console.severity.is_enabled() {
                file_violations.extend(no_console::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_console,
                ));
            }
            if config.rules.no_debugger.severity.is_enabled() {
                file_violations.extend(no_debugger::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_debugger,
                ));
            }
            if config.rules.no_eval.severity.is_enabled() {
                file_violations.extend(no_eval::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_eval,
                ));
            }
            if config.rules.no_empty_interface.severity.is_enabled() {
                file_violations.extend(no_empty_interface::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_empty_interface,
                ));
            }
            if config.rules.no_interface.severity.is_enabled() {
                file_violations.extend(no_interface::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_interface,
                ));
            }
            if config.rules.no_mutable_exports.severity.is_enabled() {
                file_violations.extend(no_mutable_exports::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_mutable_exports,
                ));
            }
            if config.rules.no_namespace.severity.is_enabled() {
                file_violations.extend(no_namespace::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_namespace,
                ));
            }
            if config.rules.no_silent_catch.severity.is_enabled() {
                file_violations.extend(no_silent_catch::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_silent_catch,
                ));
            }
            if config
                .rules
                .no_test_code_in_production
                .severity
                .is_enabled()
            {
                file_violations.extend(no_test_code_in_production::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_test_code_in_production,
                    &config.structure.tests,
                ));
            }
            if config.rules.no_then_chain.severity.is_enabled() {
                file_violations.extend(no_then_chain::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_then_chain,
                ));
            }
            if config.rules.hook_no_jsx.severity.is_enabled() {
                file_violations.extend(hook_no_jsx::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.hook_no_jsx,
                    &config.structure.hooks,
                ));
            }
            if config.rules.hook_prefix.severity.is_enabled() {
                file_violations.extend(hook_prefix::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.hook_prefix,
                    &config.structure.hooks,
                ));
            }
            if config
                .rules
                .component_file_only_components
                .severity
                .is_enabled()
            {
                file_violations.extend(component_file_only_components::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.component_file_only_components,
                    &config.structure.components,
                ));
            }
            if config.rules.no_test_import.severity.is_enabled() {
                file_violations.extend(no_test_import::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_test_import,
                    &config.structure.tests,
                ));
            }
            if config.rules.entry_file_no_logic.severity.is_enabled() {
                file_violations.extend(entry_file_no_logic::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.entry_file_no_logic,
                ));
            }
            if config.rules.no_non_null_assertion.severity.is_enabled() {
                file_violations.extend(no_non_null_assertion::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_non_null_assertion,
                ));
            }
            if config.rules.no_any.severity.is_enabled() {
                file_violations.extend(no_any::check_file(
                    file,
                    program,
                    &line_index,
                    &config.rules.no_any,
                    &config.structure.generated,
                ));
            }
        }
        if config.rules.no_comments.severity.is_enabled() {
            file_violations.extend(no_comments::check_file(
                file,
                &source,
                &config.rules.no_comments,
            ));
        }
        if config.rules.no_logic_in_barrel.severity.is_enabled() {
            file_violations.extend(no_logic_in_barrel::check_file(
                file,
                &source,
                &config.rules.no_logic_in_barrel,
            ));
        }
        if config.rules.no_large_file.severity.is_enabled() {
            file_violations.extend(no_large_file::check_file(
                file,
                &source,
                &config.rules.no_large_file,
            ));
        }
        if config.rules.no_barrel_files.severity.is_enabled() {
            file_violations.extend(no_barrel_files::check_file(
                file,
                &source,
                &config.rules.no_barrel_files,
            ));
        }
        if config.rules.no_barrel_chain.severity.is_enabled() {
            file_violations.extend(no_barrel_chain::check_file(
                file,
                &source,
                files,
                &config.rules.no_barrel_chain,
            ));
        }
        if config.rules.no_logic_in_domain.severity.is_enabled() {
            file_violations.extend(no_logic_in_domain::check_file(
                file,
                &source,
                &config.rules.no_logic_in_domain,
                &config.structure.types,
                &config.structure.constants,
            ));
        }
        if config.rules.prefer_satisfies.severity.is_enabled() {
            file_violations.extend(prefer_satisfies::check_file(
                file,
                &source,
                &config.rules.prefer_satisfies,
            ));
        }

        let suppressed_count = file_violations
            .iter()
            .filter(|v| ignore::should_suppress_violation(&directives, v.line, v.rule))
            .count();

        let stale_directives: Vec<ignore::IgnoreDirective> = directives
            .iter()
            .filter(|d| {
                !file_violations
                    .iter()
                    .any(|v| d.should_suppress(v.line, v.rule))
            })
            .cloned()
            .collect();

        if !directives.is_empty() {
            suppression_files.push(ignore::FileSuppressionInfo {
                file: file.clone(),
                suppressed_count,
                stale_directives,
            });
        }

        file_violations.retain(|v| !ignore::should_suppress_violation(&directives, v.line, v.rule));

        violations.extend(file_violations);
    }

    Ok((
        violations,
        ignore::SuppressionReport {
            files: suppression_files,
        },
    ))
}

pub fn check_directories(
    root: &std::path::Path,
    no_empty_directories: crate::config::NoEmptyDirectoriesRuleConfig,
) -> Vec<Violation> {
    if !no_empty_directories.severity.is_enabled() {
        return Vec::new();
    }

    no_empty_directories::check_directories(root, &no_empty_directories)
}

pub fn check_duplicate_file_names(
    files: &[PathBuf],
    no_duplicate_file_names: crate::config::NoDuplicateFileNamesRuleConfig,
) -> Vec<Violation> {
    if !no_duplicate_file_names.severity.is_enabled() {
        return Vec::new();
    }

    no_duplicate_file_names::check_files(files, &no_duplicate_file_names)
}

pub fn check_max_items_per_directory(
    root: &std::path::Path,
    config: crate::config::MaxItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_items_per_directory::check_directories(root, &config)
}

pub fn check_min_items_per_directory(
    root: &std::path::Path,
    config: crate::config::MinItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    min_items_per_directory::check_directories(root, &config)
}

pub fn check_max_directory_depth(
    root: &std::path::Path,
    config: crate::config::MaxDirectoryDepthRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_directory_depth::check_directories(root, &config)
}

pub fn check_dump_files(
    files: &[PathBuf],
    config: crate::config::NoDumpFilesRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    no_dump_files::check_files(files, &config)
}
