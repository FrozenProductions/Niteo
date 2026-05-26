mod defaults;
mod raw;
mod resolve;
pub mod rules;

pub use resolve::{ProjectConfig, write_default_config};
pub use rules::{
    BooleanPrefixRuleConfig, CommentsRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig,
    GitignoreConfig, HookPrefixRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig, NoConsoleRuleConfig,
    NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig,
    NoInterfaceRuleConfig, NoLogicInDomainRuleConfig, RuleConfig, Severity, UpwardImportRuleConfig,
};
