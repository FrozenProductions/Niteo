use std::fs;
use std::path::Path;

use crate::config::{MaxDirectoryDepthRuleConfig, Severity};
use crate::rules::{MAX_DIRECTORY_DEPTH_RULE_ID, Violation};
const MESSAGE: &str = "Directory nesting depth exceeds the configured limit.";

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

pub fn check_directories(root: &Path, config: &MaxDirectoryDepthRuleConfig) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|s| s.to_string()));

    walk_directories(root, &ignored, config.max_depth, 0, &mut violations);

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    max_depth: usize,
    depth: usize,
    violations: &mut Vec<Violation>,
) {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();

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

            let child_depth = depth + 1;

            if child_depth > max_depth {
                violations.push(depth_violation(
                    &path,
                    Severity::Warn,
                    child_depth,
                    max_depth,
                ));
            }

            subdirs.push(path);
        } else if is_source_file(&path) {
            let file_depth = depth + 1;

            if file_depth > max_depth {
                violations.push(depth_violation(
                    &path,
                    Severity::Warn,
                    file_depth,
                    max_depth,
                ));
            }
        }
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, max_depth, depth + 1, violations);
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}

fn depth_violation(path: &Path, severity: Severity, depth: usize, max_depth: usize) -> Violation {
    Violation {
        file: path.to_path_buf(),
        line: None,
        column: None,
        rule: MAX_DIRECTORY_DEPTH_RULE_ID,
        message: MESSAGE,
        severity,
        detail: Some(format!("Depth {} exceeds maximum of {}.", depth, max_depth)),
        subject: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violation_detail_shows_depth_and_limit() {
        let v = depth_violation(Path::new("src/a/b/c/d/e/f.ts"), Severity::Warn, 6, 5);
        assert_eq!(v.detail, Some("Depth 6 exceeds maximum of 5.".to_string()));
    }
}
