use std::sync::Arc;

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

macro_rules! graph_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        pub struct $name {
            pub config: $config_ty,
        }
        impl GraphRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
                $module::check_file(
                    ctx.file,
                    ctx.line_index,
                    ctx.import_graph.as_ref(),
                    &self.config,
                )
            }
        }
    };
}

macro_rules! text_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident) => {
        pub struct $name {
            pub config: $config_ty,
        }
        impl TextRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &TextContext<'_>) -> Vec<Violation> {
                $module::check_file(ctx.file, ctx.source, &self.config)
            }
        }
    };
}

macro_rules! context_ast_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident, $($field:ident: $field_ty:ty),+ $(,)?) => {
        pub struct $name {
            pub config: $config_ty,
            $(pub $field: $field_ty,)*
        }
        impl AstRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
                $module::check_file(
                    ctx.file,
                    ctx.program,
                    ctx.line_index,
                    &self.config,
                    $(self.$field.as_ref(),)*
                )
            }
        }
    };
}

macro_rules! context_fixable_ast_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident, $($field:ident: $field_ty:ty),+ $(,)?) => {
        pub struct $name {
            pub config: $config_ty,
            $(pub $field: $field_ty,)*
        }
        impl AstRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
                $module::check_file(
                    ctx.file,
                    ctx.program,
                    ctx.line_index,
                    &self.config,
                    $(self.$field.as_ref(),)*
                )
            }

            fn supports_fix(&self) -> bool {
                true
            }

            fn fix(&self, ctx: &AstContext<'_>) -> Vec<Fix> {
                $module::fix_file(
                    ctx.file,
                    ctx.program,
                    ctx.source,
                    &self.config,
                    $(self.$field.as_ref(),)*
                )
            }
        }
    };
}

macro_rules! context_graph_rule_adapter {
    ($name:ident, $id:expr, $config_ty:ty, $module:ident, $($field:ident: $field_ty:ty),+ $(,)?) => {
        pub struct $name {
            pub config: $config_ty,
            $(pub $field: $field_ty,)*
        }
        impl GraphRule for $name {
            fn severity(&self) -> Severity {
                self.config.severity
            }
            fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation> {
                $module::check_file(
                    ctx.file,
                    ctx.line_index,
                    ctx.import_graph.as_ref(),
                    &self.config,
                    $(self.$field.as_ref(),)*
                )
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

context_ast_rule_adapter!(
    NoDefaultExportAdapter,
    NO_DEFAULT_EXPORT_RULE_ID,
    crate::config::NoDefaultExportRuleConfig,
    no_default_export,
    components: Arc<crate::config::structure::DomainConfig>,
);

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

graph_rule_adapter!(
    NoUpwardImportAdapter,
    NO_UPWARD_IMPORT_RULE_ID,
    crate::config::UpwardImportRuleConfig,
    no_upward_import
);

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

pub struct SortImportsAdapter {
    pub config: crate::config::RuleConfig,
}
impl AstRule for SortImportsAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        sort_imports::check_file(
            ctx.file,
            ctx.program,
            ctx.source,
            ctx.line_index,
            &self.config,
        )
    }
    fn supports_fix(&self) -> bool {
        true
    }
    fn fix(&self, ctx: &AstContext<'_>) -> Vec<Fix> {
        sort_imports::fix_file(ctx.file, ctx.program, ctx.source, &self.config)
    }
}

pub struct SortExportsAdapter {
    pub config: crate::config::RuleConfig,
}
impl AstRule for SortExportsAdapter {
    fn severity(&self) -> Severity {
        self.config.severity
    }
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation> {
        sort_exports::check_file(
            ctx.file,
            ctx.program,
            ctx.source,
            ctx.line_index,
            &self.config,
        )
    }
    fn supports_fix(&self) -> bool {
        true
    }
    fn fix(&self, ctx: &AstContext<'_>) -> Vec<Fix> {
        sort_exports::fix_file(ctx.file, ctx.program, ctx.source, &self.config)
    }
}

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
    crate::config::NoThenChainRuleConfig,
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
    NoAwaitInLoopAdapter,
    NO_AWAIT_IN_LOOP_RULE_ID,
    crate::config::RuleConfig,
    no_await_in_loop
);
ast_rule_adapter!(
    NoPromiseExecutorReturnAdapter,
    NO_PROMISE_EXECUTOR_RETURN_RULE_ID,
    crate::config::RuleConfig,
    no_promise_executor_return
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
ast_rule_adapter!(
    NoUnnecessaryTypeAssertionAdapter,
    NO_UNNECESSARY_TYPE_ASSERTION_RULE_ID,
    crate::config::RuleConfig,
    no_unnecessary_type_assertion
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
    crate::config::BarrelRuleConfig,
    no_logic_in_barrel
);

text_rule_adapter!(
    NoLargeFileAdapter,
    NO_LARGE_FILE_RULE_ID,
    crate::config::FileLengthRuleConfig,
    no_large_file
);

ast_rule_adapter!(
    NoBarrelFilesAdapter,
    NO_BARREL_FILES_RULE_ID,
    crate::config::BarrelRuleConfig,
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
    pub types: Arc<crate::config::structure::DomainConfig>,
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
            self.types.as_ref(),
        )
    }
}

context_ast_rule_adapter!(
    HookNoJsxAdapter,
    HOOK_NO_JSX_RULE_ID,
    crate::config::RuleConfig,
    hook_no_jsx,
    hooks: Arc<crate::config::structure::DomainConfig>,
);

context_ast_rule_adapter!(
    HookPrefixAdapter,
    HOOK_PREFIX_RULE_ID,
    crate::config::HookPrefixRuleConfig,
    hook_prefix,
    hooks: Arc<crate::config::structure::DomainConfig>,
);

context_ast_rule_adapter!(
    ComponentFileOnlyComponentsAdapter,
    COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID,
    crate::config::RuleConfig,
    component_file_only_components,
    components: Arc<crate::config::structure::DomainConfig>,
);

context_ast_rule_adapter!(
    NoTestCodeInProductionAdapter,
    NO_TEST_CODE_IN_PRODUCTION_RULE_ID,
    crate::config::RuleConfig,
    no_test_code_in_production,
    tests: Arc<crate::config::structure::DomainConfig>,
);

context_graph_rule_adapter!(
    NoTestImportAdapter,
    NO_TEST_IMPORT_RULE_ID,
    crate::config::RuleConfig,
    no_test_import,
    tests: Arc<crate::config::structure::DomainConfig>,
);

context_fixable_ast_rule_adapter!(
    NoAnyAdapter,
    NO_ANY_RULE_ID,
    crate::config::NoAnyRuleConfig,
    no_any,
    generated: Arc<crate::config::structure::DomainConfig>,
);

graph_rule_adapter!(
    NoBarrelChainAdapter,
    NO_BARREL_CHAIN_RULE_ID,
    crate::config::RuleConfig,
    no_barrel_chain
);

pub struct NoCircularImportAdapter {
    pub config: crate::config::NoCircularImportRuleConfig,
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
    pub context: no_orphan_files::NoOrphanFilesContext,
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
            &self.context,
            &self.config,
        )
    }
}

context_ast_rule_adapter!(
    NoLogicInDomainAdapter,
    NO_LOGIC_IN_DOMAIN_RULE_ID,
    crate::config::RuleConfig,
    no_logic_in_domain,
    types: Arc<crate::config::structure::DomainConfig>,
    constants: Arc<crate::config::structure::DomainConfig>,
);

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
