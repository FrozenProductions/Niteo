use std::fs;
use std::path::Path;

use crate::config::{MinFilesPerDirectoryRuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "min-files-per-directory";
const MESSAGE: &str =
    "Directory has too few source files. Consider merging with a sibling or removing.";

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

pub fn check_directories(root: &Path, config: &MinFilesPerDirectoryRuleConfig) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|s| s.to_string()));

    walk_directories(root, &ignored, config.min_files, &mut violations);

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    min_files: usize,
    violations: &mut Vec<Violation>,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();
    let mut file_count = 0;

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
            subdirs.push(path);
        } else if is_source_file(&path) {
            file_count += 1;
        }
    }

    if file_count > 0 && file_count < min_files {
        violations.push(directory_violation(
            current,
            Severity::Warn,
            file_count,
            min_files,
        ));
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, min_files, violations);
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
    file_count: usize,
    min_files: usize,
) -> Violation {
    Violation {
        file: dir.to_path_buf(),
        line: None,
        column: None,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: Some(format!(
            "Contains {} TypeScript file(s) (minimum: {}).",
            file_count, min_files
        )),
        subject: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violation_detail_shows_count_and_minimum() {
        let v = directory_violation(Path::new("src"), Severity::Warn, 1, 3);
        assert_eq!(
            v.detail,
            Some("Contains 1 TypeScript file(s) (minimum: 3).".to_string())
        );
    }
}
