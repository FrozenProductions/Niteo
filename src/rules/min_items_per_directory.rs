use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{MinItemsPerDirectoryRuleConfig, Severity};
use crate::rules::{MIN_ITEMS_PER_DIRECTORY_RULE_ID, Violation};
const MESSAGE: &str =
    "Directory has too few source items. Consider merging with a sibling or removing.";

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

pub fn check_directories(
    root: &Path,
    config: &MinItemsPerDirectoryRuleConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|dir| dir.to_string()));

    walk_directories(
        root,
        &ignored,
        exclude_dirs,
        config.min_items,
        config.count_folders,
        &mut violations,
    );

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    exclude_dirs: &[PathBuf],
    min_items: usize,
    count_folders: bool,
    violations: &mut Vec<Violation>,
) {
    if exclude_dirs.iter().any(|excl| current == excl.as_path()) {
        return;
    }

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
            if exclude_dirs.contains(&path) {
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

    if item_count > 0 && item_count < min_items {
        violations.push(directory_violation(
            current,
            Severity::Warn,
            item_count,
            min_items,
            count_folders,
        ));
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, exclude_dirs, min_items, count_folders, violations);
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
    min_items: usize,
    count_folders: bool,
) -> Violation {
    let kind = if count_folders { "items" } else { "files" };
    Violation {
        file: dir.to_path_buf(),
        line: None,
        column: None,
        rule: MIN_ITEMS_PER_DIRECTORY_RULE_ID,
        message: MESSAGE,
        severity,
        detail: Some(format!(
            "Contains {} TypeScript {} (minimum: {}).",
            item_count, kind, min_items
        )),
        subject: None,
    }
}


