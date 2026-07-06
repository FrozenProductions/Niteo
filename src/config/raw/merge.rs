use std::collections::HashMap;

use super::architecture::RawLayerBoundaryConfig;
use super::config::{RawConfig, RawFailOnConfig, RawProjectConfig, RawProjectStructure};
use super::domain::RawDomainConfig;
use super::rules::RawRuleConfig;

impl RawConfig {
    pub fn merge(parent: &RawConfig, child: &RawConfig) -> RawConfig {
        RawConfig {
            project: Self::merge_project(parent.project.as_ref(), child.project.as_ref()),
            architecture: Self::merge_architecture(
                parent.architecture.as_ref(),
                child.architecture.as_ref(),
            ),
            rules: Self::merge_rules(parent.rules.as_ref(), child.rules.as_ref()),
            fix: Self::merge_fix(parent.fix.as_ref(), child.fix.as_ref()),
            fail_on: Self::merge_fail_on(parent.fail_on.as_ref(), child.fail_on.as_ref()),
        }
    }

    fn merge_fix(
        parent: Option<&HashMap<String, bool>>,
        child: Option<&HashMap<String, bool>>,
    ) -> Option<HashMap<String, bool>> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => {
                let mut merged = parent.clone();
                merged.extend(child.iter().map(|(k, v)| (k.clone(), *v)));
                Some(merged)
            }
        }
    }

    fn merge_project(
        parent: Option<&RawProjectConfig>,
        child: Option<&RawProjectConfig>,
    ) -> Option<RawProjectConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawProjectConfig {
                root: parent.root.clone(),
                respect_gitignore: child.respect_gitignore.or(parent.respect_gitignore),
                history: child.history.or(parent.history),
                baseline: child.baseline.clone().or_else(|| parent.baseline.clone()),
                structure: Self::merge_structure(
                    parent.structure.as_ref(),
                    child.structure.as_ref(),
                ),
            }),
        }
    }

    fn merge_architecture(
        parent: Option<&super::architecture::RawArchitectureConfig>,
        child: Option<&super::architecture::RawArchitectureConfig>,
    ) -> Option<super::architecture::RawArchitectureConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(super::architecture::RawArchitectureConfig {
                layers: RawLayerBoundaryConfig::merge(
                    parent.layers.as_ref(),
                    child.layers.as_ref(),
                ),
            }),
        }
    }

    fn merge_structure(
        parent: Option<&RawProjectStructure>,
        child: Option<&RawProjectStructure>,
    ) -> Option<RawProjectStructure> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawProjectStructure {
                hooks: RawDomainConfig::merge_option(parent.hooks.as_ref(), child.hooks.as_ref()),
                components: RawDomainConfig::merge_option(
                    parent.components.as_ref(),
                    child.components.as_ref(),
                ),
                types: RawDomainConfig::merge_option(parent.types.as_ref(), child.types.as_ref()),
                constants: RawDomainConfig::merge_option(
                    parent.constants.as_ref(),
                    child.constants.as_ref(),
                ),
                tests: RawDomainConfig::merge_option(parent.tests.as_ref(), child.tests.as_ref()),
                generated: RawDomainConfig::merge_option(
                    parent.generated.as_ref(),
                    child.generated.as_ref(),
                ),
            }),
        }
    }

    fn merge_rules(
        parent: Option<&HashMap<String, RawRuleConfig>>,
        child: Option<&HashMap<String, RawRuleConfig>>,
    ) -> Option<HashMap<String, RawRuleConfig>> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => {
                let mut merged = parent.clone();
                for (name, child_rule) in child {
                    match merged.get(name) {
                        Some(parent_rule) => {
                            merged.insert(
                                name.clone(),
                                RawRuleConfig::merge(parent_rule, child_rule),
                            );
                        }
                        None => {
                            merged.insert(name.clone(), child_rule.clone());
                        }
                    }
                }
                Some(merged)
            }
        }
    }

    fn merge_fail_on(
        parent: Option<&RawFailOnConfig>,
        child: Option<&RawFailOnConfig>,
    ) -> Option<RawFailOnConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(RawFailOnConfig {
                default: child.default.clone().or_else(|| parent.default.clone()),
                rules: merge_optional_string_maps(parent.rules.as_ref(), child.rules.as_ref()),
                categories: merge_optional_string_maps(
                    parent.categories.as_ref(),
                    child.categories.as_ref(),
                ),
            }),
        }
    }
}

fn merge_optional_string_maps(
    parent: Option<&HashMap<String, String>>,
    child: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    match (parent, child) {
        (None, None) => None,
        (Some(parent), None) => Some(parent.clone()),
        (None, Some(child)) => Some(child.clone()),
        (Some(parent), Some(child)) => {
            let mut merged = parent.clone();
            merged.extend(
                child
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
            Some(merged)
        }
    }
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::{Context, Result};

    use super::super::super::rules::Severity;
    use super::RawConfig;

    #[test]
    fn merge_child_severity_overrides_parent_options() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "warn"
allow-patterns = ["debug"]
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "error"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_console.severity, Severity::Error);
        Ok(())
    }

    #[test]
    fn merge_child_options_partial_override() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-large-file]
severity = "warn"
max-lines = 300
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-large-file]
severity = "error"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_large_file.severity, Severity::Error);
        assert_eq!(rules_config.no_large_file.max_lines, 300);
        Ok(())
    }

    #[test]
    fn merge_inherits_parent_rules_not_in_child() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "warn"
[rules.no-debugger]
severity = "error"
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[rules.no-console]
severity = "off"
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let rules_config = merged.rules_config().map_err(anyhow::Error::msg)?;
        assert_eq!(rules_config.no_console.severity, Severity::Off);
        assert_eq!(rules_config.no_debugger.severity, Severity::Error);
        Ok(())
    }

    #[test]
    fn merge_structure_child_overrides_parent_domain() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts"]
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
folders = ["__tests__"]
file-suffixes = [".spechild.ts"]
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let structure = merged.structure();
        assert_eq!(structure.tests.folders, vec!["__tests__"]);
        assert_eq!(structure.tests.file_suffixes, vec![".spechild.ts"]);
        Ok(())
    }

    #[test]
    fn merge_structure_partial_child_preserves_parent_field() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
folders = ["tests"]
file-suffixes = [".test.ts"]
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[project.structure.tests]
file-suffixes = [".spec.ts"]
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let structure = merged.structure();
        assert_eq!(structure.tests.folders, vec!["tests"]);
        assert_eq!(structure.tests.file_suffixes, vec![".spec.ts"]);
        Ok(())
    }

    #[test]
    fn merge_structure_preserves_parent_root() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[project]
root = "src"
respect-gitignore = true
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[project]
respect-gitignore = false
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let project = merged.project.as_ref().context("expected project")?;
        assert_eq!(project.root, Some(PathBuf::from("src")));
        assert_eq!(project.respect_gitignore, Some(false));
        Ok(())
    }

    #[test]
    fn merge_fix_child_overrides_parent() -> Result<()> {
        let parent: RawConfig = toml::from_str(
            r#"
[fix]
no-debugger = false
no-focused-test = true
"#,
        )?;
        let child: RawConfig = toml::from_str(
            r#"
[fix]
no-debugger = true
"#,
        )?;

        let merged = RawConfig::merge(&parent, &child);
        let fix = merged.fix.context("expected fix")?;
        assert_eq!(fix.get("no-debugger"), Some(&true));
        assert_eq!(fix.get("no-focused-test"), Some(&true));
        Ok(())
    }
}
