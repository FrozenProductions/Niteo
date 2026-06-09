use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::import_graph::helpers::{
    extensionless, is_barrel_file, is_relative_specifier, normalize_path,
};
use crate::tsconfig::{ResolvedPathAlias, TsConfig, match_alias};

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
