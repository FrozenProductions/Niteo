use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{NoEmptyDomainRuleConfig, Severity};
use crate::rules::{NO_EMPTY_DOMAIN_RULE_ID, Violation};
const MESSAGE: &str =
    "Domain folder contains only barrel files with no real source. Add implementation or remove.";

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
    config: &NoEmptyDomainRuleConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|dir| dir.to_string()));

    walk_directories(root, &ignored, exclude_dirs, &mut violations);

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    exclude_dirs: &[PathBuf],
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
    let mut has_non_barrel_source = false;

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
        } else if path.is_file() && is_source_file(&path) {
            if !is_barrel_file(&path) {
                has_non_barrel_source = true;
            } else if !is_barrel_with_real_reexports(&path) {
                // empty barrel — doesn't count as real source
            } else {
                // barrel with re-exports — still just a barrel
            }
        }
    }

    let has_any_source = has_source_files_in_dir(current);

    if has_any_source && !has_non_barrel_source {
        violations.push(Violation {
            file: current.to_path_buf(),
            line: None,
            column: None,
            rule: NO_EMPTY_DOMAIN_RULE_ID,
            message: MESSAGE,
            severity: Severity::Warn,
            detail: None,
            subject: None,
        });
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, exclude_dirs, violations);
    }
}

fn has_source_files_in_dir(dir: &Path) -> bool {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && is_source_file(&path) {
            return true;
        }
    }

    false
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}

fn is_barrel_file(path: &Path) -> bool {
    path.file_name().and_then(|os_name| os_name.to_str()) == Some("index.ts")
        || path.file_name().and_then(|os_name| os_name.to_str()) == Some("index.tsx")
}

fn is_barrel_with_real_reexports(path: &Path) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    !source.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_barrel_file() {
        assert!(is_barrel_file(Path::new("src/components/index.ts")));
        assert!(is_barrel_file(Path::new("index.tsx")));
        assert!(!is_barrel_file(Path::new("Button.tsx")));
    }

    #[test]
    fn detects_source_file() {
        assert!(is_source_file(Path::new("Button.tsx")));
        assert!(is_source_file(Path::new("utils.ts")));
        assert!(!is_source_file(Path::new("readme.md")));
    }
}
