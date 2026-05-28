macro_rules! declare_rules {
    (
        $( $mod_name:ident => { id: $rule_id:ident, value: $rule_value:literal, config: $config_type:ty $(, default_severity: $default_sev:expr )? } ),* $(,)?
    ) => {
        $( mod $mod_name; )*

        use anyhow::{Context, Result};
        use std::fs;
        use std::path::{Path, PathBuf};

        use oxc_allocator::Allocator;
        use oxc_parser::Parser;

        use crate::config::structure::{DomainConfig, ProjectStructureConfig};
        use crate::config::{ProjectConfig, Severity};
        use crate::ignore;
        use crate::import_graph::ImportGraph;
        use crate::syntax::LineIndex;

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

pub trait FileRule {
    fn severity(&self) -> Severity;
    fn needs_ast(&self) -> bool;
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation>;
}

pub struct FileContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    pub program: Option<&'a oxc_ast::ast::Program<'a>>,
    pub line_index: &'a LineIndex,
    pub type_location_style: no_inline_types::TypeLocationStyle,
    pub import_graph: &'a ImportGraph,
}

macro_rules! ast_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        struct $name {
            config: $config_ty,
        }
        impl FileRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn needs_ast(&self) -> bool {
                true
            }
            fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
                let Some(program) = ctx.program else {
                    return vec![];
                };
                $module::check_file(ctx.file, program, ctx.line_index, &self.config)
            }
        }
    };
}

macro_rules! text_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        struct $name {
            config: $config_ty,
        }
        impl FileRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn needs_ast(&self) -> bool {
                false
            }
            fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
                $module::check_file(ctx.file, ctx.source, &self.config)
            }
        }
    };
}

ast_rule_adapter!(
    BooleanPrefixAdapter,
    BOOLEAN_PREFIX_RULE_ID,
    crate::config::BooleanPrefixRuleConfig,
    boolean_prefix
);
ast_rule_adapter!(
    NoConsoleAdapter,
    NO_CONSOLE_RULE_ID,
    crate::config::NoConsoleRuleConfig,
    no_console
);
ast_rule_adapter!(
    NoDefaultExportAdapter,
    NO_DEFAULT_EXPORT_RULE_ID,
    crate::config::RuleConfig,
    no_default_export
);
ast_rule_adapter!(
    NoExportStarAdapter,
    NO_EXPORT_STAR_RULE_ID,
    crate::config::RuleConfig,
    no_export_star
);
ast_rule_adapter!(
    MaxFileExportsAdapter,
    MAX_FILE_EXPORTS_RULE_ID,
    crate::config::FileExportsRuleConfig,
    max_file_exports
);
struct NoUpwardImportAdapter {
    config: crate::config::UpwardImportRuleConfig,
}
impl FileRule for NoUpwardImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        false
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        no_upward_import::check_file(ctx.file, ctx.line_index, ctx.import_graph, &self.config)
    }
}
ast_rule_adapter!(
    NoEnumsAdapter,
    NO_ENUMS_RULE_ID,
    crate::config::RuleConfig,
    no_enums
);
ast_rule_adapter!(
    NoDebuggerAdapter,
    NO_DEBUGGER_RULE_ID,
    crate::config::RuleConfig,
    no_debugger
);
ast_rule_adapter!(
    NoEvalAdapter,
    NO_EVAL_RULE_ID,
    crate::config::RuleConfig,
    no_eval
);
ast_rule_adapter!(
    NoEmptyInterfaceAdapter,
    NO_EMPTY_INTERFACE_RULE_ID,
    crate::config::RuleConfig,
    no_empty_interface
);
ast_rule_adapter!(
    NoInterfaceAdapter,
    NO_INTERFACE_RULE_ID,
    crate::config::NoInterfaceRuleConfig,
    no_interface
);
ast_rule_adapter!(
    NoMutableExportsAdapter,
    NO_MUTABLE_EXPORTS_RULE_ID,
    crate::config::RuleConfig,
    no_mutable_exports
);
ast_rule_adapter!(
    NoNamespaceAdapter,
    NO_NAMESPACE_RULE_ID,
    crate::config::RuleConfig,
    no_namespace
);
ast_rule_adapter!(
    NoSilentCatchAdapter,
    NO_SILENT_CATCH_RULE_ID,
    crate::config::RuleConfig,
    no_silent_catch
);
ast_rule_adapter!(
    NoThenChainAdapter,
    NO_THEN_CHAIN_RULE_ID,
    crate::config::RuleConfig,
    no_then_chain
);
ast_rule_adapter!(
    EntryFileNoLogicAdapter,
    ENTRY_FILE_NO_LOGIC_RULE_ID,
    crate::config::EntryFileNoLogicRuleConfig,
    entry_file_no_logic
);
ast_rule_adapter!(
    NoNonNullAssertionAdapter,
    NO_NON_NULL_ASSERTION_RULE_ID,
    crate::config::RuleConfig,
    no_non_null_assertion
);

text_rule_adapter!(
    NoCommentsAdapter,
    NO_COMMENTS_RULE_ID,
    crate::config::CommentsRuleConfig,
    no_comments
);
text_rule_adapter!(
    NoLogicInBarrelAdapter,
    NO_LOGIC_IN_BARREL_RULE_ID,
    crate::config::RuleConfig,
    no_logic_in_barrel
);
text_rule_adapter!(
    NoLargeFileAdapter,
    NO_LARGE_FILE_RULE_ID,
    crate::config::FileLengthRuleConfig,
    no_large_file
);
text_rule_adapter!(
    NoBarrelFilesAdapter,
    NO_BARREL_FILES_RULE_ID,
    crate::config::RuleConfig,
    no_barrel_files
);
text_rule_adapter!(
    PreferSatisfiesAdapter,
    PREFER_SATISFIES_RULE_ID,
    crate::config::RuleConfig,
    prefer_satisfies
);

struct NoComponentDefaultExportAdapter {
    config: crate::config::RuleConfig,
    components: DomainConfig,
}
impl FileRule for NoComponentDefaultExportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        no_component_default_export::check_file(
            ctx.file,
            program,
            ctx.line_index,
            &self.config,
            &self.components,
        )
    }
}

struct NoInlineTypesAdapter {
    config: crate::config::RuleConfig,
    types: DomainConfig,
}
impl FileRule for NoInlineTypesAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        no_inline_types::check_file(
            ctx.file,
            program,
            ctx.line_index,
            &self.config,
            ctx.type_location_style,
            &self.types,
        )
    }
}

struct HookNoJsxAdapter {
    config: crate::config::RuleConfig,
    hooks: DomainConfig,
}
impl FileRule for HookNoJsxAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        hook_no_jsx::check_file(ctx.file, program, ctx.line_index, &self.config, &self.hooks)
    }
}

struct HookPrefixAdapter {
    config: crate::config::HookPrefixRuleConfig,
    hooks: DomainConfig,
}
impl FileRule for HookPrefixAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        hook_prefix::check_file(ctx.file, program, ctx.line_index, &self.config, &self.hooks)
    }
}

struct ComponentFileOnlyComponentsAdapter {
    config: crate::config::RuleConfig,
    components: DomainConfig,
}
impl FileRule for ComponentFileOnlyComponentsAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        component_file_only_components::check_file(
            ctx.file,
            program,
            ctx.line_index,
            &self.config,
            &self.components,
        )
    }
}

struct NoTestCodeInProductionAdapter {
    config: crate::config::RuleConfig,
    tests: DomainConfig,
}
impl FileRule for NoTestCodeInProductionAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        no_test_code_in_production::check_file(
            ctx.file,
            program,
            ctx.line_index,
            &self.config,
            &self.tests,
        )
    }
}

struct NoTestImportAdapter {
    config: crate::config::RuleConfig,
    tests: DomainConfig,
}
impl FileRule for NoTestImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        false
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        no_test_import::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph,
            &self.config,
            &self.tests,
        )
    }
}

struct NoAnyAdapter {
    config: crate::config::NoAnyRuleConfig,
    generated: DomainConfig,
}
impl FileRule for NoAnyAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        true
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        let Some(program) = ctx.program else {
            return vec![];
        };
        no_any::check_file(
            ctx.file,
            program,
            ctx.line_index,
            &self.config,
            &self.generated,
        )
    }
}

struct NoBarrelChainAdapter {
    config: crate::config::RuleConfig,
}
impl FileRule for NoBarrelChainAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        false
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        no_barrel_chain::check_file(ctx.file, ctx.line_index, ctx.import_graph, &self.config)
    }
}

struct NoLogicInDomainAdapter {
    config: crate::config::RuleConfig,
    types: DomainConfig,
    constants: DomainConfig,
}
impl FileRule for NoLogicInDomainAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn needs_ast(&self) -> bool {
        false
    }
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation> {
        no_logic_in_domain::check_file(
            ctx.file,
            ctx.source,
            &self.config,
            &self.types,
            &self.constants,
        )
    }
}

fn build_file_rules(
    config: &RulesConfig,
    structure: &ProjectStructureConfig,
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
        }),
        Box::new(NoExportStarAdapter {
            config: config.no_export_star.clone(),
        }),
        Box::new(MaxFileExportsAdapter {
            config: config.max_file_exports.clone(),
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
        Box::new(NoThenChainAdapter {
            config: config.no_then_chain.clone(),
        }),
        Box::new(EntryFileNoLogicAdapter {
            config: config.entry_file_no_logic.clone(),
        }),
        Box::new(NoNonNullAssertionAdapter {
            config: config.no_non_null_assertion.clone(),
        }),
        Box::new(NoComponentDefaultExportAdapter {
            config: config.no_component_default_export.clone(),
            components: structure.components.clone(),
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
        Box::new(NoBarrelChainAdapter {
            config: config.no_barrel_chain.clone(),
        }),
        Box::new(NoLogicInDomainAdapter {
            config: config.no_logic_in_domain.clone(),
            types: structure.types.clone(),
            constants: structure.constants.clone(),
        }),
    ]
}

pub fn check_files(
    files: &[PathBuf],
    config: &ProjectConfig,
    import_graph: &ImportGraph,
) -> Result<(Vec<Violation>, ignore::SuppressionReport)> {
    let rules = build_file_rules(&config.rules, &config.structure);

    let any_enabled = rules.iter().any(|r| r.severity().is_enabled());
    if !any_enabled {
        return Ok((vec![], ignore::SuppressionReport { files: vec![] }));
    }

    let needs_ast = rules
        .iter()
        .any(|r| r.severity().is_enabled() && r.needs_ast());

    let type_location_style =
        no_inline_types::TypeLocationStyle::detect(files, &config.structure.types);

    let mut violations = Vec::new();
    let mut suppression_files = Vec::new();

    for file in files {
        let source = fs::read_to_string(file)
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
        };

        let mut file_violations = Vec::new();
        for rule in &rules {
            if rule.severity().is_enabled() {
                file_violations.extend(rule.check(&ctx));
            }
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
    root: &Path,
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
    root: &Path,
    config: crate::config::MaxItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_items_per_directory::check_directories(root, &config)
}

pub fn check_min_items_per_directory(
    root: &Path,
    config: crate::config::MinItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    min_items_per_directory::check_directories(root, &config)
}

pub fn check_max_directory_depth(
    root: &Path,
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
