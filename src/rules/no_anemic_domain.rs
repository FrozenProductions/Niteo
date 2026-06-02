use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{NoAnemicDomainRuleConfig, Severity};
use crate::rules::{NO_ANEMIC_DOMAIN_RULE_ID, Violation};
const MESSAGE: &str =
    "Domain has too few files. Consider flattening into a parent or sibling directory.";

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
    config: &NoAnemicDomainRuleConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|dir| dir.to_string()));

    walk_directories(
        root,
        &ignored,
        exclude_dirs,
        config.max_files,
        &mut violations,
    );

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    exclude_dirs: &[PathBuf],
    max_files: usize,
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
            if exclude_dirs.contains(&path) {
                continue;
            }
            subdirs.push(path);
        } else if is_source_file(&path) {
            file_count += 1;
        }
    }

    if file_count > 0 && file_count <= max_files {
        violations.push(Violation {
            file: current.to_path_buf(),
            line: None,
            column: None,
            rule: NO_ANEMIC_DOMAIN_RULE_ID,
            message: MESSAGE,
            severity: Severity::Warn,
            detail: Some(format!(
                "Contains {} TypeScript file{} (threshold: {}).",
                file_count,
                if file_count == 1 { "" } else { "s" },
                max_files
            )),
            subject: None,
        });
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, exclude_dirs, max_files, violations);
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}
