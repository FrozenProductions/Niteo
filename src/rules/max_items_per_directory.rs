use std::fs;
use std::path::Path;

use crate::config::{MaxItemsPerDirectoryRuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "max-items-per-directory";
const MESSAGE: &str = "Directory exceeds the maximum number of items. Consider sub-grouping.";

const IGNORED_DIRECTORIES: &[&str] = &[
    "node_modules",
    ".git",
    ".vscode",
    ".idea",
    "dist",
    "build",
    "out",
    ".next",
    ".svelte-kit",
    "target",
];

const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx"];

pub fn check_directories(root: &Path, config: &MaxItemsPerDirectoryRuleConfig) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|s| s.to_string()));

    walk_directories(
        root,
        &ignored,
        config.max_items,
        config.count_folders,
        &mut violations,
    );

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    max_items: usize,
    count_folders: bool,
    violations: &mut Vec<Violation>,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();
    let mut item_count = 0;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if ignored.iter().any(|ign| name_str == *ign) {
                continue;
            }
            if count_folders {
                item_count += 1;
            }
            subdirs.push(path);
        } else if is_source_file(&path) {
            item_count += 1;
        }
    }

    if item_count > max_items {
        violations.push(directory_violation(
            current,
            Severity::Warn,
            item_count,
            max_items,
            count_folders,
        ));
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, max_items, count_folders, violations);
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}

fn directory_violation(
    dir: &Path,
    severity: Severity,
    item_count: usize,
    max_items: usize,
    count_folders: bool,
) -> Violation {
    let kind = if count_folders { "items" } else { "files" };
    Violation {
        file: dir.to_path_buf(),
        line: None,
        column: None,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: Some(format!(
            "Contains {} TypeScript {} (limit: {}).",
            item_count, kind, max_items
        )),
        subject: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violation_detail_shows_count_and_limit_files_only() {
        let v = directory_violation(Path::new("src"), Severity::Warn, 15, 10, false);
        assert_eq!(
            v.detail,
            Some("Contains 15 TypeScript files (limit: 10).".to_string())
        );
    }

    #[test]
    fn violation_detail_shows_count_and_limit_including_folders() {
        let v = directory_violation(Path::new("src"), Severity::Warn, 25, 20, true);
        assert_eq!(
            v.detail,
            Some("Contains 25 TypeScript items (limit: 20).".to_string())
        );
    }
}
