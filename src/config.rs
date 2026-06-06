pub mod architecture;
mod configset;
mod defaults;
pub(crate) mod raw;
mod resolve;
pub mod rules;
pub mod structure;

pub use configset::{ConfigSet, ConfigSetOptions};
pub use resolve::{ProjectConfig, write_default_config};
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
