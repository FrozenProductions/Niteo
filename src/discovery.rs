use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_files(root: &Path, scope: Option<&Path>) -> Result<Vec<PathBuf>> {
    let base = root;
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    visit(base, scope, &mut files)?;
    Ok(files)
}

fn visit(path: &Path, scope: Option<&Path>, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if should_skip(&entry_path, scope) {
            continue;
        }

        if entry_path.is_dir() {
            visit(&entry_path, scope, files)?;
            continue;
        }

        if matches_typescript_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn should_skip(path: &Path, scope: Option<&Path>) -> bool {
    if let Some(scope) = scope {
        if !path.starts_with(scope) {
            return true;
        }
    }

    false
}

fn matches_typescript_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts") | Some("tsx")
    )
}
