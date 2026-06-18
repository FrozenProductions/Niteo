pub mod architecture;
mod configset;
pub mod defaults;
pub mod presets;
pub(crate) mod raw;
mod resolve;
pub mod rule_metadata;
pub mod rules;
pub mod structure;
pub mod validation;

pub use configset::{ConfigSet, ConfigSetOptions};
pub use resolve::{ProjectConfig, resolve_baseline_path, write_default_config};
pub use rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, EntryFileNoLogicRuleConfig, FileExportsRuleConfig,
    FileLengthRuleConfig, GitignoreConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxFunctionParamsRuleConfig, MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig,
    NoAbbreviationsRuleConfig, NoAnemicDomainRuleConfig, NoAnyRuleConfig, NoConsoleRuleConfig,
    NoDefaultExportRuleConfig, NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig,
    NoEmptyDirectoriesRuleConfig, NoEmptyDomainRuleConfig, NoGodDomainRuleConfig,
    NoInterfaceRuleConfig, NoMagicNumbersRuleConfig, NoNestedFunctionsRuleConfig,
    NoOrphanFilesRuleConfig, NoRestrictedImportsRuleConfig, RuleConfig, Severity,
    UpwardImportRuleConfig,
};
