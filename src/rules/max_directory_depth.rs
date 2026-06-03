use std::path::Path;

use crate::config::{MaxDirectoryDepthRuleConfig, Severity};
use crate::directory_inventory::{DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{MAX_DIRECTORY_DEPTH_RULE_ID, Violation};
const MESSAGE: &str = "Directory nesting depth exceeds the configured limit.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &MaxDirectoryDepthRuleConfig,
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

        for subdir in &facts.subdirectories {
            let child_depth = facts.depth + 1;
            if child_depth > config.max_depth {
                violations.push(depth_violation(
                    subdir,
                    Severity::Warn,
                    child_depth,
                    config.max_depth,
                ));
            }
        }

        for source_file in &facts.source_files {
            let file_depth = facts.depth + 1;
            if file_depth > config.max_depth {
                violations.push(depth_violation(
                    source_file,
                    Severity::Warn,
                    file_depth,
                    config.max_depth,
                ));
            }
        }
    }

    violations
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
