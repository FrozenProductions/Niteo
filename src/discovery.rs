use anyhow::{bail, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use crate::config::GitignoreConfig;

pub fn discover_files(
    root: &Path,
    scope: Option<&Path>,
    gitignore_config: &GitignoreConfig,
) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        bail!("root path does not exist: {}", root.display());
    }

    let mut builder = WalkBuilder::new(root);
    builder.git_ignore(gitignore_config.enabled);
    builder.hidden(false);
    builder.follow_links(false);

    if let Some(scope) = scope {
        let scope = scope.to_path_buf();
        builder.filter_entry(move |entry| entry.path().starts_with(&scope));
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry?;
        if entry.path().is_file() && matches_typescript_file(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

fn matches_typescript_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts") | Some("tsx")
    )
}
