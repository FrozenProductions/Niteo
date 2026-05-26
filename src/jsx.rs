use std::path::Path;

use crate::config::structure::DomainConfig;

pub fn is_hook_file(path: &Path, hooks: &DomainConfig) -> bool {
    hooks.matches_file(path)
}

pub fn is_component_file(path: &Path, components: &DomainConfig) -> bool {
    components.matches_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::structure::ProjectStructureConfig;
    use std::path::Path;

    fn default_hooks() -> DomainConfig {
        ProjectStructureConfig::default().hooks
    }

    fn default_components() -> DomainConfig {
        ProjectStructureConfig::default().components
    }

    #[test]
    fn hook_file_by_suffix() {
        assert!(is_hook_file(Path::new("useAuth.hook.ts"), &default_hooks()));
        assert!(is_hook_file(
            Path::new("useAuth.hooks.ts"),
            &default_hooks()
        ));
    }

    #[test]
    fn hook_file_in_hooks_folder() {
        assert!(is_hook_file(
            Path::new("src/hooks/useAuth.ts"),
            &default_hooks()
        ));
        assert!(is_hook_file(
            Path::new("hooks/useAuth.tsx"),
            &default_hooks()
        ));
    }

    #[test]
    fn non_hook_file() {
        assert!(!is_hook_file(
            Path::new("src/components/Button.tsx"),
            &default_hooks()
        ));
        assert!(!is_hook_file(
            Path::new("src/utils/format.ts"),
            &default_hooks()
        ));
    }

    #[test]
    fn component_file_by_suffix() {
        assert!(is_component_file(
            Path::new("Button.component.tsx"),
            &default_components()
        ));
        assert!(is_component_file(
            Path::new("Card.components.tsx"),
            &default_components()
        ));
    }

    #[test]
    fn component_file_in_components_folder() {
        assert!(is_component_file(
            Path::new("src/components/Button.tsx"),
            &default_components()
        ));
        assert!(is_component_file(
            Path::new("components/Modal.tsx"),
            &default_components()
        ));
    }

    #[test]
    fn non_component_file() {
        assert!(!is_component_file(
            Path::new("src/hooks/useAuth.ts"),
            &default_components()
        ));
        assert!(!is_component_file(
            Path::new("src/utils/format.ts"),
            &default_components()
        ));
    }

    #[test]
    fn custom_hook_folder() {
        let hooks = DomainConfig {
            folders: vec!["custom-hooks".to_string()],
            file_suffixes: vec![".hook.ts".to_string()],
        };
        assert!(is_hook_file(Path::new("custom-hooks/useAuth.ts"), &hooks));
        assert!(is_hook_file(Path::new("useAuth.hook.ts"), &hooks));
        assert!(!is_hook_file(Path::new("hooks/useAuth.ts"), &hooks));
    }

    #[test]
    fn custom_component_suffix() {
        let components = DomainConfig {
            folders: vec!["ui".to_string()],
            file_suffixes: vec![".ui.tsx".to_string()],
        };
        assert!(is_component_file(Path::new("ui/Button.tsx"), &components));
        assert!(is_component_file(Path::new("Button.ui.tsx"), &components));
        assert!(!is_component_file(
            Path::new("components/Button.tsx"),
            &components
        ));
    }
}
