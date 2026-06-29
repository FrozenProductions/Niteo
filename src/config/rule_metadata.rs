#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCategory {
    LanguageTypescript,
    SourceHygiene,
    ExportModuleShape,
    FileDirectory,
    Domain,
    Import,
}

impl RuleCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleCategory::LanguageTypescript => "typescript",
            RuleCategory::SourceHygiene => "hygiene",
            RuleCategory::ExportModuleShape => "exports",
            RuleCategory::FileDirectory => "files",
            RuleCategory::Domain => "domain",
            RuleCategory::Import => "imports",
        }
    }
}

impl std::str::FromStr for RuleCategory {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "typescript" => Ok(Self::LanguageTypescript),
            "hygiene" => Ok(Self::SourceHygiene),
            "exports" => Ok(Self::ExportModuleShape),
            "files" => Ok(Self::FileDirectory),
            "domain" => Ok(Self::Domain),
            "imports" => Ok(Self::Import),
            _ => Err(format!(
                "unknown category '{value}'; must be one of: typescript, hygiene, exports, files, domain, imports"
            )),
        }
    }
}

pub(crate) type RuleMetadata = crate::rule_documentation::catalog::RuleDocumentation;

pub(crate) fn all_rule_metadata() -> &'static [RuleMetadata] {
    crate::rule_documentation::catalog::all_rules()
}

pub(crate) fn rule_by_id(rule_id: &str) -> Option<&'static RuleMetadata> {
    crate::rule_documentation::catalog::find_rule(rule_id)
}

pub(crate) fn known_option_names_for_rule(rule_id: &str) -> Option<Vec<&'static str>> {
    rule_by_id(rule_id).map(|metadata| {
        metadata
            .options
            .iter()
            .map(|option| option.name)
            .filter(|name| !name.contains('.'))
            .collect()
    })
}
