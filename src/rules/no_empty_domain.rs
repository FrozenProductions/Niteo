use crate::config::{NoEmptyDomainRuleConfig, Severity};
use crate::directory_inventory::{DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{NO_EMPTY_DOMAIN_RULE_ID, Violation};
const MESSAGE: &str =
    "Domain folder contains only barrel files with no real source. Add implementation or remove.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &NoEmptyDomainRuleConfig,
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

        let has_any_source = !facts.source_files.is_empty();
        let barrel_count = facts.barrel_files.len();
        let non_barrel_count = facts.source_files.len() - barrel_count;
        let has_non_barrel_source = non_barrel_count > 0;

        if has_any_source && !has_non_barrel_source {
            violations.push(Violation {
                file: facts.path.clone(),
                line: None,
                column: None,
                rule: NO_EMPTY_DOMAIN_RULE_ID,
                message: MESSAGE,
                severity: Severity::Warn,
                detail: None,
                subject: None,
            });
        }
    }

    violations
}
