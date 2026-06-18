use std::path::Path;

use crate::config::{NoEmptyDirectoriesRuleConfig, Severity};
use crate::directory_inventory::{DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{NO_EMPTY_DIRECTORIES_RULE_ID, Violation};
const MESSAGE: &str = "Remove directories with no source files or only empty barrel files.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &NoEmptyDirectoriesRuleConfig,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let ignored: Vec<&str> = config
        .ignore_dirs
        .iter()
        .map(|s| s.as_str())
        .chain(DEFAULT_IGNORED_DIRECTORIES.iter().copied())
        .collect();

    for facts in &inventory.directories {
        let dir_name = facts
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if ignored.contains(&dir_name) {
            continue;
        }

        let has_source_file = !facts.source_files.is_empty();
        let has_subdirs = !facts.subdirectories.is_empty();

        let is_empty_dir = !has_source_file && !has_subdirs;

        let is_empty_barrel_dir = if has_source_file && !has_subdirs {
            let barrel_count = facts.barrel_files.len();
            let non_barrel_count = facts.source_files.len() - barrel_count;

            if non_barrel_count == 0 && barrel_count > 0 {
                facts.barrel_files.iter().all(|b| b.is_empty)
            } else {
                false
            }
        } else {
            false
        };

        if is_empty_dir || is_empty_barrel_dir {
            violations.push(directory_violation(&facts.path, Severity::Warn));
        }
    }

    violations
}

fn directory_violation(dir: &Path, severity: Severity) -> Violation {
    Violation {
        file: dir.to_path_buf(),
        line: None,
        column: None,
        rule: NO_EMPTY_DIRECTORIES_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}
