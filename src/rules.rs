mod boolean_prefix;
mod component_file_only_components;
mod entry_file_no_logic;
mod hook_no_jsx;
mod hook_prefix;
mod max_directory_depth;
mod max_file_exports;
mod max_items_per_directory;
mod min_items_per_directory;
mod no_any;
mod no_barrel_chain;
mod no_barrel_files;
mod no_comments;
mod no_component_default_export;
mod no_console;
mod no_debugger;
mod no_default_export;
mod no_dump_files;
mod no_duplicate_file_names;
mod no_empty_directories;
mod no_empty_interface;
mod no_enums;
mod no_eval;
mod no_export_star;
mod no_inline_types;
mod no_interface;
mod no_large_file;
mod no_logic_in_barrel;
mod no_logic_in_domain;
mod no_mutable_exports;
mod no_namespace;
mod no_non_null_assertion;
mod no_silent_catch;
mod no_test_code_in_production;
mod no_test_import;
mod no_then_chain;
mod no_upward_import;
mod prefer_satisfies;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_parser::Parser;

use crate::config::{
    MaxDirectoryDepthRuleConfig, MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig,
    NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig,
    ProjectConfig, Severity,
};
use crate::ignore;

pub type RuleId = &'static str;

pub const BOOLEAN_PREFIX_RULE_ID: RuleId = "boolean-prefix";
pub const COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID: RuleId = "component-file-only-components";
pub const HOOK_NO_JSX_RULE_ID: RuleId = "hook-no-jsx";
pub const HOOK_PREFIX_RULE_ID: RuleId = "hook-prefix";
pub const MAX_DIRECTORY_DEPTH_RULE_ID: RuleId = "max-directory-depth";
pub const MAX_FILE_EXPORTS_RULE_ID: RuleId = "max-file-exports";
pub const MAX_ITEMS_PER_DIRECTORY_RULE_ID: RuleId = "max-items-per-directory";
pub const MIN_ITEMS_PER_DIRECTORY_RULE_ID: RuleId = "min-items-per-directory";
pub const NO_BARREL_CHAIN_RULE_ID: RuleId = "no-barrel-chain";
pub const NO_BARREL_FILES_RULE_ID: RuleId = "no-barrel-files";
pub const NO_COMMENTS_RULE_ID: RuleId = "no-comments";
pub const NO_CONSOLE_RULE_ID: RuleId = "no-console";
pub const NO_DEBUGGER_RULE_ID: RuleId = "no-debugger";
pub const NO_COMPONENT_DEFAULT_EXPORT_RULE_ID: RuleId = "no-component-default-export";
pub const NO_DEFAULT_EXPORT_RULE_ID: RuleId = "no-default-export";
pub const NO_DUPLICATE_FILE_NAMES_RULE_ID: RuleId = "no-duplicate-file-names";
pub const NO_DUMP_FILES_RULE_ID: RuleId = "no-dump-files";
pub const NO_EMPTY_DIRECTORIES_RULE_ID: RuleId = "no-empty-directories";
pub const NO_EMPTY_INTERFACE_RULE_ID: RuleId = "no-empty-interface";
pub const NO_ENUMS_RULE_ID: RuleId = "no-enums";
pub const NO_EVAL_RULE_ID: RuleId = "no-eval";
pub const NO_EXPORT_STAR_RULE_ID: RuleId = "no-export-star";
pub const NO_INLINE_TYPES_RULE_ID: RuleId = "no-inline-types";
pub const NO_INTERFACE_RULE_ID: RuleId = "no-interface";
pub const NO_LARGE_FILE_RULE_ID: RuleId = "no-large-file";
pub const NO_LOGIC_IN_BARREL_RULE_ID: RuleId = "no-logic-in-barrel";
pub const NO_LOGIC_IN_DOMAIN_RULE_ID: RuleId = "no-logic-in-domain";
pub const NO_MUTABLE_EXPORTS_RULE_ID: RuleId = "no-mutable-exports";
pub const NO_NAMESPACE_RULE_ID: RuleId = "no-namespace";
pub const NO_SILENT_CATCH_RULE_ID: RuleId = "no-silent-catch";
pub const NO_TEST_CODE_IN_PRODUCTION_RULE_ID: RuleId = "no-test-code-in-production";
pub const NO_THEN_CHAIN_RULE_ID: RuleId = "no-then-chain";
pub const NO_UPWARD_IMPORT_RULE_ID: RuleId = "no-upward-import";
pub const PREFER_SATISFIES_RULE_ID: RuleId = "prefer-satisfies";
pub const NO_TEST_IMPORT_RULE_ID: RuleId = "no-test-import";
pub const ENTRY_FILE_NO_LOGIC_RULE_ID: RuleId = "entry-file-no-logic";
pub const NO_NON_NULL_ASSERTION_RULE_ID: RuleId = "no-non-null-assertion";
pub const NO_ANY_RULE_ID: RuleId = "no-any";

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
    no_empty_directories: NoEmptyDirectoriesRuleConfig,
) -> Vec<Violation> {
    if !no_empty_directories.severity.is_enabled() {
        return Vec::new();
    }

    no_empty_directories::check_directories(root, &no_empty_directories)
}

pub fn check_duplicate_file_names(
    files: &[PathBuf],
    no_duplicate_file_names: NoDuplicateFileNamesRuleConfig,
) -> Vec<Violation> {
    if !no_duplicate_file_names.severity.is_enabled() {
        return Vec::new();
    }

    no_duplicate_file_names::check_files(files, &no_duplicate_file_names)
}

pub fn check_max_items_per_directory(
    root: &std::path::Path,
    config: MaxItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_items_per_directory::check_directories(root, &config)
}

pub fn check_min_items_per_directory(
    root: &std::path::Path,
    config: MinItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    min_items_per_directory::check_directories(root, &config)
}

pub fn check_max_directory_depth(
    root: &std::path::Path,
    config: MaxDirectoryDepthRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_directory_depth::check_directories(root, &config)
}

pub fn check_dump_files(files: &[PathBuf], config: NoDumpFilesRuleConfig) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    no_dump_files::check_files(files, &config)
}
