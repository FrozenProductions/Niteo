use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_parser::Parser;

use crate::config::{self, ProjectConfig};
use crate::ignore;
use crate::import_graph::ImportGraph;
use crate::rule_adapters::*;
use crate::rules::{FileContext, FileRule, RulesConfig, Violation};
use crate::syntax::LineIndex;

pub fn check_files(
    files: &[PathBuf],
    config_set: &config::ConfigSet,
    import_graph: &ImportGraph,
    workspace: Option<&crate::workspace::Workspace>,
) -> Result<(Vec<Violation>, ignore::SuppressionReport)> {
    let mut violations = Vec::new();
    let mut suppression_files = Vec::new();

    // Group files by config pointer identity so rules are built once per unique config
    let mut grouped: std::collections::HashMap<usize, Vec<&PathBuf>> =
        std::collections::HashMap::new();
    for file in files {
        let config = config_set.config_for_file(file);
        let config_ptr = config as *const ProjectConfig as usize;
        grouped.entry(config_ptr).or_default().push(file);
    }

    for group_files in grouped.values() {
        let first_file = match group_files.first() {
            Some(f) => f,
            None => continue,
        };
        let config = config_set.config_for_file(first_file);
        let rules = build_file_rules(
            &config.rules,
            &config.structure,
            &config.architecture,
            import_graph,
            workspace,
        );

        let any_enabled = rules.iter().any(|rule| rule.severity().is_enabled());
        if !any_enabled {
            continue;
        }

        let needs_ast = rules
            .iter()
            .any(|rule| rule.severity().is_enabled() && rule.needs_ast());

        let file_refs: Vec<PathBuf> = group_files.iter().map(|file| (*file).clone()).collect();
        let type_location_style = crate::rules::no_inline_types::TypeLocationStyle::detect(
            &file_refs,
            &config.structure.types,
        );

        for file in group_files {
            let source = std::fs::read_to_string(*file)
                .with_context(|| format!("failed to read {}", file.display()))?;

            let directives = ignore::parse_ignore_directives(&source);
            let line_index = LineIndex::new(&source);

            let allocator = Allocator::default();
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

            let ctx = FileContext {
                file,
                source: &source,
                program: parse_result.as_ref(),
                line_index: &line_index,
                type_location_style,
                import_graph,
                workspace,
            };

            let mut file_violations = Vec::new();
            for rule in &rules {
                if rule.severity().is_enabled() {
                    file_violations.extend(rule.check(&ctx));
                }
            }

            let suppressed_count = file_violations
                .iter()
                .filter(|violation| {
                    ignore::should_suppress_violation(&directives, violation.line, violation.rule)
                })
                .count();

            let stale_directives: Vec<ignore::IgnoreDirective> = directives
                .iter()
                .filter(|directive| {
                    !file_violations
                        .iter()
                        .any(|violation| directive.should_suppress(violation.line, violation.rule))
                })
                .cloned()
                .collect();

            if !directives.is_empty() {
                suppression_files.push(ignore::FileSuppressionInfo {
                    file: (*file).clone(),
                    suppressed_count,
                    stale_directives,
                });
            }

            file_violations.retain(|violation| {
                !ignore::should_suppress_violation(&directives, violation.line, violation.rule)
            });

            violations.extend(file_violations);
        }
    }

    Ok((
        violations,
        ignore::SuppressionReport {
            files: suppression_files,
        },
    ))
}

fn build_file_rules(
    config: &RulesConfig,
    structure: &config::structure::ProjectStructureConfig,
    architecture: &config::architecture::ArchitectureConfig,
    import_graph: &ImportGraph,
    workspace: Option<&crate::workspace::Workspace>,
) -> Vec<Box<dyn FileRule>> {
    vec![
        Box::new(BooleanPrefixAdapter {
            config: config.boolean_prefix.clone(),
        }),
        Box::new(NoConsoleAdapter {
            config: config.no_console.clone(),
        }),
        Box::new(NoDefaultExportAdapter {
            config: config.no_default_export.clone(),
            components: structure.components.clone(),
        }),
        Box::new(NoExportStarAdapter {
            config: config.no_export_star.clone(),
        }),
        Box::new(NoFocusedTestAdapter {
            config: config.no_focused_test.clone(),
        }),
        Box::new(MaxFileExportsAdapter {
            config: config.max_file_exports.clone(),
        }),
        Box::new(MaxFunctionParamsAdapter {
            config: config.max_function_params.clone(),
        }),
        Box::new(NoUpwardImportAdapter {
            config: config.no_upward_import.clone(),
        }),
        Box::new(NoEnumsAdapter {
            config: config.no_enums.clone(),
        }),
        Box::new(NoDebuggerAdapter {
            config: config.no_debugger.clone(),
        }),
        Box::new(NoEvalAdapter {
            config: config.no_eval.clone(),
        }),
        Box::new(NoEmptyInterfaceAdapter {
            config: config.no_empty_interface.clone(),
        }),
        Box::new(NoInterfaceAdapter {
            config: config.no_interface.clone(),
        }),
        Box::new(NoMutableExportsAdapter {
            config: config.no_mutable_exports.clone(),
        }),
        Box::new(NoNamespaceAdapter {
            config: config.no_namespace.clone(),
        }),
        Box::new(NoSilentCatchAdapter {
            config: config.no_silent_catch.clone(),
        }),
        Box::new(NoSkippedTestAdapter {
            config: config.no_skipped_test.clone(),
        }),
        Box::new(NoThenChainAdapter {
            config: config.no_then_chain.clone(),
        }),
        Box::new(EntryFileNoLogicAdapter {
            config: config.entry_file_no_logic.clone(),
        }),
        Box::new(ExplicitReturnTypeAdapter {
            config: config.explicit_return_type.clone(),
        }),
        Box::new(NoNonNullAssertionAdapter {
            config: config.no_non_null_assertion.clone(),
        }),
        Box::new(NoMagicNumbersAdapter {
            config: config.no_magic_numbers.clone(),
        }),
        Box::new(NoTypeAssertionAdapter {
            config: config.no_type_assertion.clone(),
        }),
        Box::new(NoProcessEnvAdapter {
            config: config.no_process_env.clone(),
        }),
        Box::new(NoAbbreviationsAdapter {
            config: config.no_abbreviations.clone(),
        }),
        Box::new(NoRestrictedImportsAdapter {
            config: config.no_restricted_imports.clone(),
        }),
        Box::new(NoInlineTypesAdapter {
            config: config.no_inline_types.clone(),
            types: structure.types.clone(),
        }),
        Box::new(HookNoJsxAdapter {
            config: config.hook_no_jsx.clone(),
            hooks: structure.hooks.clone(),
        }),
        Box::new(HookPrefixAdapter {
            config: config.hook_prefix.clone(),
            hooks: structure.hooks.clone(),
        }),
        Box::new(ComponentFileOnlyComponentsAdapter {
            config: config.component_file_only_components.clone(),
            components: structure.components.clone(),
        }),
        Box::new(NoTestCodeInProductionAdapter {
            config: config.no_test_code_in_production.clone(),
            tests: structure.tests.clone(),
        }),
        Box::new(NoTestImportAdapter {
            config: config.no_test_import.clone(),
            tests: structure.tests.clone(),
        }),
        Box::new(NoAnyAdapter {
            config: config.no_any.clone(),
            generated: structure.generated.clone(),
        }),
        Box::new(NoNestedFunctionsAdapter {
            config: config.no_nested_functions.clone(),
        }),
        Box::new(NoCommentsAdapter {
            config: config.no_comments.clone(),
        }),
        Box::new(NoLogicInBarrelAdapter {
            config: config.no_logic_in_barrel.clone(),
        }),
        Box::new(NoLargeFileAdapter {
            config: config.no_large_file.clone(),
        }),
        Box::new(NoBarrelFilesAdapter {
            config: config.no_barrel_files.clone(),
        }),
        Box::new(PreferSatisfiesAdapter {
            config: config.prefer_satisfies.clone(),
        }),
        Box::new(PreferReadonlyAdapter {
            config: config.prefer_readonly.clone(),
        }),
        Box::new(NoBarrelChainAdapter {
            config: config.no_barrel_chain.clone(),
        }),
        Box::new(NoCircularImportAdapter {
            config: config.no_circular_import.clone(),
            context: crate::rules::no_circular_import::CircularImportContext::new(import_graph),
        }),
        Box::new(NoOrphanFilesAdapter {
            config: config.no_orphan_files.clone(),
        }),
        Box::new(NoPrivatePackageImportAdapter {
            config: config.no_private_package_import.clone(),
        }),
        Box::new(NoPackageCycleAdapter {
            config: config.no_package_cycle.clone(),
            context: crate::rules::no_package_cycle::PackageCycleContext::new(
                workspace.unwrap_or(&crate::workspace::Workspace {
                    root: std::path::PathBuf::new(),
                    packages: vec![],
                }),
                import_graph,
            ),
        }),
        Box::new(NoLogicInDomainAdapter {
            config: config.no_logic_in_domain.clone(),
            types: structure.types.clone(),
            constants: structure.constants.clone(),
        }),
        Box::new(LayerBoundariesAdapter {
            config: config.layer_boundaries.clone(),
            layers: architecture.layers.clone(),
        }),
    ]
}

pub fn check_duplicate_file_names(
    files: &[PathBuf],
    no_duplicate_file_names: crate::config::NoDuplicateFileNamesRuleConfig,
) -> Vec<Violation> {
    if !no_duplicate_file_names.severity.is_enabled() {
        return Vec::new();
    }

    crate::rules::no_duplicate_file_names::check_files(files, &no_duplicate_file_names)
}

pub fn check_dump_files(
    files: &[PathBuf],
    config: crate::config::NoDumpFilesRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    crate::rules::no_dump_files::check_files(files, &config)
}

pub fn check_directory_rules(
    root: &Path,
    rules: &RulesConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let inventory = crate::directory_inventory::collect_directory_inventory(root, exclude_dirs);
    let mut violations = Vec::new();

    if rules.no_empty_directories.severity.is_enabled() {
        violations.extend(crate::rules::no_empty_directories::check_inventory(
            &inventory,
            &rules.no_empty_directories,
        ));
    }

    if rules.directory_must_have_barrel.severity.is_enabled() {
        violations.extend(crate::rules::directory_must_have_barrel::check_inventory(
            &inventory,
            &rules.directory_must_have_barrel,
        ));
    }

    if rules.max_items_per_directory.severity.is_enabled() {
        violations.extend(crate::rules::max_items_per_directory::check_inventory(
            &inventory,
            &rules.max_items_per_directory,
        ));
    }

    if rules.min_items_per_directory.severity.is_enabled() {
        violations.extend(crate::rules::min_items_per_directory::check_inventory(
            &inventory,
            &rules.min_items_per_directory,
        ));
    }

    if rules.max_directory_depth.severity.is_enabled() {
        violations.extend(crate::rules::max_directory_depth::check_inventory(
            &inventory,
            &rules.max_directory_depth,
        ));
    }

    if rules.no_empty_domain.severity.is_enabled() {
        violations.extend(crate::rules::no_empty_domain::check_inventory(
            &inventory,
            &rules.no_empty_domain,
        ));
    }

    if rules.no_anemic_domain.severity.is_enabled() {
        violations.extend(crate::rules::no_anemic_domain::check_inventory(
            &inventory,
            &rules.no_anemic_domain,
        ));
    }

    if rules.no_god_domain.severity.is_enabled() {
        violations.extend(crate::rules::no_god_domain::check_inventory(
            &inventory,
            &rules.no_god_domain,
        ));
    }

    violations
}
