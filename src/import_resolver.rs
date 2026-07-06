use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::import_graph::helpers::{
    extensionless, is_barrel_file, is_relative_specifier, normalize_path,
};
use crate::tsconfig::{ResolvedPathAlias, TsConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecifierKind {
    Relative,
    Alias,
    External,
}

struct WildcardNode {
    aliases: Vec<usize>,
    children: HashMap<char, WildcardNode>,
}

impl WildcardNode {
    fn new() -> Self {
        Self {
            aliases: Vec::new(),
            children: HashMap::new(),
        }
    }
}

pub(crate) struct ImportResolverIndex {
    entries: HashMap<PathBuf, PathBuf>,
    aliases: Vec<ResolvedPathAlias>,
    exact_aliases: HashMap<String, usize>,
    wildcard_root: WildcardNode,
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

        let mut exact_aliases = HashMap::new();
        let mut wildcard_root = WildcardNode::new();
        for (index, alias) in aliases.iter().enumerate() {
            if alias.pattern.contains('*') {
                let mut node = &mut wildcard_root;
                for ch in alias.prefix.chars() {
                    node = node.children.entry(ch).or_insert_with(WildcardNode::new);
                }
                node.aliases.push(index);
            } else {
                exact_aliases.insert(alias.pattern.clone(), index);
            }
        }

        Self {
            entries,
            aliases,
            exact_aliases,
            wildcard_root,
            base_url,
        }
    }

    pub(crate) fn classify_and_resolve(
        &self,
        source_file: &Path,
        specifier: &str,
    ) -> (SpecifierKind, Option<PathBuf>) {
        if is_relative_specifier(specifier) {
            let resolved = self.resolve_relative(source_file, specifier);
            return (SpecifierKind::Relative, resolved);
        }

        if let Some(&index) = self.exact_aliases.get(specifier) {
            let resolved = self.resolve_alias_index(index);
            return (SpecifierKind::Alias, resolved);
        }

        let candidates = self.collect_wildcard_candidates(specifier);
        for &index in &candidates {
            let alias = &self.aliases[index];
            if let Some(captured) = Self::match_wildcard(alias, specifier)
                && let Some(resolved) = self.resolve_alias_targets(alias, captured)
            {
                return (SpecifierKind::Alias, Some(resolved));
            }
        }

        (SpecifierKind::External, None)
    }

    fn resolve_relative(&self, source_file: &Path, specifier: &str) -> Option<PathBuf> {
        let parent = source_file.parent()?;
        let target = normalize_path(&parent.join(specifier));
        self.entries.get(&target).cloned()
    }

    fn collect_wildcard_candidates(&self, specifier: &str) -> Vec<usize> {
        let mut node = &self.wildcard_root;
        let mut candidates = Vec::new();
        candidates.extend(&node.aliases);
        for ch in specifier.chars() {
            match node.children.get(&ch) {
                Some(next) => {
                    node = next;
                    candidates.extend(&node.aliases);
                }
                None => break,
            }
        }
        candidates.sort();
        candidates
    }

    fn match_wildcard<'a>(alias: &ResolvedPathAlias, specifier: &'a str) -> Option<&'a str> {
        if specifier.starts_with(&alias.prefix) && specifier.ends_with(&alias.suffix) {
            let wildcard_start = alias.prefix.len();
            let wildcard_end = specifier.len().saturating_sub(alias.suffix.len());
            if wildcard_end >= wildcard_start {
                return Some(specifier.get(wildcard_start..wildcard_end).unwrap_or(""));
            }
        }
        None
    }

    fn resolve_alias_index(&self, index: usize) -> Option<PathBuf> {
        let alias = &self.aliases[index];
        self.resolve_alias_targets(alias, "")
    }

    fn resolve_alias_targets(&self, alias: &ResolvedPathAlias, captured: &str) -> Option<PathBuf> {
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
        ImportResolverIndex::new(all_files, None)
            .classify_and_resolve(source_file, specifier)
            .1
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
        let dummy = &Path::new("src/a.ts");
        assert_eq!(
            resolver.classify_and_resolve(dummy, "./foo").0,
            SpecifierKind::Relative
        );
        assert_eq!(
            resolver.classify_and_resolve(dummy, "../bar").0,
            SpecifierKind::Relative
        );
        assert_eq!(
            resolver.classify_and_resolve(dummy, "/abs").0,
            SpecifierKind::Relative
        );

        Ok(())
    }

    #[test]
    fn classifies_alias_specifier() -> Result<()> {
        let tsconfig = TsConfig::new(
            PathBuf::from("/repo"),
            vec![ResolvedPathAlias {
                pattern: "@/*".into(),
                prefix: "@/".into(),
                suffix: "".into(),
                targets: vec![PathTargetPattern {
                    prefix: "src/".into(),
                    suffix: "".into(),
                }],
            }],
        );
        let files = vec![PathBuf::from("/repo/src/shared/date.ts")];
        let resolver = ImportResolverIndex::new(&files, Some(&tsconfig));
        assert_eq!(
            resolver
                .classify_and_resolve(Path::new("/repo/src/a.ts"), "@/shared/date")
                .0,
            SpecifierKind::Alias
        );

        Ok(())
    }

    #[test]
    fn classifies_external_specifier() -> Result<()> {
        let resolver = ImportResolverIndex::new(&[], None);
        let dummy = &Path::new("src/a.ts");
        assert_eq!(
            resolver.classify_and_resolve(dummy, "lodash").0,
            SpecifierKind::External
        );
        assert_eq!(
            resolver.classify_and_resolve(dummy, "@scope/package").0,
            SpecifierKind::External
        );

        Ok(())
    }

    #[test]
    fn classifies_alias_when_tsconfig_has_multiple_aliases() -> Result<()> {
        let tsconfig = TsConfig::new(
            PathBuf::from("/repo"),
            vec![
                ResolvedPathAlias {
                    pattern: "@components/*".into(),
                    prefix: "@components/".into(),
                    suffix: "".into(),
                    targets: vec![PathTargetPattern {
                        prefix: "src/components/".into(),
                        suffix: "".into(),
                    }],
                },
                ResolvedPathAlias {
                    pattern: "@/*".into(),
                    prefix: "@/".into(),
                    suffix: "".into(),
                    targets: vec![PathTargetPattern {
                        prefix: "src/".into(),
                        suffix: "".into(),
                    }],
                },
            ],
        );
        let files = vec![
            PathBuf::from("/repo/src/app.ts"),
            PathBuf::from("/repo/src/components/button.ts"),
        ];
        let resolver = ImportResolverIndex::new(&files, Some(&tsconfig));
        assert_eq!(
            resolver
                .classify_and_resolve(Path::new("/repo/src/a.ts"), "@/app")
                .0,
            SpecifierKind::Alias
        );
        assert_eq!(
            resolver
                .classify_and_resolve(Path::new("/repo/src/a.ts"), "@components/button")
                .0,
            SpecifierKind::Alias
        );
        assert_eq!(
            resolver
                .classify_and_resolve(Path::new("/repo/src/a.ts"), "lodash")
                .0,
            SpecifierKind::External
        );
        assert_eq!(
            resolver
                .classify_and_resolve(Path::new("/repo/src/a.ts"), "./relative")
                .0,
            SpecifierKind::Relative
        );

        Ok(())
    }
}
