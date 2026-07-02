use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use oxc_parser::Parser;

use crate::allocator::with_reusable_allocator;
use crate::config;
use crate::ignore;
use crate::import_graph::ImportGraph;
use crate::rule_adapters::*;
use crate::rules::{AstContext, FileRuleSet, GraphContext, RulesConfig, TextContext, Violation};
use crate::syntax::LineIndex;

type FileResult = (
    Vec<Violation>,
    Option<ignore::FileSuppressionInfo>,
    Option<(PathBuf, String)>,
);

fn compute_suppression_info(
    file: &Path,
    violations: &[Violation],
    directives: &[ignore::IgnoreDirective],
) -> Option<ignore::FileSuppressionInfo> {
    if directives.is_empty() {
        return None;
    }

    let suppressed_count = violations
        .iter()
        .filter(|violation| {
            ignore::should_suppress_violation(directives, violation.line, violation.rule)
        })
        .count();

    let stale_directives = directives
        .iter()
        .filter(|directive| {
            !violations
                .iter()
                .any(|violation| directive.should_suppress(violation.line, violation.rule))
        })
        .cloned()
        .collect();

    Some(ignore::FileSuppressionInfo {
        file: file.to_path_buf(),
        suppressed_count,
        stale_directives,
    })
}

pub fn check_files_with_parallelism(
    files: &[PathBuf],
    config_set: &config::ConfigSet,
    import_graph: Arc<ImportGraph>,
    workspace: Option<Arc<crate::workspace::Workspace>>,
    cached_violations: &HashMap<PathBuf, Vec<Violation>>,
    parallel: bool,
) -> Result<(
    Vec<Violation>,
    ignore::SuppressionReport,
    HashMap<PathBuf, String>,
)> {
    let mut violations = Vec::new();
    let mut suppression_files = Vec::new();
    let mut parse_failures: HashMap<PathBuf, String> = HashMap::new();

    // Build rule sets once per config; dense ids keep dispatch allocation-free.
    let config_count = config_set.configs().count();
    let mut grouped: Vec<Vec<&PathBuf>> = vec![Vec::new(); config_count];
    for file in files {
        let (config_id, _) = config_set.config_with_id_for_file(file);
        grouped[config_id].push(file);
    }

    struct ConfigRuntime {
        rules: Arc<FileRuleSet>,
        type_location_style: crate::rules::TypeLocationStyle,
    }
    let mut runtime_by_config: Vec<Option<ConfigRuntime>> =
        (0..config_count).map(|_| None).collect();

    for (config_id, group_files) in grouped.iter().enumerate() {
        if group_files.is_empty() {
            continue;
        }
        let first_file = group_files.first().context("empty config group")?;
        let (_, config) = config_set.config_with_id_for_file(first_file);
        let rules = Arc::new(build_file_rules(
            &config.rules,
            &config.structure,
            &config.architecture,
            import_graph.clone(),
            workspace.clone(),
        ));
        let any_enabled = any_rule_enabled(&rules);
        if !any_enabled {
            continue;
        }
        let file_refs: Vec<PathBuf> = group_files.iter().map(|file| (*file).clone()).collect();
        let type_location_style =
            crate::rules::TypeLocationStyle::detect(&file_refs, &config.structure.types);
        runtime_by_config[config_id] = Some(ConfigRuntime {
            rules,
            type_location_style,
        });
    }

    let process_file = |file: &PathBuf| -> Result<FileResult> {
        let (config_id, _) = config_set.config_with_id_for_file(file);
        let Some(runtime) = runtime_by_config.get(config_id).and_then(Option::as_ref) else {
            return Ok((Vec::new(), None, None));
        };
        let rules = runtime.rules.clone();
        let type_location_style = runtime.type_location_style;

        let source = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let directives = ignore::parse_ignore_directives(&source);

        if let Some(cached) = cached_violations.get(file) {
            let suppression_info = compute_suppression_info(file, cached, &directives);
            let kept: Vec<Violation> = cached
                .iter()
                .filter(|violation| {
                    !ignore::should_suppress_violation(&directives, violation.line, violation.rule)
                })
                .cloned()
                .collect();
            return Ok((kept, suppression_info, None));
        }

        let (new_violations, parse_failure) = with_reusable_allocator(|allocator| {
            let line_index = LineIndex::new(&source);
            let needs_ast = rules
                .ast_rules
                .iter()
                .any(|rule| rule.severity().is_enabled());

            let parse_result: Option<oxc_ast::ast::Program<'_>> = if needs_ast {
                match crate::syntax::source_type_from_path(file) {
                    Some(source_type) => {
                        let parser_return = Parser::new(allocator, &source, source_type).parse();
                        if parser_return.panicked {
                            return (Vec::new(), Some((file.clone(), "parse error".to_string())));
                        }
                        Some(parser_return.program)
                    }
                    None => None,
                }
            } else {
                None
            };

            let mut violations = Vec::new();

            let text_ctx = TextContext {
                file,
                source: &source,
                line_index: &line_index,
                type_location_style,
            };
            for rule in rules.text_rules.iter() {
                if rule.severity().is_enabled() {
                    violations.extend(rule.check(&text_ctx));
                }
            }

            let graph_ctx = GraphContext {
                file,
                line_index: &line_index,
                type_location_style,
                import_graph: import_graph.clone(),
                workspace: workspace.clone(),
            };
            for rule in rules.graph_rules.iter() {
                if rule.severity().is_enabled() {
                    violations.extend(rule.check(&graph_ctx));
                }
            }

            if let Some(program) = parse_result.as_ref() {
                let ast_ctx = AstContext {
                    file,
                    source: &source,
                    program,
                    line_index: &line_index,
                    type_location_style,
                };
                for rule in rules.ast_rules.iter() {
                    if rule.severity().is_enabled() {
                        violations.extend(rule.check(&ast_ctx));
                    }
                }
            }

            (violations, None)
        });

        if let Some(parse_failure) = parse_failure {
            return Ok((Vec::new(), None, Some(parse_failure)));
        }

        let suppression_info = compute_suppression_info(file, &new_violations, &directives);
        let mut file_violations = new_violations;
        file_violations.retain(|violation| {
            !ignore::should_suppress_violation(&directives, violation.line, violation.rule)
        });

        Ok((file_violations, suppression_info, None))
    };

    let file_results: Vec<Result<_>> = if parallel {
        files.par_iter().map(&process_file).collect()
    } else {
        files.iter().map(&process_file).collect()
    };

    for result in file_results {
        let (file_violations, suppression_info, parse_failure) = result?;
        violations.extend(file_violations);
        if let Some(info) = suppression_info {
            suppression_files.push(info);
        }
        if let Some((path, message)) = parse_failure {
            parse_failures.insert(path, message);
        }
    }

    Ok((
        violations,
        ignore::SuppressionReport {
            files: suppression_files,
        },
        parse_failures,
    ))
}

fn any_rule_enabled(rules: &FileRuleSet) -> bool {
    rules
        .ast_rules
        .iter()
        .any(|rule| rule.severity().is_enabled())
        || rules
            .text_rules
            .iter()
            .any(|rule| rule.severity().is_enabled())
        || rules
            .graph_rules
            .iter()
            .any(|rule| rule.severity().is_enabled())
}

pub fn check_files(
    files: &[PathBuf],
    config_set: &config::ConfigSet,
    import_graph: Arc<ImportGraph>,
    workspace: Option<Arc<crate::workspace::Workspace>>,
    cached_violations: &HashMap<PathBuf, Vec<Violation>>,
) -> Result<(
    Vec<Violation>,
    ignore::SuppressionReport,
    HashMap<PathBuf, String>,
)> {
    check_files_with_parallelism(
        files,
        config_set,
        import_graph,
        workspace,
        cached_violations,
        true,
    )
}

/// Run `check_files` against a freshly-resolved project configuration.
///
/// This is exposed for the `parallelism_benchmark` so it can measure the same
/// workload with `parallel` set to `false` (single-threaded) and `true`
/// (multi-threaded). It builds the import graph, workspace, and config set once
/// per call, matching the real analysis flow.
pub fn check_files_for_benchmark(
    project_root: &Path,
    files: &[PathBuf],
    parallel: bool,
) -> Result<Vec<Violation>> {
    let config_set = crate::config::ConfigSet::resolve(
        project_root,
        crate::config::ConfigSetOptions {
            root_override: None,
            scan_scope: None,
            deny_child_configs: false,
        },
    )?;
    let tsconfig = crate::tsconfig::discover_and_parse(project_root)?;
    let graph = Arc::new(crate::import_graph::build_import_graph(
        files,
        |file| {
            config_set
                .config_for_file(file)
                .structure
                .tests
                .matches_file(file)
        },
        tsconfig.as_ref(),
    )?);
    let workspace = crate::workspace::Workspace::discover(project_root)
        .ok()
        .map(Arc::new);
    let cached_violations: HashMap<PathBuf, Vec<Violation>> = HashMap::new();
    let (violations, _, _) = check_files_with_parallelism(
        files,
        &config_set,
        graph,
        workspace,
        &cached_violations,
        parallel,
    )?;
    Ok(violations)
}

pub fn build_file_rules(
    config: &RulesConfig,
    structure: &config::structure::ProjectStructureConfig,
    architecture: &config::architecture::ArchitectureConfig,
    import_graph: Arc<ImportGraph>,
    workspace: Option<Arc<crate::workspace::Workspace>>,
) -> FileRuleSet {
    let mut ast_rules: Vec<Box<dyn crate::rules::AstRule + Send + Sync>> = vec![
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
        Box::new(NoEnumsAdapter {
            config: config.no_enums.clone(),
        }),
        Box::new(NoDebuggerAdapter {
            config: config.no_debugger.clone(),
        }),
        Box::new(NoEvalAdapter {
            config: config.no_eval.clone(),
        }),
        Box::new(NoSideEffectImportsAdapter {
            config: config.no_side_effect_imports.clone(),
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
        Box::new(NoAwaitInLoopAdapter {
            config: config.no_await_in_loop.clone(),
        }),
        Box::new(NoPromiseExecutorReturnAdapter {
            config: config.no_promise_executor_return.clone(),
        }),
        Box::new(NoUnsafeOptionalChainingAdapter {
            config: config.no_unsafe_optional_chaining.clone(),
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
        Box::new(NoBarrelFilesAdapter {
            config: config.no_barrel_files.clone(),
        }),
        Box::new(PreferSatisfiesAdapter {
            config: config.prefer_satisfies.clone(),
        }),
        Box::new(PreferReadonlyAdapter {
            config: config.prefer_readonly.clone(),
        }),
    ];

    ast_rules.push(Box::new(NoLogicInDomainAdapter {
        config: config.no_logic_in_domain.clone(),
        types: structure.types.clone(),
        constants: structure.constants.clone(),
    }));

    let text_rules: Vec<Box<dyn crate::rules::TextRule + Send + Sync>> =
        vec![Box::new(NoLargeFileAdapter {
            config: config.no_large_file.clone(),
        })];

    let mut graph_rules: Vec<Box<dyn crate::rules::GraphRule + Send + Sync>> = vec![
        Box::new(NoUpwardImportAdapter {
            config: config.no_upward_import.clone(),
        }),
        Box::new(LayerBoundariesAdapter {
            config: config.layer_boundaries.clone(),
            layers: architecture.layers.clone(),
        }),
        Box::new(NoTestImportAdapter {
            config: config.no_test_import.clone(),
            tests: structure.tests.clone(),
        }),
        Box::new(NoBarrelChainAdapter {
            config: config.no_barrel_chain.clone(),
        }),
        Box::new(NoCircularImportAdapter {
            config: config.no_circular_import.clone(),
            context: crate::rules::no_circular_import::CircularImportContext::new(
                import_graph.as_ref(),
            ),
        }),
        Box::new(NoOrphanFilesAdapter {
            config: config.no_orphan_files.clone(),
        }),
        Box::new(NoPrivatePackageImportAdapter {
            config: config.no_private_package_import.clone(),
        }),
    ];

    if let Some(workspace) = workspace {
        graph_rules.push(Box::new(NoPackageCycleAdapter {
            config: config.no_package_cycle.clone(),
            context: crate::rules::no_package_cycle::PackageCycleContext::new(
                workspace.as_ref(),
                import_graph.as_ref(),
            ),
        }));
    }

    FileRuleSet {
        ast_rules,
        text_rules,
        graph_rules,
    }
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
