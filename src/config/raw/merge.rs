use std::collections::HashMap;

use super::architecture::RawLayerBoundaryConfig;
use super::config::{RawConfig, RawProjectConfig, RawProjectStructure};
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
                hooks: child.hooks.clone().or_else(|| parent.hooks.clone()),
                components: child
                    .components
                    .clone()
                    .or_else(|| parent.components.clone()),
                types: child.types.clone().or_else(|| parent.types.clone()),
                constants: child.constants.clone().or_else(|| parent.constants.clone()),
                tests: child.tests.clone().or_else(|| parent.tests.clone()),
                generated: child.generated.clone().or_else(|| parent.generated.clone()),
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
}
