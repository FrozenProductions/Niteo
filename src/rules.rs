macro_rules! declare_rules {
    (
        $( $mod_name:ident => { id: $rule_id:ident, value: $rule_value:literal, config: $config_type:ty $(, default_severity: $default_sev:expr )? } ),* $(,)?
    ) => {
        $( pub mod $mod_name; )*

        use std::path::{Path, PathBuf};

        use crate::config::Severity;
        use crate::import_graph::ImportGraph;
        use crate::syntax::LineIndex;

        pub type RuleId = &'static str;

        $( pub const $rule_id: RuleId = $rule_value; )*

        pub fn known_rule_ids() -> &'static [RuleId] {
            &[$( $rule_value ),*]
        }

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
    directory_must_have_barrel => { id: DIRECTORY_MUST_HAVE_BARREL_RULE_ID, value: "directory-must-have-barrel", config: crate::config::RuleConfig },
    entry_file_no_logic => { id: ENTRY_FILE_NO_LOGIC_RULE_ID, value: "entry-file-no-logic", config: crate::config::EntryFileNoLogicRuleConfig },
    explicit_return_type => { id: EXPLICIT_RETURN_TYPE_RULE_ID, value: "explicit-return-type", config: crate::config::RuleConfig },
    hook_no_jsx => { id: HOOK_NO_JSX_RULE_ID, value: "hook-no-jsx", config: crate::config::RuleConfig },
    hook_prefix => { id: HOOK_PREFIX_RULE_ID, value: "hook-prefix", config: crate::config::HookPrefixRuleConfig },
    layer_boundaries => { id: LAYER_BOUNDARIES_RULE_ID, value: "layer-boundaries", config: crate::config::RuleConfig, default_severity: Severity::Off },
    max_directory_depth => { id: MAX_DIRECTORY_DEPTH_RULE_ID, value: "max-directory-depth", config: crate::config::MaxDirectoryDepthRuleConfig },
    max_file_exports => { id: MAX_FILE_EXPORTS_RULE_ID, value: "max-file-exports", config: crate::config::FileExportsRuleConfig },
    max_function_params => { id: MAX_FUNCTION_PARAMS_RULE_ID, value: "max-function-params", config: crate::config::MaxFunctionParamsRuleConfig },
    max_items_per_directory => { id: MAX_ITEMS_PER_DIRECTORY_RULE_ID, value: "max-items-per-directory", config: crate::config::MaxItemsPerDirectoryRuleConfig },
    min_items_per_directory => { id: MIN_ITEMS_PER_DIRECTORY_RULE_ID, value: "min-items-per-directory", config: crate::config::MinItemsPerDirectoryRuleConfig },
    no_any => { id: NO_ANY_RULE_ID, value: "no-any", config: crate::config::NoAnyRuleConfig },
    no_barrel_chain => { id: NO_BARREL_CHAIN_RULE_ID, value: "no-barrel-chain", config: crate::config::RuleConfig },
    no_circular_import => { id: NO_CIRCULAR_IMPORT_RULE_ID, value: "no-circular-import", config: crate::config::RuleConfig },
    no_barrel_files => { id: NO_BARREL_FILES_RULE_ID, value: "no-barrel-files", config: crate::config::RuleConfig },
    no_comments => { id: NO_COMMENTS_RULE_ID, value: "no-comments", config: crate::config::CommentsRuleConfig },
    no_console => { id: NO_CONSOLE_RULE_ID, value: "no-console", config: crate::config::NoConsoleRuleConfig },
    no_debugger => { id: NO_DEBUGGER_RULE_ID, value: "no-debugger", config: crate::config::RuleConfig },
    no_default_export => { id: NO_DEFAULT_EXPORT_RULE_ID, value: "no-default-export", config: crate::config::NoDefaultExportRuleConfig },
    no_dump_files => { id: NO_DUMP_FILES_RULE_ID, value: "no-dump-files", config: crate::config::NoDumpFilesRuleConfig },
    no_duplicate_file_names => { id: NO_DUPLICATE_FILE_NAMES_RULE_ID, value: "no-duplicate-file-names", config: crate::config::NoDuplicateFileNamesRuleConfig },
    no_empty_directories => { id: NO_EMPTY_DIRECTORIES_RULE_ID, value: "no-empty-directories", config: crate::config::NoEmptyDirectoriesRuleConfig },
    no_empty_domain => { id: NO_EMPTY_DOMAIN_RULE_ID, value: "no-empty-domain", config: crate::config::NoEmptyDomainRuleConfig },
    no_anemic_domain => { id: NO_ANEMIC_DOMAIN_RULE_ID, value: "no-anemic-domain", config: crate::config::NoAnemicDomainRuleConfig },
    no_god_domain => { id: NO_GOD_DOMAIN_RULE_ID, value: "no-god-domain", config: crate::config::NoGodDomainRuleConfig },
    no_empty_interface => { id: NO_EMPTY_INTERFACE_RULE_ID, value: "no-empty-interface", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_enums => { id: NO_ENUMS_RULE_ID, value: "no-enums", config: crate::config::RuleConfig },
    no_eval => { id: NO_EVAL_RULE_ID, value: "no-eval", config: crate::config::RuleConfig },
    no_export_star => { id: NO_EXPORT_STAR_RULE_ID, value: "no-export-star", config: crate::config::RuleConfig },
    no_focused_test => { id: NO_FOCUSED_TEST_RULE_ID, value: "no-focused-test", config: crate::config::RuleConfig },
    no_inline_types => { id: NO_INLINE_TYPES_RULE_ID, value: "no-inline-types", config: crate::config::RuleConfig },
    no_interface => { id: NO_INTERFACE_RULE_ID, value: "no-interface", config: crate::config::NoInterfaceRuleConfig },
    no_large_file => { id: NO_LARGE_FILE_RULE_ID, value: "no-large-file", config: crate::config::FileLengthRuleConfig },
    no_logic_in_barrel => { id: NO_LOGIC_IN_BARREL_RULE_ID, value: "no-logic-in-barrel", config: crate::config::RuleConfig },
    no_logic_in_domain => { id: NO_LOGIC_IN_DOMAIN_RULE_ID, value: "no-logic-in-domain", config: crate::config::RuleConfig },
    no_abbreviations => { id: NO_ABBREVIATIONS_RULE_ID, value: "no-abbreviations", config: crate::config::NoAbbreviationsRuleConfig },
    no_restricted_imports => { id: NO_RESTRICTED_IMPORTS_RULE_ID, value: "no-restricted-imports", config: crate::config::NoRestrictedImportsRuleConfig },
    no_mutable_exports => { id: NO_MUTABLE_EXPORTS_RULE_ID, value: "no-mutable-exports", config: crate::config::RuleConfig },
    no_namespace => { id: NO_NAMESPACE_RULE_ID, value: "no-namespace", config: crate::config::RuleConfig },
    no_nested_functions => { id: NO_NESTED_FUNCTIONS_RULE_ID, value: "no-nested-functions", config: crate::config::NoNestedFunctionsRuleConfig },
    no_non_null_assertion => { id: NO_NON_NULL_ASSERTION_RULE_ID, value: "no-non-null-assertion", config: crate::config::RuleConfig },
    no_magic_numbers => { id: NO_MAGIC_NUMBERS_RULE_ID, value: "no-magic-numbers", config: crate::config::NoMagicNumbersRuleConfig },
    no_orphan_files => { id: NO_ORPHAN_FILES_RULE_ID, value: "no-orphan-files", config: crate::config::NoOrphanFilesRuleConfig },
    no_package_cycle => { id: NO_PACKAGE_CYCLE_RULE_ID, value: "no-package-cycle", config: crate::config::RuleConfig },
    no_private_package_import => { id: NO_PRIVATE_PACKAGE_IMPORT_RULE_ID, value: "no-private-package-import", config: crate::config::RuleConfig },
    no_type_assertion => { id: NO_TYPE_ASSERTION_RULE_ID, value: "no-type-assertion", config: crate::config::RuleConfig },
    no_process_env => { id: NO_PROCESS_ENV_RULE_ID, value: "no-process-env", config: crate::config::RuleConfig },
    no_silent_catch => { id: NO_SILENT_CATCH_RULE_ID, value: "no-silent-catch", config: crate::config::RuleConfig },
    no_skipped_test => { id: NO_SKIPPED_TEST_RULE_ID, value: "no-skipped-test", config: crate::config::RuleConfig },
    no_test_code_in_production => { id: NO_TEST_CODE_IN_PRODUCTION_RULE_ID, value: "no-test-code-in-production", config: crate::config::RuleConfig },
    no_test_import => { id: NO_TEST_IMPORT_RULE_ID, value: "no-test-import", config: crate::config::RuleConfig },
    no_then_chain => { id: NO_THEN_CHAIN_RULE_ID, value: "no-then-chain", config: crate::config::RuleConfig },
    no_upward_import => { id: NO_UPWARD_IMPORT_RULE_ID, value: "no-upward-import", config: crate::config::UpwardImportRuleConfig },
    prefer_satisfies => { id: PREFER_SATISFIES_RULE_ID, value: "prefer-satisfies", config: crate::config::RuleConfig, default_severity: Severity::Info },
    prefer_readonly => { id: PREFER_READONLY_RULE_ID, value: "prefer-readonly", config: crate::config::RuleConfig },
}

pub mod test_call_utils;

#[derive(Debug, Clone)]
pub struct Fix {
    pub file: PathBuf,
    pub rule: RuleId,
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

pub trait FileRule {
    fn severity(&self) -> Severity;
    fn needs_ast(&self) -> bool;
    fn check(&self, ctx: &FileContext<'_>) -> Vec<Violation>;

    fn supports_fix(&self) -> bool {
        false
    }

    fn fix(&self, _ctx: &FileContext<'_>) -> Vec<Fix> {
        Vec::new()
    }
}

pub struct FileContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    pub program: Option<&'a oxc_ast::ast::Program<'a>>,
    pub line_index: &'a LineIndex,
    pub type_location_style: no_inline_types::TypeLocationStyle,
    pub import_graph: &'a ImportGraph,
    pub workspace: Option<&'a crate::workspace::Workspace>,
}

pub use crate::rules_runner::{
    check_directory_rules, check_dump_files, check_duplicate_file_names, check_files,
};
