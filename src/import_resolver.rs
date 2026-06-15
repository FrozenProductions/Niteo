use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::import_graph::helpers::{
    extensionless, is_barrel_file, is_relative_specifier, normalize_path,
};
use crate::tsconfig::{ResolvedPathAlias, TsConfig, match_alias};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecifierKind {
    Relative,
    Alias,
    External,
}

pub(crate) struct ImportResolverIndex {
    entries: HashMap<PathBuf, PathBuf>,
    aliases: Vec<ResolvedPathAlias>,
    base_url: PathBuf,
}

impl ImportResolverIndex {
    pub(crate) fn new(files: &[PathBuf], tsconfig: Option<&TsConfig>) -> Self {
        let mut entries = HashMap::new();
        for file in files {
            let normalized = normalize_path(file);
            entries
                .entry(normalized.clone())
                .or_insert_with(|| file.clone());

            let without_ext = extensionless(&normalized);
            if without_ext != normalized {
                entries.entry(without_ext).or_insert_with(|| file.clone());
            }

            if is_barrel_file(file)
                && let Some(parent) = normalized.parent()
            {
                entries
                    .entry(parent.to_path_buf())
                    .or_insert_with(|| file.clone());
            }
        }

        let (aliases, base_url) = match tsconfig {
            Some(config) => (config.aliases.clone(), config.base_url.clone()),
            None => (Vec::new(), PathBuf::from(".")),
        };

        Self {
            entries,
            aliases,
            base_url,
        }
    }

    pub(crate) fn resolve(&self, source_file: &Path, specifier: &str) -> Option<PathBuf> {
        if is_relative_specifier(specifier) {
            let parent = source_file.parent()?;
            let target = normalize_path(&parent.join(specifier));
            return self.entries.get(&target).cloned();
        }

        self.resolve_alias(specifier)
    }

    pub(crate) fn classify_specifier(&self, specifier: &str) -> SpecifierKind {
        if is_relative_specifier(specifier) {
            return SpecifierKind::Relative;
        }
        if self
            .aliases
            .iter()
            .any(|a| match_alias(a, specifier).is_some())
        {
            return SpecifierKind::Alias;
        }
        SpecifierKind::External
    }

    fn resolve_alias(&self, specifier: &str) -> Option<PathBuf> {
        for alias in &self.aliases {
            let Some(captured) = match_alias(alias, specifier) else {
                continue;
            };
            for target in &alias.targets {
                let candidate = format!("{}{}{}", target.prefix, captured, target.suffix);
                let resolved = normalize_path(&self.base_url.join(&candidate));
                if let Some(found) = self.entries.get(&resolved) {
                    return Some(found.clone());
                }
                let without_ext = extensionless(&resolved);
                if without_ext != resolved
                    && let Some(found) = self.entries.get(&without_ext)
                {
                    return Some(found.clone());
                }
                if let Some(parent) = resolved.parent()
                    && let Some(found) = self.entries.get(parent)
                {
                    return Some(found.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use std::path::{Path, PathBuf};

    use crate::import_graph::helpers::{is_relative_specifier, normalize_path};
    use crate::tsconfig::{PathTargetPattern, ResolvedPathAlias, TsConfig};

    use super::*;

    fn resolve_import_specifier(
        source_file: &Path,
        specifier: &str,
        all_files: &[PathBuf],
    ) -> Option<PathBuf> {
        ImportResolverIndex::new(all_files, None).resolve(source_file, specifier)
    }

    #[test]
    fn identifies_relative_specifiers() -> Result<()> {
        assert!(is_relative_specifier("./foo"));
        assert!(is_relative_specifier("../bar"));
        assert!(is_relative_specifier("/absolute"));
        assert!(!is_relative_specifier("lodash"));
        assert!(!is_relative_specifier("@scope/package"));

        Ok(())
    }

    #[test]
    fn normalizes_paths_correctly() -> Result<()> {
        let path = Path::new("src/components/../utils/./helper");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("src/utils/helper"));

        Ok(())
    }

    #[test]
    fn resolves_relative_import() -> Result<()> {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));

        Ok(())
    }

    #[test]
    fn resolves_import_with_extension() -> Result<()> {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b.ts", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));

        Ok(())
    }

    #[test]
    fn resolves_directory_import_to_barrel() -> Result<()> {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components/index.ts"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./components", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/components/index.ts")));

        Ok(())
    }

    #[test]
    fn returns_none_for_external_import() -> Result<()> {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "lodash", &files);
        assert_eq!(resolved, None);

        Ok(())
    }

    #[test]
    fn returns_none_for_unresolved_import() -> Result<()> {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./nonexistent", &files);
        assert_eq!(resolved, None);

        Ok(())
    }

    #[test]
    fn resolves_extensionless_duplicate_deterministically() -> Result<()> {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/b.ts"),
            PathBuf::from("src/b.tsx"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));

        let files_reversed = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/b.tsx"),
            PathBuf::from("src/b.ts"),
        ];
        let resolved_reversed =
            resolve_import_specifier(Path::new("src/a.ts"), "./b", &files_reversed);
        assert_eq!(resolved_reversed, Some(PathBuf::from("src/b.tsx")));

        Ok(())
    }

    #[test]
    fn resolves_directory_barrel_duplicate_deterministically() -> Result<()> {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components.ts"),
            PathBuf::from("src/components/index.ts"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./components", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/components.ts")));

        let files_reversed = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components/index.ts"),
            PathBuf::from("src/components.ts"),
        ];
        let resolved_reversed =
            resolve_import_specifier(Path::new("src/a.ts"), "./components", &files_reversed);
        assert_eq!(
            resolved_reversed,
            Some(PathBuf::from("src/components/index.ts"))
        );

        Ok(())
    }

    #[test]
    fn classifies_relative_specifier() -> Result<()> {
        let resolver = ImportResolverIndex::new(&[], None);
        assert_eq!(
            resolver.classify_specifier("./foo"),
            SpecifierKind::Relative
        );
        assert_eq!(
            resolver.classify_specifier("../bar"),
            SpecifierKind::Relative
        );
        assert_eq!(resolver.classify_specifier("/abs"), SpecifierKind::Relative);

        Ok(())
    }

    #[test]
    fn classifies_alias_specifier() -> Result<()> {
        let tsconfig = TsConfig {
            base_url: PathBuf::from("/repo"),
            aliases: vec![ResolvedPathAlias {
                pattern: "@/*".into(),
                prefix: "@/".into(),
                suffix: "".into(),
                targets: vec![PathTargetPattern {
                    prefix: "src/".into(),
                    suffix: "".into(),
                }],
            }],
        };
        let resolver = ImportResolverIndex::new(&[], Some(&tsconfig));
        assert_eq!(
            resolver.classify_specifier("@/shared/date"),
            SpecifierKind::Alias
        );

        Ok(())
    }

    #[test]
    fn classifies_external_specifier() -> Result<()> {
        let resolver = ImportResolverIndex::new(&[], None);
        assert_eq!(
            resolver.classify_specifier("lodash"),
            SpecifierKind::External
        );
        assert_eq!(
            resolver.classify_specifier("@scope/package"),
            SpecifierKind::External
        );

        Ok(())
    }

    #[test]
    fn classifies_alias_when_tsconfig_has_multiple_aliases() -> Result<()> {
        let tsconfig = TsConfig {
            base_url: PathBuf::from("/repo"),
            aliases: vec![
                ResolvedPathAlias {
                    pattern: "@components/*".into(),
                    prefix: "@components/".into(),
                    suffix: "".into(),
                    targets: vec![],
                },
                ResolvedPathAlias {
                    pattern: "@/*".into(),
                    prefix: "@/".into(),
                    suffix: "".into(),
                    targets: vec![],
                },
            ],
        };
        let resolver = ImportResolverIndex::new(&[], Some(&tsconfig));
        assert_eq!(resolver.classify_specifier("@/app"), SpecifierKind::Alias);
        assert_eq!(
            resolver.classify_specifier("@components/button"),
            SpecifierKind::Alias
        );
        assert_eq!(
            resolver.classify_specifier("lodash"),
            SpecifierKind::External
        );
        assert_eq!(
            resolver.classify_specifier("./relative"),
            SpecifierKind::Relative
        );

        Ok(())
    }
}
