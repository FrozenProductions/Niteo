macro_rules! declare_rules {
    (
        $( $mod_name:ident => { id: $rule_id:ident, value: $rule_value:literal, path: $path:literal, config: $config_type:ty $(, default_severity: $default_sev:expr )? } ),* $(,)?
    ) => {
        $( #[path = $path] pub mod $mod_name; )*

        use std::path::{Component, Path, PathBuf};

        use crate::config::Severity;
        use crate::import_graph::ImportGraph;
        use crate::syntax::LineIndex;
        use oxc_span::Span;

        pub type RuleId = &'static str;

        $( pub const $rule_id: RuleId = $rule_value; )*

        pub fn known_rule_ids() -> &'static [RuleId] {
            &[$( $rule_value ),*]
        }

        /// A lint violation detected by a rule.
        ///
        /// # Field conventions
        ///
        /// - `file`: The workspace-relative or absolute path of the file with the violation.
        /// - `span`: Byte range in source text. Set for AST rules, `None` for file/directory checks.
        /// - `line` / `column`: 1-based position from `span` for AST rules, `Some(1)` for
        ///   file-level rules, `None` for directory-level rules.
        /// - `rule`: The rule's unique identifier (e.g., `"no-console"`).
        /// - `message`: Static one-line rule description shared across all violations from the rule.
        /// - `severity`: Enforcement level (Off, Info, Warn, Error).
        /// - `subject`: Names the specific code element causing the violation — a variable name,
        ///   function name, import specifier, or filename. Short and label-like. `None` when the
        ///   violation applies to the whole file or directory.
        /// - `detail`: Contextual explanation with computed values (counts, limits, paths). A
        ///   human-readable sentence supplementing the static `message`. `None` when the message
        ///   alone suffices.
        #[derive(Debug, Clone)]
        pub struct Violation {
            pub file: PathBuf,
            pub span: Option<Span>,
            pub line: Option<usize>,
            pub column: Option<usize>,
            pub rule: RuleId,
            pub message: &'static str,
            pub severity: Severity,
            pub detail: Option<String>,
            pub subject: Option<String>,
        }

        impl Violation {
            /// Create a violation from an AST span, resolving line and column automatically.
            pub fn from_span(
                file: &Path,
                line_index: &LineIndex,
                span: Span,
                rule: RuleId,
                message: &'static str,
                severity: Severity,
            ) -> Self {
                let pos = line_index.position_for(span);
                Violation {
                    file: file.to_path_buf(),
                    span: Some(span),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule,
                    message,
                    severity,
                    detail: None,
                    subject: None,
                }
            }

            /// Create a file-level violation with no span but line/column set to 1.
            pub fn for_file(
                file: &Path,
                rule: RuleId,
                message: &'static str,
                severity: Severity,
            ) -> Self {
                Violation {
                    file: file.to_path_buf(),
                    span: None,
                    line: Some(1),
                    column: Some(1),
                    rule,
                    message,
                    severity,
                    detail: None,
                    subject: None,
                }
            }

            /// Create a directory-level violation with no span, line, or column.
            pub fn for_directory(
                file: &Path,
                rule: RuleId,
                message: &'static str,
                severity: Severity,
            ) -> Self {
                Violation {
                    file: file.to_path_buf(),
                    span: None,
                    line: None,
                    column: None,
                    rule,
                    message,
                    severity,
                    detail: None,
                    subject: None,
                }
            }

            /// Attach a contextual detail message.
            pub fn with_detail(mut self, detail: String) -> Self {
                self.detail = Some(detail);
                self
            }

            /// Attach a subject identifier.
            pub fn with_subject(mut self, subject: String) -> Self {
                self.subject = Some(subject);
                self
            }
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

        impl RulesConfig {
            /// Returns a deterministic JSON representation of each rule's own
            /// configuration options. These strings are combined with project
            /// structure and architecture context to build per-rule cache hashes.
            pub fn rule_option_hashes(&self) -> std::collections::HashMap<String, String> {
                let mut result = std::collections::HashMap::new();
                $(
                    result.insert(
                        $rule_value.to_string(),
                        serde_json::to_string(&self.$mod_name)
                            .unwrap_or_else(|_| format!("{{\"severity\":\"{}\"}}", self.$mod_name.severity.as_str())),
                    );
                )*
                result
            }
        }
    };

    (@sev) => { Severity::Warn };
    (@sev $sev:expr) => { $sev };
}

use std::sync::Arc;

use crate::config::structure::DomainConfig;

declare_rules! {
    boolean_prefix => { id: BOOLEAN_PREFIX_RULE_ID, value: "boolean-prefix", path: "rules/domain/boolean_prefix.rs", config: crate::config::BooleanPrefixRuleConfig, default_severity: Severity::Info },
    component_file_only_components => { id: COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID, value: "component-file-only-components", path: "rules/domain/component_file_only_components.rs", config: crate::config::RuleConfig },
    directory_must_have_barrel => { id: DIRECTORY_MUST_HAVE_BARREL_RULE_ID, value: "directory-must-have-barrel", path: "rules/files/directory_must_have_barrel.rs", config: crate::config::BarrelRuleConfig },
    entry_file_no_logic => { id: ENTRY_FILE_NO_LOGIC_RULE_ID, value: "entry-file-no-logic", path: "rules/domain/entry_file_no_logic.rs", config: crate::config::EntryFileNoLogicRuleConfig },
    explicit_return_type => { id: EXPLICIT_RETURN_TYPE_RULE_ID, value: "explicit-return-type", path: "rules/hygiene/explicit_return_type.rs", config: crate::config::ExplicitReturnTypeRuleConfig },
    hook_no_jsx => { id: HOOK_NO_JSX_RULE_ID, value: "hook-no-jsx", path: "rules/domain/hook_no_jsx.rs", config: crate::config::RuleConfig },
    hook_prefix => { id: HOOK_PREFIX_RULE_ID, value: "hook-prefix", path: "rules/domain/hook_prefix.rs", config: crate::config::HookPrefixRuleConfig, default_severity: Severity::Info },
    layer_boundaries => { id: LAYER_BOUNDARIES_RULE_ID, value: "layer-boundaries", path: "rules/imports/layer_boundaries.rs", config: crate::config::RuleConfig, default_severity: Severity::Off },
    max_directory_depth => { id: MAX_DIRECTORY_DEPTH_RULE_ID, value: "max-directory-depth", path: "rules/files/max_directory_depth.rs", config: crate::config::MaxDirectoryDepthRuleConfig },
    max_file_exports => { id: MAX_FILE_EXPORTS_RULE_ID, value: "max-file-exports", path: "rules/exports/max_file_exports.rs", config: crate::config::FileExportsRuleConfig },
    max_function_params => { id: MAX_FUNCTION_PARAMS_RULE_ID, value: "max-function-params", path: "rules/exports/max_function_params.rs", config: crate::config::MaxFunctionParamsRuleConfig },
    max_items_per_directory => { id: MAX_ITEMS_PER_DIRECTORY_RULE_ID, value: "max-items-per-directory", path: "rules/files/max_items_per_directory.rs", config: crate::config::MaxItemsPerDirectoryRuleConfig },
    min_items_per_directory => { id: MIN_ITEMS_PER_DIRECTORY_RULE_ID, value: "min-items-per-directory", path: "rules/files/min_items_per_directory.rs", config: crate::config::MinItemsPerDirectoryRuleConfig },
    no_any => { id: NO_ANY_RULE_ID, value: "no-any", path: "rules/typescript/no_any.rs", config: crate::config::NoAnyRuleConfig },
    no_barrel_chain => { id: NO_BARREL_CHAIN_RULE_ID, value: "no-barrel-chain", path: "rules/exports/no_barrel_chain.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    no_circular_import => { id: NO_CIRCULAR_IMPORT_RULE_ID, value: "no-circular-import", path: "rules/imports/no_circular_import.rs", config: crate::config::NoCircularImportRuleConfig, default_severity: Severity::Error },
    no_barrel_files => { id: NO_BARREL_FILES_RULE_ID, value: "no-barrel-files", path: "rules/exports/no_barrel_files.rs", config: crate::config::BarrelRuleConfig, default_severity: Severity::Info },
    no_comments => { id: NO_COMMENTS_RULE_ID, value: "no-comments", path: "rules/hygiene/no_comments.rs", config: crate::config::CommentsRuleConfig, default_severity: Severity::Info },
    no_console => { id: NO_CONSOLE_RULE_ID, value: "no-console", path: "rules/hygiene/no_console.rs", config: crate::config::NoConsoleRuleConfig },
    no_debugger => { id: NO_DEBUGGER_RULE_ID, value: "no-debugger", path: "rules/hygiene/no_debugger.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_default_export => { id: NO_DEFAULT_EXPORT_RULE_ID, value: "no-default-export", path: "rules/exports/no_default_export.rs", config: crate::config::NoDefaultExportRuleConfig, default_severity: Severity::Info },
    no_dump_files => { id: NO_DUMP_FILES_RULE_ID, value: "no-dump-files", path: "rules/files/no_dump_files.rs", config: crate::config::NoDumpFilesRuleConfig },
    no_duplicate_file_names => { id: NO_DUPLICATE_FILE_NAMES_RULE_ID, value: "no-duplicate-file-names", path: "rules/files/no_duplicate_file_names.rs", config: crate::config::NoDuplicateFileNamesRuleConfig },
    no_empty_directories => { id: NO_EMPTY_DIRECTORIES_RULE_ID, value: "no-empty-directories", path: "rules/files/no_empty_directories.rs", config: crate::config::NoEmptyDirectoriesRuleConfig },
    no_empty_domain => { id: NO_EMPTY_DOMAIN_RULE_ID, value: "no-empty-domain", path: "rules/domain/no_empty_domain.rs", config: crate::config::NoEmptyDomainRuleConfig },
    no_anemic_domain => { id: NO_ANEMIC_DOMAIN_RULE_ID, value: "no-anemic-domain", path: "rules/domain/no_anemic_domain.rs", config: crate::config::NoAnemicDomainRuleConfig },
    no_god_domain => { id: NO_GOD_DOMAIN_RULE_ID, value: "no-god-domain", path: "rules/domain/no_god_domain.rs", config: crate::config::NoGodDomainRuleConfig },
    no_empty_interface => { id: NO_EMPTY_INTERFACE_RULE_ID, value: "no-empty-interface", path: "rules/typescript/no_empty_interface.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_enums => { id: NO_ENUMS_RULE_ID, value: "no-enums", path: "rules/typescript/no_enums.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    no_eval => { id: NO_EVAL_RULE_ID, value: "no-eval", path: "rules/hygiene/no_eval.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_export_star => { id: NO_EXPORT_STAR_RULE_ID, value: "no-export-star", path: "rules/exports/no_export_star.rs", config: crate::config::RuleConfig },
    no_focused_test => { id: NO_FOCUSED_TEST_RULE_ID, value: "no-focused-test", path: "rules/hygiene/no_focused_test.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_inline_types => { id: NO_INLINE_TYPES_RULE_ID, value: "no-inline-types", path: "rules/domain/no_inline_types.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    no_interface => { id: NO_INTERFACE_RULE_ID, value: "no-interface", path: "rules/typescript/no_interface.rs", config: crate::config::NoInterfaceRuleConfig, default_severity: Severity::Info },
    no_large_file => { id: NO_LARGE_FILE_RULE_ID, value: "no-large-file", path: "rules/files/no_large_file.rs", config: crate::config::FileLengthRuleConfig },
    no_logic_in_barrel => { id: NO_LOGIC_IN_BARREL_RULE_ID, value: "no-logic-in-barrel", path: "rules/exports/no_logic_in_barrel.rs", config: crate::config::BarrelRuleConfig },
    no_logic_in_domain => { id: NO_LOGIC_IN_DOMAIN_RULE_ID, value: "no-logic-in-domain", path: "rules/domain/no_logic_in_domain.rs", config: crate::config::RuleConfig },
    no_abbreviations => { id: NO_ABBREVIATIONS_RULE_ID, value: "no-abbreviations", path: "rules/hygiene/no_abbreviations.rs", config: crate::config::NoAbbreviationsRuleConfig, default_severity: Severity::Info },
    no_restricted_imports => { id: NO_RESTRICTED_IMPORTS_RULE_ID, value: "no-restricted-imports", path: "rules/imports/no_restricted_imports.rs", config: crate::config::NoRestrictedImportsRuleConfig },
    no_side_effect_imports => { id: NO_SIDE_EFFECT_IMPORTS_RULE_ID, value: "no-side-effect-imports", path: "rules/imports/no_side_effect_imports.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    sort_imports => { id: SORT_IMPORTS_RULE_ID, value: "sort-imports", path: "rules/imports/sort_imports.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    no_mutable_exports => { id: NO_MUTABLE_EXPORTS_RULE_ID, value: "no-mutable-exports", path: "rules/exports/no_mutable_exports.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    sort_exports => { id: SORT_EXPORTS_RULE_ID, value: "sort-exports", path: "rules/exports/sort_exports.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    no_namespace => { id: NO_NAMESPACE_RULE_ID, value: "no-namespace", path: "rules/typescript/no_namespace.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_nested_functions => { id: NO_NESTED_FUNCTIONS_RULE_ID, value: "no-nested-functions", path: "rules/hygiene/no_nested_functions.rs", config: crate::config::NoNestedFunctionsRuleConfig },
    no_non_null_assertion => { id: NO_NON_NULL_ASSERTION_RULE_ID, value: "no-non-null-assertion", path: "rules/typescript/no_non_null_assertion.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_await_in_loop => { id: NO_AWAIT_IN_LOOP_RULE_ID, value: "no-await-in-loop", path: "rules/hygiene/no_await_in_loop.rs", config: crate::config::RuleConfig },
    no_promise_executor_return => { id: NO_PROMISE_EXECUTOR_RETURN_RULE_ID, value: "no-promise-executor-return", path: "rules/hygiene/no_promise_executor_return.rs", config: crate::config::RuleConfig },
    no_unsafe_optional_chaining => { id: NO_UNSAFE_OPTIONAL_CHAINING_RULE_ID, value: "no-unsafe-optional-chaining", path: "rules/typescript/no_unsafe_optional_chaining.rs", config: crate::config::RuleConfig },
    no_magic_numbers => { id: NO_MAGIC_NUMBERS_RULE_ID, value: "no-magic-numbers", path: "rules/typescript/no_magic_numbers.rs", config: crate::config::NoMagicNumbersRuleConfig, default_severity: Severity::Info },
    no_orphan_files => { id: NO_ORPHAN_FILES_RULE_ID, value: "no-orphan-files", path: "rules/imports/no_orphan_files.rs", config: crate::config::NoOrphanFilesRuleConfig },
    no_package_cycle => { id: NO_PACKAGE_CYCLE_RULE_ID, value: "no-package-cycle", path: "rules/imports/no_package_cycle.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_private_package_import => { id: NO_PRIVATE_PACKAGE_IMPORT_RULE_ID, value: "no-private-package-import", path: "rules/imports/no_private_package_import.rs", config: crate::config::RuleConfig },
    no_type_assertion => { id: NO_TYPE_ASSERTION_RULE_ID, value: "no-type-assertion", path: "rules/typescript/no_type_assertion.rs", config: crate::config::RuleConfig },
    no_unnecessary_type_assertion => { id: NO_UNNECESSARY_TYPE_ASSERTION_RULE_ID, value: "no-unnecessary-type-assertion", path: "rules/typescript/no_unnecessary_type_assertion.rs", config: crate::config::RuleConfig },
    no_process_env => { id: NO_PROCESS_ENV_RULE_ID, value: "no-process-env", path: "rules/typescript/no_process_env.rs", config: crate::config::RuleConfig },
    no_silent_catch => { id: NO_SILENT_CATCH_RULE_ID, value: "no-silent-catch", path: "rules/hygiene/no_silent_catch.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_skipped_test => { id: NO_SKIPPED_TEST_RULE_ID, value: "no-skipped-test", path: "rules/hygiene/no_skipped_test.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_test_code_in_production => { id: NO_TEST_CODE_IN_PRODUCTION_RULE_ID, value: "no-test-code-in-production", path: "rules/imports/no_test_code_in_production.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_test_import => { id: NO_TEST_IMPORT_RULE_ID, value: "no-test-import", path: "rules/imports/no_test_import.rs", config: crate::config::RuleConfig, default_severity: Severity::Error },
    no_then_chain => { id: NO_THEN_CHAIN_RULE_ID, value: "no-then-chain", path: "rules/hygiene/no_then_chain.rs", config: crate::config::NoThenChainRuleConfig, default_severity: Severity::Info },
    no_upward_import => { id: NO_UPWARD_IMPORT_RULE_ID, value: "no-upward-import", path: "rules/imports/no_upward_import.rs", config: crate::config::UpwardImportRuleConfig },
    prefer_satisfies => { id: PREFER_SATISFIES_RULE_ID, value: "prefer-satisfies", path: "rules/typescript/prefer_satisfies.rs", config: crate::config::RuleConfig, default_severity: Severity::Info },
    prefer_readonly => { id: PREFER_READONLY_RULE_ID, value: "prefer-readonly", path: "rules/hygiene/prefer_readonly.rs", config: crate::config::RuleConfig },
}

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

pub trait AstRule: Send + Sync {
    fn rule_id(&self) -> RuleId;
    fn severity(&self) -> Severity;
    fn check(&self, ctx: &AstContext<'_>) -> Vec<Violation>;

    fn supports_fix(&self) -> bool {
        false
    }

    fn fix(&self, _ctx: &AstContext<'_>) -> Vec<Fix> {
        Vec::new()
    }
}

pub trait TextRule: Send + Sync {
    fn rule_id(&self) -> RuleId;
    fn severity(&self) -> Severity;
    fn check(&self, ctx: &TextContext<'_>) -> Vec<Violation>;
}

pub trait GraphRule: Send + Sync {
    fn rule_id(&self) -> RuleId;
    fn severity(&self) -> Severity;
    fn check(&self, ctx: &GraphContext<'_>) -> Vec<Violation>;
}

pub struct AstContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    pub program: &'a oxc_ast::ast::Program<'a>,
    pub line_index: &'a LineIndex,
    pub type_location_style: TypeLocationStyle,
}

pub struct TextContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    pub line_index: &'a LineIndex,
    pub type_location_style: TypeLocationStyle,
}

pub struct GraphContext<'a> {
    pub file: &'a Path,
    pub line_index: &'a LineIndex,
    pub type_location_style: TypeLocationStyle,
    pub import_graph: Arc<ImportGraph>,
    pub workspace: Option<Arc<crate::workspace::Workspace>>,
}

pub struct FileRuleSet {
    pub ast_rules: Vec<Box<dyn AstRule + Send + Sync>>,
    pub text_rules: Vec<Box<dyn TextRule + Send + Sync>>,
    pub graph_rules: Vec<Box<dyn GraphRule + Send + Sync>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeLocationStyle {
    allows_type_files: bool,
    allows_types_directories: bool,
}

impl TypeLocationStyle {
    pub fn detect(files: &[PathBuf], types: &DomainConfig) -> Self {
        let allows_type_files = files.iter().any(|file| is_type_file(file, types));
        let allows_types_directories = files.iter().any(|file| is_in_types_directory(file, types));

        // Without a types directory convention, keep colocated type files valid.
        Self {
            allows_type_files: allows_type_files || !allows_types_directories,
            allows_types_directories,
        }
    }

    fn allows_file(self, file: &Path, types: &DomainConfig) -> bool {
        is_declaration_file(file)
            || (self.allows_type_files && is_type_file(file, types))
            || (self.allows_types_directories && is_in_types_directory(file, types))
    }
}

fn is_type_file(file: &Path, types: &DomainConfig) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            types
                .file_suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix.as_str()))
        })
}

fn is_declaration_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".d.ts"))
}

fn is_in_types_directory(file: &Path, types: &DomainConfig) -> bool {
    file.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if types.folders.iter().any(|folder| name.to_str() == Some(folder))
        )
    })
}

pub use crate::rules_runner::{
    check_directory_rules, check_files, check_files_for_benchmark, check_files_with_parallelism,
};
