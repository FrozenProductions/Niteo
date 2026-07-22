use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use oxc_parser::Parser;

use crate::allocator::with_reusable_allocator;
use crate::config;
use crate::directory_inventory::DirectoryInventory;
use crate::ignore;
use crate::import_graph::ImportGraph;
use crate::rule_adapters::*;
use crate::rules::*;
use crate::syntax::with_reusable_line_index;
use std::collections::HashSet;

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

pub struct FileCheckInput<'a> {
    pub files: &'a [PathBuf],
    pub config_set: &'a config::ConfigSet,
    pub import_graph: Arc<ImportGraph>,
    pub workspace: Option<Arc<crate::workspace::Workspace>>,
    pub cached_violations: Arc<HashMap<PathBuf, Vec<Violation>>>,
    pub sources: &'a HashMap<PathBuf, String>,
    pub changed_rules: Arc<HashSet<crate::rules::RuleId>>,
}

pub fn check_files_with_parallelism(
    input: &FileCheckInput<'_>,
    parallel: bool,
    verbose: u8,
) -> Result<(
    Vec<Violation>,
    ignore::SuppressionReport,
    HashMap<PathBuf, String>,
)> {
    let files = input.files;
    let config_set = input.config_set;
    let import_graph = input.import_graph.clone();
    let workspace = input.workspace.clone();
    let cached_violations = input.cached_violations.clone();
    let sources = input.sources;
    let mut violations = Vec::new();
    let mut suppression_files = Vec::new();
    let mut parse_failures: HashMap<PathBuf, String> = HashMap::new();

    // Build rule sets once per config; dense ids keep dispatch allocation-free.
    let config_count = config_set.configs().count();
    let mut grouped: Vec<Vec<&PathBuf>> = vec![Vec::new(); config_count];
    let mut file_config_ids: Vec<usize> = Vec::with_capacity(files.len());
    for file in files {
        let (config_id, _) = config_set.config_with_id_for_file(file);
        grouped[config_id].push(file);
        file_config_ids.push(config_id);
    }

    struct ConfigRuntime {
        rules: Arc<FileRuleSet>,
        changed_rules: Arc<FileRuleSet>,
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
            None,
        ));
        let any_enabled = any_rule_enabled(&rules);
        if !any_enabled {
            continue;
        }
        let changed_rules = Arc::new(build_file_rules(
            &config.rules,
            &config.structure,
            &config.architecture,
            import_graph.clone(),
            workspace.clone(),
            Some(&input.changed_rules),
        ));
        let file_refs: Vec<PathBuf> = group_files.iter().map(|file| (*file).clone()).collect();
        let type_location_style =
            crate::rules::TypeLocationStyle::detect(&file_refs, &config.structure.types);
        runtime_by_config[config_id] = Some(ConfigRuntime {
            rules,
            changed_rules,
            type_location_style,
        });
    }

    let process_file = |file: &PathBuf, config_id: usize| -> Result<FileResult> {
        let Some(runtime) = runtime_by_config.get(config_id).and_then(Option::as_ref) else {
            return Ok((Vec::new(), None, None));
        };
        let type_location_style = runtime.type_location_style;

        let owned_source;
        let source: &str = if let Some(s) = sources.get(file.as_path()) {
            s.as_str()
        } else {
            owned_source = std::fs::read_to_string(file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            &owned_source
        };

        let directives = ignore::parse_ignore_directives(source);

        let run_rules = |rules: &FileRuleSet| -> (Vec<Violation>, Option<(PathBuf, String)>) {
            with_reusable_line_index(source, |line_index| {
                with_reusable_allocator(|allocator| {
                    let needs_ast = rules
                        .ast_rules
                        .iter()
                        .any(|rule| rule.severity().is_enabled());

                    let parse_result: Option<oxc_ast::ast::Program<'_>> = if needs_ast {
                        match crate::syntax::source_type_from_path(file) {
                            Some(source_type) => {
                                let parser_return =
                                    Parser::new(allocator, source, source_type).parse();
                                if parser_return.panicked {
                                    return (
                                        Vec::new(),
                                        Some((file.clone(), "parse error".to_string())),
                                    );
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
                        source,
                        line_index,
                        type_location_style,
                    };
                    for rule in rules.text_rules.iter() {
                        if rule.severity().is_enabled() {
                            violations.extend(rule.check(&text_ctx));
                        }
                    }

                    let graph_ctx = GraphContext {
                        file,
                        line_index,
                        type_location_style,
                        import_graph: import_graph.clone(),
                        workspace: workspace.clone(),
                    };
                    if !import_graph.has_graph_parse_failure(file) {
                        for rule in rules.graph_rules.iter() {
                            if rule.severity().is_enabled() {
                                violations.extend(rule.check(&graph_ctx));
                            }
                        }
                    }

                    if let Some(program) = parse_result.as_ref() {
                        let ast_ctx = AstContext {
                            file,
                            source,
                            program,
                            line_index,
                            type_location_style,
                        };
                        for rule in rules.ast_rules.iter() {
                            if rule.severity().is_enabled() {
                                violations.extend(rule.check(&ast_ctx));
                            }
                        }
                    }

                    (violations, None)
                })
            })
        };

        if let Some(cached) = cached_violations.get(file) {
            if input.changed_rules.is_empty() {
                let suppression_info = compute_suppression_info(file, cached, &directives);
                let kept: Vec<Violation> = cached
                    .iter()
                    .filter(|violation| {
                        !ignore::should_suppress_violation(
                            &directives,
                            violation.line,
                            violation.rule,
                        )
                    })
                    .cloned()
                    .collect();
                return Ok((kept, suppression_info, None));
            }

            let mut kept: Vec<Violation> = cached
                .iter()
                .filter(|violation| {
                    !input.changed_rules.contains(violation.rule)
                        && !ignore::should_suppress_violation(
                            &directives,
                            violation.line,
                            violation.rule,
                        )
                })
                .cloned()
                .collect();

            let (changed_violations, parse_failure) = run_rules(&runtime.changed_rules);
            if let Some(parse_failure) = parse_failure {
                return Ok((Vec::new(), None, Some(parse_failure)));
            }

            kept.extend(changed_violations);
            let suppression_info = compute_suppression_info(file, &kept, &directives);
            kept.retain(|violation| {
                !ignore::should_suppress_violation(&directives, violation.line, violation.rule)
            });
            return Ok((kept, suppression_info, None));
        }

        let (new_violations, parse_failure) = run_rules(&runtime.rules);
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
        let total = files.len();
        let progress_bar = if verbose >= 2 {
            let bar = ProgressBar::new(total as u64);
            bar.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
            );
            bar.set_message("linting");
            Some(bar)
        } else {
            None
        };
        let processed = AtomicUsize::new(0);

        let results = files
            .par_iter()
            .zip(&file_config_ids)
            .map(|(file, config_id)| {
                let result = process_file(file, *config_id);
                let count = processed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(ref bar) = progress_bar {
                    bar.set_position(count as u64);
                }
                result
            })
            .collect();

        if let Some(bar) = progress_bar {
            bar.finish_and_clear();
        }
        results
    } else {
        files
            .iter()
            .zip(&file_config_ids)
            .map(|(file, config_id)| process_file(file, *config_id))
            .collect()
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

#[allow(clippy::too_many_arguments)]
pub fn check_files(
    files: &[PathBuf],
    config_set: &config::ConfigSet,
    import_graph: Arc<ImportGraph>,
    workspace: Option<Arc<crate::workspace::Workspace>>,
    cached_violations: Arc<HashMap<PathBuf, Vec<Violation>>>,
    sources: &HashMap<PathBuf, String>,
    changed_rules: Arc<HashSet<crate::rules::RuleId>>,
    verbose: u8,
) -> Result<(
    Vec<Violation>,
    ignore::SuppressionReport,
    HashMap<PathBuf, String>,
)> {
    let input = FileCheckInput {
        files,
        config_set,
        import_graph,
        workspace,
        cached_violations,
        sources,
        changed_rules,
    };
    check_files_with_parallelism(&input, true, verbose)
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
    let workspace = match crate::workspace::Workspace::discover(project_root) {
        Ok(workspace) => Some(Arc::new(workspace)),
        Err(error) => {
            eprintln!("warning: workspace discovery failed: {error}");
            None
        }
    };
    let cached_violations = Arc::new(HashMap::new());
    let sources = HashMap::new();
    let changed_rules = Arc::new(HashSet::new());
    let input = FileCheckInput {
        files,
        config_set: &config_set,
        import_graph: graph,
        workspace,
        cached_violations,
        sources: &sources,
        changed_rules,
    };
    let (violations, _, _) = check_files_with_parallelism(&input, parallel, 0)?;
    Ok(violations)
}

pub fn build_file_rules(
    config: &RulesConfig,
    structure: &config::structure::ProjectStructureConfig,
    architecture: &config::architecture::ArchitectureConfig,
    import_graph: Arc<ImportGraph>,
    workspace: Option<Arc<crate::workspace::Workspace>>,
    rules_to_run: Option<&std::collections::HashSet<crate::rules::RuleId>>,
) -> FileRuleSet {
    let components = Arc::new(structure.components.clone());
    let types = Arc::new(structure.types.clone());
    let hooks = Arc::new(structure.hooks.clone());
    let tests = Arc::new(structure.tests.clone());
    let generated = Arc::new(structure.generated.clone());
    let constants = Arc::new(structure.constants.clone());

    let mut ast_rules: Vec<Box<dyn crate::rules::AstRule + Send + Sync>> = Vec::new();

    macro_rules! push_ast {
        ($rule_id:expr, $rule_config:expr, $adapter:expr) => {
            if $rule_config.severity.is_enabled()
                && rules_to_run.map_or(true, |rules| rules.contains($rule_id))
            {
                ast_rules.push(Box::new($adapter));
            }
        };
    }

    push_ast!(
        BOOLEAN_PREFIX_RULE_ID,
        config.boolean_prefix,
        BooleanPrefixAdapter {
            config: config.boolean_prefix.clone(),
        }
    );
    push_ast!(
        NO_CONSOLE_RULE_ID,
        config.no_console,
        NoConsoleAdapter {
            config: config.no_console.clone(),
        }
    );
    push_ast!(
        NO_DEFAULT_EXPORT_RULE_ID,
        config.no_default_export,
        NoDefaultExportAdapter {
            config: config.no_default_export.clone(),
            components: components.clone(),
        }
    );
    push_ast!(
        NO_EXPORT_STAR_RULE_ID,
        config.no_export_star,
        NoExportStarAdapter {
            config: config.no_export_star.clone(),
        }
    );
    push_ast!(
        NO_FOCUSED_TEST_RULE_ID,
        config.no_focused_test,
        NoFocusedTestAdapter {
            config: config.no_focused_test.clone(),
        }
    );
    push_ast!(
        MAX_FILE_EXPORTS_RULE_ID,
        config.max_file_exports,
        MaxFileExportsAdapter {
            config: config.max_file_exports.clone(),
        }
    );
    push_ast!(
        MAX_FUNCTION_PARAMS_RULE_ID,
        config.max_function_params,
        MaxFunctionParamsAdapter {
            config: config.max_function_params.clone(),
        }
    );
    push_ast!(
        NO_ENUMS_RULE_ID,
        config.no_enums,
        NoEnumsAdapter {
            config: config.no_enums.clone(),
        }
    );
    push_ast!(
        NO_DEBUGGER_RULE_ID,
        config.no_debugger,
        NoDebuggerAdapter {
            config: config.no_debugger.clone(),
        }
    );
    push_ast!(
        NO_EVAL_RULE_ID,
        config.no_eval,
        NoEvalAdapter {
            config: config.no_eval.clone(),
        }
    );
    push_ast!(
        NO_SIDE_EFFECT_IMPORTS_RULE_ID,
        config.no_side_effect_imports,
        NoSideEffectImportsAdapter {
            config: config.no_side_effect_imports.clone(),
        }
    );
    push_ast!(
        SORT_IMPORTS_RULE_ID,
        config.sort_imports,
        SortImportsAdapter {
            config: config.sort_imports.clone(),
        }
    );
    push_ast!(
        SORT_EXPORTS_RULE_ID,
        config.sort_exports,
        SortExportsAdapter {
            config: config.sort_exports.clone(),
        }
    );
    push_ast!(
        NO_EMPTY_INTERFACE_RULE_ID,
        config.no_empty_interface,
        NoEmptyInterfaceAdapter {
            config: config.no_empty_interface.clone(),
        }
    );
    push_ast!(
        NO_INTERFACE_RULE_ID,
        config.no_interface,
        NoInterfaceAdapter {
            config: config.no_interface.clone(),
        }
    );
    push_ast!(
        NO_MUTABLE_EXPORTS_RULE_ID,
        config.no_mutable_exports,
        NoMutableExportsAdapter {
            config: config.no_mutable_exports.clone(),
        }
    );
    push_ast!(
        NO_NAMESPACE_RULE_ID,
        config.no_namespace,
        NoNamespaceAdapter {
            config: config.no_namespace.clone(),
        }
    );
    push_ast!(
        NO_SILENT_CATCH_RULE_ID,
        config.no_silent_catch,
        NoSilentCatchAdapter {
            config: config.no_silent_catch.clone(),
        }
    );
    push_ast!(
        NO_SKIPPED_TEST_RULE_ID,
        config.no_skipped_test,
        NoSkippedTestAdapter {
            config: config.no_skipped_test.clone(),
        }
    );
    push_ast!(
        NO_THEN_CHAIN_RULE_ID,
        config.no_then_chain,
        NoThenChainAdapter {
            config: config.no_then_chain.clone(),
        }
    );
    push_ast!(
        ENTRY_FILE_NO_LOGIC_RULE_ID,
        config.entry_file_no_logic,
        EntryFileNoLogicAdapter {
            config: config.entry_file_no_logic.clone(),
        }
    );
    push_ast!(
        EXPLICIT_RETURN_TYPE_RULE_ID,
        config.explicit_return_type,
        ExplicitReturnTypeAdapter {
            config: config.explicit_return_type.clone(),
        }
    );
    push_ast!(
        NO_NON_NULL_ASSERTION_RULE_ID,
        config.no_non_null_assertion,
        NoNonNullAssertionAdapter {
            config: config.no_non_null_assertion.clone(),
        }
    );
    push_ast!(
        NO_AWAIT_IN_LOOP_RULE_ID,
        config.no_await_in_loop,
        NoAwaitInLoopAdapter {
            config: config.no_await_in_loop.clone(),
        }
    );
    push_ast!(
        NO_PROMISE_EXECUTOR_RETURN_RULE_ID,
        config.no_promise_executor_return,
        NoPromiseExecutorReturnAdapter {
            config: config.no_promise_executor_return.clone(),
        }
    );
    push_ast!(
        NO_UNSAFE_OPTIONAL_CHAINING_RULE_ID,
        config.no_unsafe_optional_chaining,
        NoUnsafeOptionalChainingAdapter {
            config: config.no_unsafe_optional_chaining.clone(),
        }
    );
    push_ast!(
        NO_MAGIC_NUMBERS_RULE_ID,
        config.no_magic_numbers,
        NoMagicNumbersAdapter {
            config: config.no_magic_numbers.clone(),
        }
    );
    push_ast!(
        NO_TYPE_ASSERTION_RULE_ID,
        config.no_type_assertion,
        NoTypeAssertionAdapter {
            config: config.no_type_assertion.clone(),
        }
    );
    push_ast!(
        NO_UNNECESSARY_TYPE_ASSERTION_RULE_ID,
        config.no_unnecessary_type_assertion,
        NoUnnecessaryTypeAssertionAdapter {
            config: config.no_unnecessary_type_assertion.clone(),
        }
    );
    push_ast!(
        NO_PROCESS_ENV_RULE_ID,
        config.no_process_env,
        NoProcessEnvAdapter {
            config: config.no_process_env.clone(),
        }
    );
    push_ast!(
        NO_ABBREVIATIONS_RULE_ID,
        config.no_abbreviations,
        NoAbbreviationsAdapter {
            config: config.no_abbreviations.clone(),
        }
    );
    push_ast!(
        NO_RESTRICTED_IMPORTS_RULE_ID,
        config.no_restricted_imports,
        NoRestrictedImportsAdapter {
            config: config.no_restricted_imports.clone(),
        }
    );
    push_ast!(
        NO_INLINE_TYPES_RULE_ID,
        config.no_inline_types,
        NoInlineTypesAdapter {
            config: config.no_inline_types.clone(),
            types: types.clone(),
        }
    );
    push_ast!(
        HOOK_NO_JSX_RULE_ID,
        config.hook_no_jsx,
        HookNoJsxAdapter {
            config: config.hook_no_jsx.clone(),
            hooks: hooks.clone(),
        }
    );
    push_ast!(
        HOOK_PREFIX_RULE_ID,
        config.hook_prefix,
        HookPrefixAdapter {
            config: config.hook_prefix.clone(),
            hooks: hooks.clone(),
        }
    );
    push_ast!(
        COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID,
        config.component_file_only_components,
        ComponentFileOnlyComponentsAdapter {
            config: config.component_file_only_components.clone(),
            components: components.clone(),
        }
    );
    push_ast!(
        NO_TEST_CODE_IN_PRODUCTION_RULE_ID,
        config.no_test_code_in_production,
        NoTestCodeInProductionAdapter {
            config: config.no_test_code_in_production.clone(),
            tests: tests.clone(),
        }
    );
    push_ast!(
        NO_ANY_RULE_ID,
        config.no_any,
        NoAnyAdapter {
            config: config.no_any.clone(),
            generated: generated.clone(),
        }
    );
    push_ast!(
        NO_NESTED_FUNCTIONS_RULE_ID,
        config.no_nested_functions,
        NoNestedFunctionsAdapter {
            config: config.no_nested_functions.clone(),
        }
    );
    push_ast!(
        NO_COMMENTS_RULE_ID,
        config.no_comments,
        NoCommentsAdapter {
            config: config.no_comments.clone(),
        }
    );
    push_ast!(
        NO_LOGIC_IN_BARREL_RULE_ID,
        config.no_logic_in_barrel,
        NoLogicInBarrelAdapter {
            config: config.no_logic_in_barrel.clone(),
        }
    );
    push_ast!(
        NO_BARREL_FILES_RULE_ID,
        config.no_barrel_files,
        NoBarrelFilesAdapter {
            config: config.no_barrel_files.clone(),
        }
    );
    push_ast!(
        PREFER_SATISFIES_RULE_ID,
        config.prefer_satisfies,
        PreferSatisfiesAdapter {
            config: config.prefer_satisfies.clone(),
        }
    );
    push_ast!(
        PREFER_READONLY_RULE_ID,
        config.prefer_readonly,
        PreferReadonlyAdapter {
            config: config.prefer_readonly.clone(),
        }
    );
    push_ast!(
        NO_LOGIC_IN_DOMAIN_RULE_ID,
        config.no_logic_in_domain,
        NoLogicInDomainAdapter {
            config: config.no_logic_in_domain.clone(),
            types: types.clone(),
            constants: constants.clone(),
        }
    );

    let mut text_rules: Vec<Box<dyn crate::rules::TextRule + Send + Sync>> = Vec::new();
    if config.no_large_file.severity.is_enabled()
        && rules_to_run.is_none_or(|rules| rules.contains(NO_LARGE_FILE_RULE_ID))
    {
        text_rules.push(Box::new(NoLargeFileAdapter {
            config: config.no_large_file.clone(),
        }));
    }

    let mut graph_rules: Vec<Box<dyn crate::rules::GraphRule + Send + Sync>> = Vec::new();

    macro_rules! push_graph {
        ($rule_id:expr, $rule_config:expr, $adapter:expr) => {
            if $rule_config.severity.is_enabled()
                && rules_to_run.map_or(true, |rules| rules.contains($rule_id))
            {
                graph_rules.push(Box::new($adapter));
            }
        };
    }

    push_graph!(
        NO_UPWARD_IMPORT_RULE_ID,
        config.no_upward_import,
        NoUpwardImportAdapter {
            config: config.no_upward_import.clone(),
        }
    );
    push_graph!(
        LAYER_BOUNDARIES_RULE_ID,
        config.layer_boundaries,
        LayerBoundariesAdapter {
            config: config.layer_boundaries.clone(),
            layers: architecture.layers.clone(),
        }
    );
    push_graph!(
        NO_TEST_IMPORT_RULE_ID,
        config.no_test_import,
        NoTestImportAdapter {
            config: config.no_test_import.clone(),
            tests: tests.clone(),
        }
    );
    push_graph!(
        NO_BARREL_CHAIN_RULE_ID,
        config.no_barrel_chain,
        NoBarrelChainAdapter {
            config: config.no_barrel_chain.clone(),
        }
    );
    push_graph!(
        NO_CIRCULAR_IMPORT_RULE_ID,
        config.no_circular_import,
        NoCircularImportAdapter {
            config: config.no_circular_import.clone(),
            context: crate::rules::no_circular_import::CircularImportContext::new(
                import_graph.as_ref(),
            ),
        }
    );
    push_graph!(
        NO_ORPHAN_FILES_RULE_ID,
        config.no_orphan_files,
        NoOrphanFilesAdapter {
            config: config.no_orphan_files.clone(),
            context: crate::rules::no_orphan_files::NoOrphanFilesContext::new(
                import_graph.as_ref(),
            ),
        }
    );
    push_graph!(
        NO_PRIVATE_PACKAGE_IMPORT_RULE_ID,
        config.no_private_package_import,
        NoPrivatePackageImportAdapter {
            config: config.no_private_package_import.clone(),
        }
    );

    if config.no_package_cycle.severity.is_enabled()
        && rules_to_run.is_none_or(|rules| rules.contains(NO_PACKAGE_CYCLE_RULE_ID))
        && let Some(workspace) = workspace
    {
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
    inventory: &DirectoryInventory,
    root: &Path,
    rules: &RulesConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let inventory = crate::directory_inventory::filter_inventory(inventory, root, exclude_dirs);
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
