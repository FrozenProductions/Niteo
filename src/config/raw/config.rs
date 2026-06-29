use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

use super::super::architecture::ArchitectureConfig;
use super::super::fail_on::{FailurePolicy, FailureThreshold};
use super::super::rules::GitignoreConfig;
use super::super::structure::ProjectStructureConfig;
use super::architecture::RawArchitectureConfig;
use super::domain::RawDomainConfig;
use super::rules::RawRuleConfig;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RawConfig {
    pub project: Option<RawProjectConfig>,
    pub architecture: Option<RawArchitectureConfig>,
    pub rules: Option<HashMap<String, RawRuleConfig>>,
    pub fix: Option<HashMap<String, bool>>,
    #[serde(rename = "fail-on")]
    pub fail_on: Option<RawFailOnConfig>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct RawFailOnConfig {
    pub default: Option<String>,
    pub rules: Option<HashMap<String, String>>,
    pub categories: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawProjectConfig {
    pub root: Option<PathBuf>,
    #[serde(rename = "respect-gitignore")]
    pub respect_gitignore: Option<bool>,
    pub history: Option<bool>,
    pub baseline: Option<PathBuf>,
    pub structure: Option<RawProjectStructure>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawProjectStructure {
    pub hooks: Option<RawDomainConfig>,
    pub components: Option<RawDomainConfig>,
    pub types: Option<RawDomainConfig>,
    pub constants: Option<RawDomainConfig>,
    pub tests: Option<RawDomainConfig>,
    pub generated: Option<RawDomainConfig>,
}

impl RawConfig {
    pub fn architecture(&self) -> ArchitectureConfig {
        let raw_arch = self.architecture.as_ref();
        let layers = raw_arch
            .and_then(|arch| arch.layers.as_ref())
            .map(|raw| raw.to_layer_boundary_config())
            .unwrap_or_default();

        ArchitectureConfig { layers }
    }

    pub fn gitignore(&self) -> GitignoreConfig {
        let project = self.project.as_ref();
        GitignoreConfig {
            enabled: project
                .and_then(|project| project.respect_gitignore)
                .unwrap_or_default(),
        }
    }

    pub fn structure(&self) -> ProjectStructureConfig {
        let raw_structure = self
            .project
            .as_ref()
            .and_then(|project| project.structure.as_ref());

        let defaults = ProjectStructureConfig::default();

        ProjectStructureConfig {
            hooks: raw_structure
                .and_then(|structure| structure.hooks.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.hooks))
                .unwrap_or(defaults.hooks),
            components: raw_structure
                .and_then(|structure| structure.components.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.components))
                .unwrap_or(defaults.components),
            types: raw_structure
                .and_then(|structure| structure.types.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.types))
                .unwrap_or(defaults.types),
            constants: raw_structure
                .and_then(|structure| structure.constants.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.constants))
                .unwrap_or(defaults.constants),
            tests: raw_structure
                .and_then(|structure| structure.tests.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.tests))
                .unwrap_or(defaults.tests),
            generated: raw_structure
                .and_then(|structure| structure.generated.as_ref())
                .map(|domain| domain.to_domain_config(&defaults.generated))
                .unwrap_or(defaults.generated),
        }
    }

    pub fn fail_on_policy(&self) -> Result<FailurePolicy, String> {
        let Some(config) = self.fail_on.as_ref() else {
            return Ok(FailurePolicy::default());
        };

        let default = config
            .default
            .as_deref()
            .map(|value| value.parse::<FailureThreshold>())
            .transpose()?;

        let rules = parse_fail_on_thresholds(config.rules.as_ref())?;
        let categories = parse_fail_on_categories(config.categories.as_ref())?;

        Ok(FailurePolicy {
            default: default.unwrap_or_default(),
            rules,
            categories,
        })
    }
}

fn parse_fail_on_thresholds(
    raw: Option<&HashMap<String, String>>,
) -> Result<HashMap<String, FailureThreshold>, String> {
    let Some(raw) = raw else {
        return Ok(HashMap::new());
    };

    let mut thresholds = HashMap::with_capacity(raw.len());
    for (rule, value) in raw {
        let threshold = value
            .parse::<FailureThreshold>()
            .map_err(|error| format!("in fail-on rule '{rule}': {error}"))?;
        thresholds.insert(rule.clone(), threshold);
    }

    Ok(thresholds)
}

fn parse_fail_on_categories(
    raw: Option<&HashMap<String, String>>,
) -> Result<HashMap<crate::config::rule_metadata::RuleCategory, FailureThreshold>, String> {
    let Some(raw) = raw else {
        return Ok(HashMap::new());
    };

    let mut thresholds = HashMap::with_capacity(raw.len());
    for (category, value) in raw {
        let parsed_category = crate::config::rule_metadata::RuleCategory::from_str(category)
            .map_err(|error| format!("in fail-on category '{category}': {error}"))?;
        let threshold = value
            .parse::<FailureThreshold>()
            .map_err(|error| format!("in fail-on category '{category}': {error}"))?;
        thresholds.insert(parsed_category, threshold);
    }

    Ok(thresholds)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::super::super::structure::ProjectStructureConfig;
    use super::RawConfig;

    #[test]
    fn default_config_parses_structure() -> Result<()> {
        let source = r#"
[project]
root = "src"

[project.structure.hooks]
folders = ["hooks"]
file-suffixes = [".hook.ts", ".hooks.ts"]

[project.structure.components]
folders = ["components"]
file-suffixes = [".component.tsx", ".components.tsx"]

[project.structure.types]
folders = ["types"]
file-suffixes = [".type.ts", ".types.ts"]

[project.structure.constants]
folders = ["constants"]
file-suffixes = [".constant.ts", ".constants.ts"]

[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts", ".tests.ts"]
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".hook.ts", ".hooks.ts"]);
        assert_eq!(structure.components.folders, vec!["components"]);
        assert_eq!(structure.types.folders, vec!["types"]);
        assert_eq!(structure.constants.folders, vec!["constants"]);
        assert_eq!(structure.tests.folders, vec!["tests"]);
        assert_eq!(structure.tests.file_suffixes, vec![".test.ts", ".tests.ts"]);
        Ok(())
    }

    #[test]
    fn missing_structure_uses_defaults() -> Result<()> {
        let source = r#"
[project]
root = "src"
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let structure = raw.structure();
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.hooks.folders, defaults.hooks.folders);
        assert_eq!(structure.hooks.file_suffixes, defaults.hooks.file_suffixes);
        assert_eq!(structure.components.folders, defaults.components.folders);
        assert_eq!(structure.types.folders, defaults.types.folders);
        assert_eq!(structure.constants.folders, defaults.constants.folders);
        assert_eq!(structure.tests.folders, defaults.tests.folders);
        assert_eq!(structure.tests.file_suffixes, defaults.tests.file_suffixes);
        Ok(())
    }

    #[test]
    fn custom_structure_overrides_defaults() -> Result<()> {
        let source = r#"
[project.structure.hooks]
folders = ["custom-hooks"]
file-suffixes = [".custom.ts"]
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let structure = raw.structure();
        assert_eq!(structure.hooks.folders, vec!["custom-hooks"]);
        assert_eq!(structure.hooks.file_suffixes, vec![".custom.ts"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.components.folders, defaults.components.folders);
        Ok(())
    }

    #[test]
    fn partial_domain_config_preserves_defaults() -> Result<()> {
        let source = r#"
[project.structure.types]
folders = ["typings"]
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let structure = raw.structure();
        assert_eq!(structure.types.folders, vec!["typings"]);
        let defaults = ProjectStructureConfig::default();
        assert_eq!(structure.types.file_suffixes, defaults.types.file_suffixes);
        Ok(())
    }

    #[test]
    fn fail_on_policy_parses_defaults_rules_and_categories() -> Result<()> {
        use crate::config::{FailureThreshold, RuleCategory};

        let source = r#"
[fail-on]
default = "error"

[fail-on.rules]
no-console = "warn"

[fail-on.categories]
hygiene = "warn"
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let policy = raw.fail_on_policy().map_err(anyhow::Error::msg)?;

        assert_eq!(policy.default, FailureThreshold::Error);
        assert_eq!(
            policy.rules.get("no-console"),
            Some(&FailureThreshold::Warn)
        );
        assert_eq!(
            policy.categories.get(&RuleCategory::SourceHygiene),
            Some(&FailureThreshold::Warn)
        );
        Ok(())
    }

    #[test]
    fn fail_on_policy_rejects_invalid_threshold() -> Result<()> {
        let source = r#"
[fail-on]
default = "warning"
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let error = raw
            .fail_on_policy()
            .expect_err("invalid threshold should fail");
        assert!(error.contains("warning"));
        assert!(error.contains("error, warn, any"));
        Ok(())
    }

    #[test]
    fn fail_on_policy_rejects_invalid_category() -> Result<()> {
        let source = r#"
[fail-on.categories]
unknown = "warn"
"#;
        let raw: RawConfig = toml::from_str(source)?;
        let error = raw
            .fail_on_policy()
            .expect_err("invalid category should fail");
        assert!(error.contains("unknown"));
        assert!(error.contains("domain"));
        Ok(())
    }
}
