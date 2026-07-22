use serde::Serialize;
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize)]
pub struct DomainConfig {
    pub folders: Vec<String>,
    pub file_suffixes: Vec<String>,
}

impl DomainConfig {
    pub fn matches_file(&self, file: &Path) -> bool {
        let in_folder = file.components().any(|component| {
            matches!(
                component,
                Component::Normal(name) if self.folders.iter().any(|folder| name.to_str() == Some(folder))
            )
        });

        let has_suffix = file
            .file_name()
            .and_then(|os_name| os_name.to_str())
            .is_some_and(|name| {
                self.file_suffixes
                    .iter()
                    .any(|suffix| name.ends_with(suffix.as_str()))
            });

        in_folder || has_suffix
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStructureConfig {
    pub hooks: DomainConfig,
    pub components: DomainConfig,
    pub types: DomainConfig,
    pub constants: DomainConfig,
    pub tests: DomainConfig,
    pub generated: DomainConfig,
}

impl Default for ProjectStructureConfig {
    fn default() -> Self {
        Self {
            hooks: DomainConfig {
                folders: vec!["hooks".to_string()],
                file_suffixes: vec![".hook.ts".to_string(), ".hooks.ts".to_string()],
            },
            components: DomainConfig {
                folders: vec!["components".to_string()],
                file_suffixes: vec![".component.tsx".to_string(), ".components.tsx".to_string()],
            },
            types: DomainConfig {
                folders: vec!["types".to_string()],
                file_suffixes: vec![".type.ts".to_string(), ".types.ts".to_string()],
            },
            constants: DomainConfig {
                folders: vec!["constants".to_string()],
                file_suffixes: vec![".constant.ts".to_string(), ".constants.ts".to_string()],
            },
            tests: DomainConfig {
                folders: vec!["tests".to_string()],
                file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
            },
            generated: DomainConfig {
                folders: vec!["generated".to_string(), "__generated__".to_string()],
                file_suffixes: vec![".generated.ts".to_string(), ".generated.tsx".to_string()],
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;
    use std::path::Path;

    #[test]
    fn default_hooks_domain() -> Result<()> {
        let config = ProjectStructureConfig::default();
        assert!(config.hooks.matches_file(Path::new("src/hooks/useAuth.ts")));
        assert!(config.hooks.matches_file(Path::new("useAuth.hook.ts")));
        assert!(config.hooks.matches_file(Path::new("useAuth.hooks.ts")));
        assert!(!config.hooks.matches_file(Path::new("src/utils/format.ts")));

        Ok(())
    }

    #[test]
    fn default_components_domain() -> Result<()> {
        let config = ProjectStructureConfig::default();
        assert!(
            config
                .components
                .matches_file(Path::new("src/components/Button.tsx"))
        );
        assert!(
            config
                .components
                .matches_file(Path::new("Button.component.tsx"))
        );
        assert!(
            config
                .components
                .matches_file(Path::new("Button.components.tsx"))
        );
        assert!(
            !config
                .components
                .matches_file(Path::new("src/hooks/useAuth.ts"))
        );

        Ok(())
    }

    #[test]
    fn default_types_domain() -> Result<()> {
        let config = ProjectStructureConfig::default();
        assert!(config.types.matches_file(Path::new("types/Button.ts")));
        assert!(config.types.matches_file(Path::new("Button.type.ts")));
        assert!(config.types.matches_file(Path::new("Button.types.ts")));
        assert!(!config.types.matches_file(Path::new("Button.tsx")));

        Ok(())
    }

    #[test]
    fn default_constants_domain() -> Result<()> {
        let config = ProjectStructureConfig::default();
        assert!(
            config
                .constants
                .matches_file(Path::new("constants/routes.ts"))
        );
        assert!(config.constants.matches_file(Path::new("api.constant.ts")));
        assert!(config.constants.matches_file(Path::new("api.constants.ts")));
        assert!(
            !config
                .constants
                .matches_file(Path::new("src/utils/format.ts"))
        );

        Ok(())
    }

    #[test]
    fn default_tests_domain() -> Result<()> {
        let config = ProjectStructureConfig::default();
        assert!(config.tests.matches_file(Path::new("tests/auth.test.ts")));
        assert!(config.tests.matches_file(Path::new("src/auth.test.ts")));
        assert!(config.tests.matches_file(Path::new("src/auth.tests.ts")));
        assert!(!config.tests.matches_file(Path::new("src/auth.ts")));

        Ok(())
    }

    #[test]
    fn custom_domain_config() -> Result<()> {
        let domain = DomainConfig {
            folders: vec!["custom-hooks".to_string()],
            file_suffixes: vec![".custom.ts".to_string()],
        };
        assert!(domain.matches_file(Path::new("custom-hooks/useAuth.ts")));
        assert!(domain.matches_file(Path::new("useAuth.custom.ts")));
        assert!(!domain.matches_file(Path::new("hooks/useAuth.ts")));

        Ok(())
    }
}
