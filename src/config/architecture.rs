use serde::Serialize;
use std::collections::HashMap;
use std::path::{Component, Path};

use super::structure::DomainConfig;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ArchitectureConfig {
    pub layers: LayerBoundaryConfig,
}

#[derive(Debug, Clone, Default)]
pub struct LayerBoundaryConfig {
    pub order: Vec<String>,
    pub definitions: HashMap<String, DomainConfig>,
}

impl Serialize for LayerBoundaryConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LayerBoundaryConfig", 2)?;
        state.serialize_field("order", &self.order)?;
        let definitions: std::collections::BTreeMap<String, &DomainConfig> = self
            .definitions
            .iter()
            .map(|(name, domain)| (name.clone(), domain))
            .collect();
        state.serialize_field("definitions", &definitions)?;
        state.end()
    }
}

impl LayerBoundaryConfig {
    pub fn is_configured(&self) -> bool {
        !self.order.is_empty()
    }

    pub fn layer_for_file(&self, path: &Path) -> Option<&str> {
        let path_components: Vec<&str> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect();

        let mut best: Option<(&str, usize)> = None;

        for name in &self.order {
            if let Some(domain) = self.definitions.get(name) {
                if !domain.folders.is_empty() {
                    for folder in &domain.folders {
                        if folder.is_empty() {
                            continue;
                        }
                        let folder_components: Vec<&str> = folder.split('/').collect();
                        if path_components
                            .windows(folder_components.len())
                            .any(|window| window == folder_components.as_slice())
                        {
                            let specificity = folder.len();
                            match best {
                                Some((_, existing)) if specificity > existing => {
                                    best = Some((name.as_str(), specificity));
                                }
                                None => {
                                    best = Some((name.as_str(), specificity));
                                }
                                _ => {}
                            }
                        }
                    }
                } else if domain.matches_file(path) && best.is_none() {
                    best = Some((name.as_str(), 0));
                }
            }
        }

        best.map(|(name, _)| name)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    #[test]
    fn empty_config_is_unconfigured() -> Result<()> {
        let config = LayerBoundaryConfig::default();
        assert!(!config.is_configured());

        Ok(())
    }

    #[test]
    fn configured_when_order_exists() -> Result<()> {
        let config = LayerBoundaryConfig {
            order: vec!["app".to_string()],
            ..Default::default()
        };
        assert!(config.is_configured());

        Ok(())
    }

    #[test]
    fn layer_for_file_matches_folder() -> Result<()> {
        let config = make_test_layers();
        let path = Path::new("src/shared/date.ts");
        assert_eq!(config.layer_for_file(path), Some("shared"));

        Ok(())
    }

    #[test]
    fn layer_for_file_matches_nested() -> Result<()> {
        let config = make_test_layers();
        let path = Path::new("src/app/auth/login.ts");
        assert_eq!(config.layer_for_file(path), Some("app"));

        Ok(())
    }

    #[test]
    fn layer_for_file_unknown_returns_none() -> Result<()> {
        let config = make_test_layers();
        let path = Path::new("src/lib/something.ts");
        assert_eq!(config.layer_for_file(path), None);

        Ok(())
    }

    #[test]
    fn layer_for_file_prefers_most_specific() -> Result<()> {
        let mut config = LayerBoundaryConfig {
            order: vec!["app".to_string(), "app-sub".to_string()],
            ..Default::default()
        };
        config.definitions.insert(
            "app".to_string(),
            DomainConfig {
                folders: vec!["app".to_string()],
                file_suffixes: vec![],
            },
        );
        config.definitions.insert(
            "app-sub".to_string(),
            DomainConfig {
                folders: vec!["app/admin".to_string()],
                file_suffixes: vec![],
            },
        );

        let path = Path::new("src/app/admin/page.ts");
        assert_eq!(config.layer_for_file(path), Some("app-sub"));

        Ok(())
    }

    #[test]
    fn layer_for_folder_does_not_match_substring() -> Result<()> {
        let mut config = LayerBoundaryConfig {
            order: vec!["app".to_string()],
            ..Default::default()
        };
        config.definitions.insert(
            "app".to_string(),
            DomainConfig {
                folders: vec!["app".to_string()],
                file_suffixes: vec![],
            },
        );

        let path = Path::new("src/happy-utils/foo.ts");
        assert_eq!(config.layer_for_file(path), None);

        Ok(())
    }

    #[test]
    fn layer_for_file_suffix_match() -> Result<()> {
        let mut config = LayerBoundaryConfig {
            order: vec!["shared".to_string()],
            ..Default::default()
        };
        config.definitions.insert(
            "shared".to_string(),
            DomainConfig {
                folders: vec![],
                file_suffixes: vec![".shared.ts".to_string()],
            },
        );

        let path = Path::new("src/utils/helper.shared.ts");
        assert_eq!(config.layer_for_file(path), Some("shared"));

        Ok(())
    }

    fn make_test_layers() -> LayerBoundaryConfig {
        let mut config = LayerBoundaryConfig {
            order: vec![
                "app".to_string(),
                "features".to_string(),
                "entities".to_string(),
                "shared".to_string(),
            ],
            ..Default::default()
        };
        config.definitions.insert(
            "app".to_string(),
            DomainConfig {
                folders: vec!["app".to_string()],
                file_suffixes: vec![],
            },
        );
        config.definitions.insert(
            "features".to_string(),
            DomainConfig {
                folders: vec!["features".to_string()],
                file_suffixes: vec![],
            },
        );
        config.definitions.insert(
            "entities".to_string(),
            DomainConfig {
                folders: vec!["entities".to_string()],
                file_suffixes: vec![],
            },
        );
        config.definitions.insert(
            "shared".to_string(),
            DomainConfig {
                folders: vec!["shared".to_string()],
                file_suffixes: vec![],
            },
        );
        config
    }
}
