use crate::config::{NoAnemicDomainRuleConfig, Severity};
use crate::directory_inventory::{DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{NO_ANEMIC_DOMAIN_RULE_ID, Violation};
const MESSAGE: &str =
    "Domain has too few files. Consider flattening into a parent or sibling directory.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &NoAnemicDomainRuleConfig,
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

        if file_count > 0 && file_count <= config.max_files {
            violations.push(Violation {
                file: facts.path.clone(),
                line: None,
                column: None,
                rule: NO_ANEMIC_DOMAIN_RULE_ID,
                message: MESSAGE,
                severity: Severity::Warn,
                detail: Some(format!(
                    "Contains {} TypeScript file{} (threshold: {}).",
                    file_count,
                    if file_count == 1 { "" } else { "s" },
                    config.max_files
                )),
                subject: None,
            });
        }
    }

    violations
}
