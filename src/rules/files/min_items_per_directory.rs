use std::path::Path;

use crate::config::{MinItemsPerDirectoryRuleConfig, Severity};
use crate::directory_inventory::{DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{MIN_ITEMS_PER_DIRECTORY_RULE_ID, Violation};
const MESSAGE: &str =
    "Directory has too few source items. Consider merging with a sibling or removing.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &MinItemsPerDirectoryRuleConfig,
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

        let file_count = facts.source_files.len();
        let folder_count = facts.subdirectories.len();
        let item_count = if config.count_folders {
            file_count + folder_count
        } else {
            file_count
        };

        // Empty directories are left to no-empty-directories; this rule only
        // flags directories that exist but fall below the minimum.
        if item_count > 0 && item_count < config.min_items {
            violations.push(directory_violation(
                &facts.path,
                config.severity,
                item_count,
                config.min_items,
                config.count_folders,
            ));
        }
    }

    violations
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
        span: None,
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
