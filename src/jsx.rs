use std::path::Path;

use crate::config::structure::DomainConfig;

pub fn is_hook_file(path: &Path, hooks: &DomainConfig) -> bool {
    hooks.matches_file(path)
}

pub fn is_component_file(path: &Path, components: &DomainConfig) -> bool {
    components.matches_file(path)
}
