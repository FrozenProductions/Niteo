use serde::Deserialize;

use super::super::structure::DomainConfig;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawDomainConfig {
    pub folders: Option<Vec<String>>,
    #[serde(rename = "file-suffixes")]
    pub file_suffixes: Option<Vec<String>>,
}

impl RawDomainConfig {
    pub(super) fn merge_option(
        parent: Option<&RawDomainConfig>,
        child: Option<&RawDomainConfig>,
    ) -> Option<RawDomainConfig> {
        match (parent, child) {
            (None, None) => None,
            (Some(parent), None) => Some(parent.clone()),
            (None, Some(child)) => Some(child.clone()),
            (Some(parent), Some(child)) => Some(Self::merge(parent, child)),
        }
    }

    pub(super) fn merge(parent: &RawDomainConfig, child: &RawDomainConfig) -> RawDomainConfig {
        RawDomainConfig {
            folders: child.folders.clone().or_else(|| parent.folders.clone()),
            file_suffixes: child
                .file_suffixes
                .clone()
                .or_else(|| parent.file_suffixes.clone()),
        }
    }

    pub(super) fn to_domain_config(&self, defaults: &DomainConfig) -> DomainConfig {
        DomainConfig {
            folders: self
                .folders
                .clone()
                .unwrap_or_else(|| defaults.folders.clone()),
            file_suffixes: self
                .file_suffixes
                .clone()
                .unwrap_or_else(|| defaults.file_suffixes.clone()),
        }
    }
}
