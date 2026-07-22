pub mod architecture;
mod configset;
pub mod defaults;
pub mod fail_on;
pub mod presets;
pub(crate) mod raw;
mod resolve;
pub mod rule_metadata;
pub mod rules;
pub mod structure;
pub mod validation;

pub use configset::{ConfigSet, ConfigSetOptions};
pub use fail_on::{FailurePolicy, FailureThreshold};
pub use resolve::{ProjectConfig, resolve_baseline_path, write_default_config};
pub use rule_metadata::RuleCategory;
pub use rules::{
    BarrelRuleConfig, BooleanPrefixRuleConfig, CommentsRuleConfig, EntryFileNoLogicRuleConfig,
    ExplicitReturnTypeRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig, GitignoreConfig,
    HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig, MaxFunctionParamsRuleConfig,
    MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig, NestingContext,
    NoAbbreviationsRuleConfig, NoAnemicDomainRuleConfig, NoAnyRuleConfig,
    NoCircularImportRuleConfig, NoConsoleRuleConfig, NoDefaultExportRuleConfig,
    NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig,
    NoEmptyDomainRuleConfig, NoGodDomainRuleConfig, NoInterfaceRuleConfig,
    NoMagicNumbersRuleConfig, NoNestedFunctionsRuleConfig, NoOrphanFilesRuleConfig,
    NoRestrictedImportsRuleConfig, NoThenChainRuleConfig, RestrictedImportPattern, RuleConfig,
    Severity, UpwardImportRuleConfig,
};
