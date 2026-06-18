use std::collections::HashMap;

use serde::Deserialize;

use super::super::architecture::LayerBoundaryConfig;
use super::super::structure::DomainConfig;
use super::domain::RawDomainConfig;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawArchitectureConfig {
    pub layers: Option<RawLayerBoundaryConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawLayerBoundaryConfig {
    pub order: Option<Vec<String>>,
    #[serde(flatten)]
    pub definitions: HashMap<String, RawDomainConfig>,
}

impl RawLayerBoundaryConfig {
    pub(super) fn merge(
        parent: Option<&RawLayerBoundaryConfig>,
        child: Option<&RawLayerBoundaryConfig>,
    ) -> Option<RawLayerBoundaryConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => {
                let order = child.order.clone().or_else(|| parent.order.clone());
                let mut definitions = parent.definitions.clone();
                for (name, child_def) in &child.definitions {
                    definitions.insert(name.clone(), child_def.clone());
                }
                Some(RawLayerBoundaryConfig { order, definitions })
            }
        }
    }

    pub(super) fn to_layer_boundary_config(&self) -> LayerBoundaryConfig {
        let order = self.order.clone().unwrap_or_default();
        let mut definitions: HashMap<String, DomainConfig> = HashMap::new();

        for name in &order {
            if let Some(raw_domain) = self.definitions.get(name) {
                let domain = raw_domain.to_domain_config(&DomainConfig {
                    folders: vec![],
                    file_suffixes: vec![],
                });
                definitions.insert(name.clone(), domain);
            }
        }

        LayerBoundaryConfig { order, definitions }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};

    use super::super::config::RawConfig;

    #[test]
    fn default_config_parses_architecture() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["app", "features", "entities", "shared"]

[architecture.layers.app]
folders = ["app"]

[architecture.layers.features]
folders = ["features"]

[architecture.layers.entities]
folders = ["entities"]

[architecture.layers.shared]
folders = ["shared"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert_eq!(
            arch.layers.order,
            vec!["app", "features", "entities", "shared"]
        );
        assert_eq!(arch.layers.definitions.len(), 4);
        assert_eq!(
            arch.layers
                .definitions
                .get("shared")
                .context("expected shared layer")?
                .folders,
            vec!["shared"]
        );
        Ok(())
    }

    #[test]
    fn missing_architecture_uses_defaults() -> Result<()> {
        let source = r#"
[project]
root = "src"
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert!(arch.layers.order.is_empty());
        Ok(())
    }

    #[test]
    fn architecture_layer_order_only_defines_layers() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["shared", "core"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        assert_eq!(arch.layers.order, vec!["shared", "core"]);
        assert!(arch.layers.definitions.is_empty());
        Ok(())
    }

    #[test]
    fn architecture_layer_with_suffixes() -> Result<()> {
        let source = r#"
[architecture.layers]
order = ["features"]

[architecture.layers.features]
folders = ["features"]
file-suffixes = [".feature.ts"]
"#;
        let raw: RawConfig = toml::from_str(source).context("valid config")?;
        let arch = raw.architecture();
        let def = arch
            .layers
            .definitions
            .get("features")
            .context("expected features layer")?;
        assert_eq!(def.folders, vec!["features"]);
        assert_eq!(def.file_suffixes, vec![".feature.ts"]);
        Ok(())
    }
}
