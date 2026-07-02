use crate::config::Severity;
use crate::rules::*;

macro_rules! ast_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        pub struct $name {
            pub config: $config_ty,
        }
        impl AstRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
                $module::check_file(ctx.file, ctx.program, ctx.line_index, &self.config)
            }
        }
    };
}

macro_rules! fixable_ast_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        pub struct $name {
            pub config: $config_ty,
        }
        impl AstRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
                $module::check_file(ctx.file, ctx.program, ctx.line_index, &self.config)
            }

            fn supports_fix(&self) -> bool {
                true
            }

            fn fix(&self, ctx: &AstContext<'_>) -> Vec<Fix> {
                $module::fix_file(ctx.file, ctx.program, ctx.source, &self.config)
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

pub struct NoDefaultExportAdapter {
    pub config: crate::config::NoDefaultExportRuleConfig,
    pub components: crate::config::structure::DomainConfig,
}
impl AstRule for NoDefaultExportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        no_default_export::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.components,
        )
    }
}

ast_rule_adapter!(
    NoExportStarAdapter,
    NO_EXPORT_STAR_RULE_ID,
    crate::config::RuleConfig,
    no_export_star
);
fixable_ast_rule_adapter!(
    NoFocusedTestAdapter,
    NO_FOCUSED_TEST_RULE_ID,
    crate::config::RuleConfig,
    no_focused_test
);
ast_rule_adapter!(
    MaxFileExportsAdapter,
    MAX_FILE_EXPORTS_RULE_ID,
    crate::config::FileExportsRuleConfig,
    max_file_exports
);
ast_rule_adapter!(
    MaxFunctionParamsAdapter,
    MAX_FUNCTION_PARAMS_RULE_ID,
    crate::config::MaxFunctionParamsRuleConfig,
    max_function_params
);

pub struct NoUpwardImportAdapter {
    pub config: crate::config::UpwardImportRuleConfig,
}
impl GraphRule for NoUpwardImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_upward_import::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.config,
        )
    }
}

pub struct LayerBoundariesAdapter {
    pub config: crate::config::RuleConfig,
    pub layers: crate::config::architecture::LayerBoundaryConfig,
}
impl GraphRule for LayerBoundariesAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        layer_boundaries::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.config,
            &self.layers,
        )
    }
}

ast_rule_adapter!(
    NoEnumsAdapter,
    NO_ENUMS_RULE_ID,
    crate::config::RuleConfig,
    no_enums
);
fixable_ast_rule_adapter!(
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
    NoSideEffectImportsAdapter,
    NO_SIDE_EFFECT_IMPORTS_RULE_ID,
    crate::config::RuleConfig,
    no_side_effect_imports
);
fixable_ast_rule_adapter!(
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
fixable_ast_rule_adapter!(
    NoSkippedTestAdapter,
    NO_SKIPPED_TEST_RULE_ID,
    crate::config::RuleConfig,
    no_skipped_test
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
    ExplicitReturnTypeAdapter,
    EXPLICIT_RETURN_TYPE_RULE_ID,
    crate::config::RuleConfig,
    explicit_return_type
);
fixable_ast_rule_adapter!(
    NoNonNullAssertionAdapter,
    NO_NON_NULL_ASSERTION_RULE_ID,
    crate::config::RuleConfig,
    no_non_null_assertion
);
ast_rule_adapter!(
    NoUnsafeOptionalChainingAdapter,
    NO_UNSAFE_OPTIONAL_CHAINING_RULE_ID,
    crate::config::RuleConfig,
    no_unsafe_optional_chaining
);
ast_rule_adapter!(
    NoMagicNumbersAdapter,
    NO_MAGIC_NUMBERS_RULE_ID,
    crate::config::NoMagicNumbersRuleConfig,
    no_magic_numbers
);
ast_rule_adapter!(
    NoTypeAssertionAdapter,
    NO_TYPE_ASSERTION_RULE_ID,
    crate::config::RuleConfig,
    no_type_assertion
);
fixable_ast_rule_adapter!(
    NoProcessEnvAdapter,
    NO_PROCESS_ENV_RULE_ID,
    crate::config::RuleConfig,
    no_process_env
);
ast_rule_adapter!(
    NoAbbreviationsAdapter,
    NO_ABBREVIATIONS_RULE_ID,
    crate::config::NoAbbreviationsRuleConfig,
    no_abbreviations
);
ast_rule_adapter!(
    NoRestrictedImportsAdapter,
    NO_RESTRICTED_IMPORTS_RULE_ID,
    crate::config::NoRestrictedImportsRuleConfig,
    no_restricted_imports
);
ast_rule_adapter!(
    NoNestedFunctionsAdapter,
    NO_NESTED_FUNCTIONS_RULE_ID,
    crate::config::NoNestedFunctionsRuleConfig,
    no_nested_functions
);
ast_rule_adapter!(
    NoCommentsAdapter,
    NO_COMMENTS_RULE_ID,
    crate::config::CommentsRuleConfig,
    no_comments
);
ast_rule_adapter!(
    NoLogicInBarrelAdapter,
    NO_LOGIC_IN_BARREL_RULE_ID,
    crate::config::RuleConfig,
    no_logic_in_barrel
);

pub struct NoLargeFileAdapter {
    pub config: crate::config::FileLengthRuleConfig,
}
impl TextRule for NoLargeFileAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &TextContext<'_>) -> Vec<Violation> {
        no_large_file::check_file(ctx.file, ctx.source, &self.config)
    }
}

ast_rule_adapter!(
    NoBarrelFilesAdapter,
    NO_BARREL_FILES_RULE_ID,
    crate::config::RuleConfig,
    no_barrel_files
);
fixable_ast_rule_adapter!(
    PreferSatisfiesAdapter,
    PREFER_SATISFIES_RULE_ID,
    crate::config::RuleConfig,
    prefer_satisfies
);
fixable_ast_rule_adapter!(
    PreferReadonlyAdapter,
    PREFER_READONLY_RULE_ID,
    crate::config::RuleConfig,
    prefer_readonly
);

pub struct NoInlineTypesAdapter {
    pub config: crate::config::RuleConfig,
    pub types: crate::config::structure::DomainConfig,
}
impl AstRule for NoInlineTypesAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        no_inline_types::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            ctx.type_location_style,
            &self.types,
        )
    }
}

pub struct HookNoJsxAdapter {
    pub config: crate::config::RuleConfig,
    pub hooks: crate::config::structure::DomainConfig,
}
impl AstRule for HookNoJsxAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        hook_no_jsx::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.hooks,
        )
    }
}

pub struct HookPrefixAdapter {
    pub config: crate::config::HookPrefixRuleConfig,
    pub hooks: crate::config::structure::DomainConfig,
}
impl AstRule for HookPrefixAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        hook_prefix::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.hooks,
        )
    }
}

pub struct ComponentFileOnlyComponentsAdapter {
    pub config: crate::config::RuleConfig,
    pub components: crate::config::structure::DomainConfig,
}
impl AstRule for ComponentFileOnlyComponentsAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        component_file_only_components::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.components,
        )
    }
}

pub struct NoTestCodeInProductionAdapter {
    pub config: crate::config::RuleConfig,
    pub tests: crate::config::structure::DomainConfig,
}
impl AstRule for NoTestCodeInProductionAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        no_test_code_in_production::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.tests,
        )
    }
}

pub struct NoTestImportAdapter {
    pub config: crate::config::RuleConfig,
    pub tests: crate::config::structure::DomainConfig,
}
impl GraphRule for NoTestImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_test_import::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.config,
            &self.tests,
        )
    }
}

pub struct NoAnyAdapter {
    pub config: crate::config::NoAnyRuleConfig,
    pub generated: crate::config::structure::DomainConfig,
}
impl AstRule for NoAnyAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        no_any::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.generated,
        )
    }

    fn supports_fix(&self) -> bool {
        true
    }

    fn fix(&self, ctx: &AstContext<'_>) -> Vec<Fix> {
        no_any::fix_file(
            ctx.file,
            ctx.program,
            ctx.source,
            &self.config,
            &self.generated,
        )
    }
}

pub struct NoBarrelChainAdapter {
    pub config: crate::config::RuleConfig,
}
impl GraphRule for NoBarrelChainAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_barrel_chain::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.config,
        )
    }
}

pub struct NoCircularImportAdapter {
    pub config: crate::config::RuleConfig,
    pub context: no_circular_import::CircularImportContext,
}
impl GraphRule for NoCircularImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_circular_import::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.context,
            &self.config,
        )
    }
}

pub struct NoOrphanFilesAdapter {
    pub config: crate::config::NoOrphanFilesRuleConfig,
}
impl GraphRule for NoOrphanFilesAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_orphan_files::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            &self.config,
        )
    }
}

pub struct NoLogicInDomainAdapter {
    pub config: crate::config::RuleConfig,
    pub types: crate::config::structure::DomainConfig,
    pub constants: crate::config::structure::DomainConfig,
}
impl AstRule for NoLogicInDomainAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        no_logic_in_domain::check_file(
            ctx.file,
            ctx.program,
            ctx.line_index,
            &self.config,
            &self.types,
            &self.constants,
        )
    }
}

pub struct NoPrivatePackageImportAdapter {
    pub config: crate::config::RuleConfig,
}
impl GraphRule for NoPrivatePackageImportAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_private_package_import::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            ctx.workspace.as_deref(),
            &self.config,
        )
    }
}

pub struct NoPackageCycleAdapter {
    pub config: crate::config::RuleConfig,
    pub context: no_package_cycle::PackageCycleContext,
}
impl GraphRule for NoPackageCycleAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
        no_package_cycle::check_file(
            ctx.file,
            ctx.line_index,
            ctx.import_graph.as_ref(),
            ctx.workspace.as_deref(),
            &self.context,
            &self.config,
        )
    }
}
