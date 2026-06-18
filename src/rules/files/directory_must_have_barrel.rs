use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::directory_inventory::{DirectoryFacts, DirectoryInventory, DEFAULT_IGNORED_DIRECTORIES};
use crate::rules::{DIRECTORY_MUST_HAVE_BARREL_RULE_ID, Violation};

const MESSAGE: &str = "Non-leaf directories must expose an index.ts barrel.";
const DETAIL: &str = "Directory contains child folders but no index.ts.";

pub fn check_inventory(
    inventory: &DirectoryInventory,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let ignored: Vec<&str> = DEFAULT_IGNORED_DIRECTORIES.to_vec();

    for facts in &inventory.directories {
        if facts.subdirectories.is_empty() {
            continue;
        }

        let dir_name = facts
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if ignored.contains(&dir_name) {
            continue;
        }

        if has_index_ts(facts) {
            continue;
        }

        violations.push(directory_violation(&facts.path, config.severity));
    }

    violations
}

fn has_index_ts(facts: &DirectoryFacts) -> bool {
    facts.source_files.iter().any(|file| {
        file.file_name().and_then(|name| name.to_str()) == Some("index.ts")
    })
}

fn directory_violation(dir: &Path, severity: Severity) -> Violation {
    Violation {
        file: dir.to_path_buf(),
        span: None,
        line: None,
        column: None,
        rule: DIRECTORY_MUST_HAVE_BARREL_RULE_ID,
        message: MESSAGE,
        severity,
        detail: Some(DETAIL.to_string()),
        subject: None,
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::directory_inventory::{DirectoryFacts, DirectoryInventory};
    use std::path::PathBuf;

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn make_facts(
        path: &str,
        source_files: Vec<&str>,
        subdirectories: Vec<&str>,
    ) -> DirectoryFacts {
        DirectoryFacts {
            path: PathBuf::from(path),
            depth: 1,
            source_files: source_files.into_iter().map(PathBuf::from).collect(),
            subdirectories: subdirectories.into_iter().map(PathBuf::from).collect(),
            barrel_files: Vec::new(),
        }
    }

    #[test]
    fn reports_directory_with_child_folder_and_no_index_ts() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "src/features",
                vec!["src/features/Card.tsx"],
                vec!["src/features/billing"],
            )],
        };

        let violations = check_inventory(&inventory, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, DIRECTORY_MUST_HAVE_BARREL_RULE_ID);
        assert_eq!(violations[0].line, None);
        assert_eq!(violations[0].column, None);
        assert!(violations[0].detail.is_some());
    
        Ok(())}

    #[test]
    fn allows_directory_with_child_folder_and_index_ts() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "src/features",
                vec!["src/features/index.ts", "src/features/Card.tsx"],
                vec!["src/features/billing"],
            )],
        };

        let violations = check_inventory(&inventory, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_leaf_directory_without_index_ts() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "src/features/billing",
                vec!["src/features/billing/Card.tsx"],
                vec![],
            )],
        };

        let violations = check_inventory(&inventory, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_index_tsx() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "src/features",
                vec!["src/features/index.tsx", "src/features/Card.tsx"],
                vec!["src/features/billing"],
            )],
        };

        let violations = check_inventory(&inventory, &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn respects_severity_from_config() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "src/features",
                vec!["src/features/Card.tsx"],
                vec!["src/features/billing"],
            )],
        };

        let config = RuleConfig {
            severity: Severity::Error,
        };
        let violations = check_inventory(&inventory, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, Severity::Error);
    
        Ok(())}

    #[test]
    fn skips_ignored_directories() -> Result<()> {
        let inventory = DirectoryInventory {
            directories: vec![make_facts(
                "node_modules",
                vec![],
                vec!["node_modules/pkg"],
            )],
        };

        let violations = check_inventory(&inventory, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}
}
