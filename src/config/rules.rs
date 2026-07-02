#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Off,
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "off" => Ok(Self::Off),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(format!(
                "invalid severity '{value}'; must be one of: off, info, warn, error"
            )),
        }
    }
}

impl Severity {
    pub fn is_enabled(self) -> bool {
        self != Self::Off
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub severity: Severity,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommentsRuleConfig {
    pub severity: Severity,
    pub allow_doc_comments: bool,
}

impl Default for CommentsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_doc_comments: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileLengthRuleConfig {
    pub severity: Severity,
    pub max_lines: usize,
}

impl Default for FileLengthRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_lines: 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileExportsRuleConfig {
    pub severity: Severity,
    pub max_exports: usize,
    pub count_default: bool,
}

impl Default for FileExportsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_exports: 10,
            count_default: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpwardImportRuleConfig {
    pub severity: Severity,
    pub max_depth: usize,
    pub allow_patterns: Vec<String>,
}

impl Default for UpwardImportRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_depth: 0,
            allow_patterns: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitignoreConfig {
    pub enabled: bool,
}

impl Default for GitignoreConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone)]
pub struct NoEmptyDirectoriesRuleConfig {
    pub severity: Severity,
    pub ignore_dirs: Vec<String>,
}

impl Default for NoEmptyDirectoriesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoDuplicateFileNamesRuleConfig {
    pub severity: Severity,
    pub ignore_names: Vec<String>,
}

impl Default for NoDuplicateFileNamesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            ignore_names: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxItemsPerDirectoryRuleConfig {
    pub severity: Severity,
    pub max_items: usize,
    pub ignore_dirs: Vec<String>,
    pub count_folders: bool,
}

impl Default for MaxItemsPerDirectoryRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_items: 20,
            ignore_dirs: vec![],
            count_folders: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinItemsPerDirectoryRuleConfig {
    pub severity: Severity,
    pub min_items: usize,
    pub ignore_dirs: Vec<String>,
    pub count_folders: bool,
}

impl Default for MinItemsPerDirectoryRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            min_items: 3,
            ignore_dirs: vec![],
            count_folders: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxDirectoryDepthRuleConfig {
    pub severity: Severity,
    pub max_depth: usize,
    pub ignore_dirs: Vec<String>,
}

impl Default for MaxDirectoryDepthRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_depth: 5,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoConsoleRuleConfig {
    pub severity: Severity,
    pub allow_patterns: Vec<String>,
}

impl Default for NoConsoleRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_patterns: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BooleanPrefixRuleConfig {
    pub severity: Severity,
    pub prefixes: Vec<String>,
    pub ignore_constants: bool,
}

impl Default for BooleanPrefixRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            prefixes: vec![],
            ignore_constants: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HookPrefixRuleConfig {
    pub severity: Severity,
    pub prefixes: Vec<String>,
}

impl Default for HookPrefixRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            prefixes: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoInterfaceRuleConfig {
    pub severity: Severity,
    pub allow_declaration_merging: bool,
}

impl Default for NoInterfaceRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allow_declaration_merging: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoDumpFilesRuleConfig {
    pub severity: Severity,
    pub extra_names: Vec<String>,
}

impl Default for NoDumpFilesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            extra_names: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoAnyRuleConfig {
    pub severity: Severity,
    pub allowed_folders: Vec<String>,
}

impl Default for NoAnyRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allowed_folders: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntryFileNoLogicRuleConfig {
    pub severity: Severity,
    pub entry_files: Vec<String>,
}

impl Default for EntryFileNoLogicRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            entry_files: vec![
                "main".to_string(),
                "app".to_string(),
                "layout".to_string(),
                "page".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoAbbreviationsRuleConfig {
    pub severity: Severity,
    pub extra_abbreviations: Vec<String>,
    pub allow_abbreviations: Vec<String>,
}

impl Default for NoAbbreviationsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoDefaultExportRuleConfig {
    pub severity: Severity,
    pub components_only: bool,
}

impl Default for NoDefaultExportRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            components_only: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoRestrictedImportsRuleConfig {
    pub severity: Severity,
    pub restricted: Vec<String>,
}

impl Default for NoRestrictedImportsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            restricted: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaxFunctionParamsRuleConfig {
    pub severity: Severity,
    pub max_params: usize,
}

impl Default for MaxFunctionParamsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_params: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestingContext {
    Function,
    Arrow,
    ClassMethod,
    ObjectMethod,
}

impl std::str::FromStr for NestingContext {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "function" => Ok(Self::Function),
            "arrow" => Ok(Self::Arrow),
            "class-method" => Ok(Self::ClassMethod),
            "object-method" => Ok(Self::ObjectMethod),
            _ => Err(format!(
                "unknown nesting context '{s}', expected one of: function, arrow, class-method, object-method"
            )),
        }
    }
}

impl std::fmt::Display for NestingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Arrow => write!(f, "arrow"),
            Self::ClassMethod => write!(f, "class-method"),
            Self::ObjectMethod => write!(f, "object-method"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoNestedFunctionsRuleConfig {
    pub severity: Severity,
    pub max_depth: usize,
    pub contexts: Vec<NestingContext>,
}

impl Default for NoNestedFunctionsRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_depth: 2,
            contexts: vec![
                NestingContext::Function,
                NestingContext::Arrow,
                NestingContext::ClassMethod,
                NestingContext::ObjectMethod,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoOrphanFilesRuleConfig {
    pub severity: Severity,
    pub entry_files: Vec<String>,
}

impl Default for NoOrphanFilesRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            entry_files: vec![
                "main".to_string(),
                "app".to_string(),
                "layout".to_string(),
                "page".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoMagicNumbersRuleConfig {
    pub severity: Severity,
    pub allowed_numbers: Vec<String>,
    pub enforce_strings: bool,
}

impl Default for NoMagicNumbersRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            allowed_numbers: vec![],
            enforce_strings: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoEmptyDomainRuleConfig {
    pub severity: Severity,
    pub ignore_dirs: Vec<String>,
}

impl Default for NoEmptyDomainRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoAnemicDomainRuleConfig {
    pub severity: Severity,
    pub max_files: usize,
    pub ignore_dirs: Vec<String>,
}

impl Default for NoAnemicDomainRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_files: 1,
            ignore_dirs: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoGodDomainRuleConfig {
    pub severity: Severity,
    pub max_files: usize,
    pub ignore_dirs: Vec<String>,
}

impl Default for NoGodDomainRuleConfig {
    fn default() -> Self {
        Self {
            severity: Severity::Warn,
            max_files: 20,
            ignore_dirs: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Severity;

    #[test]
    fn from_str_parses_valid_severities() {
        assert_eq!("off".parse::<Severity>().unwrap(), Severity::Off);
        assert_eq!("info".parse::<Severity>().unwrap(), Severity::Info);
        assert_eq!("warn".parse::<Severity>().unwrap(), Severity::Warn);
        assert_eq!("error".parse::<Severity>().unwrap(), Severity::Error);
    }

    #[test]
    fn from_str_rejects_invalid_severity() {
        let error = "warning".parse::<Severity>().unwrap_err();
        assert!(error.contains("'warning'"));
        assert!(error.contains("off, info, warn, error"));
    }
}
