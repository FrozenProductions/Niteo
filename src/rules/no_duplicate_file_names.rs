use std::collections::HashMap;
use std::path::Path;

use crate::config::{NoDuplicateFileNamesRuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-duplicate-file-names";
const MESSAGE: &str = "Duplicate file names across directories are confusing in stack traces.";

const DEFAULT_IGNORED_NAMES: &[&str] = &["index.ts", "index.tsx"];

pub fn check_files(
    files: &[std::path::PathBuf],
    config: &NoDuplicateFileNamesRuleConfig,
) -> Vec<Violation> {
    let mut ignored = DEFAULT_IGNORED_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    ignored.extend(config.ignore_names.clone());

    let mut name_map: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

    for file in files {
        if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
            if ignored.iter().any(|ign| name == *ign) {
                continue;
            }
            name_map
                .entry(name.to_string())
                .or_default()
                .push(file.clone());
        }
    }

    let mut violations = Vec::new();

    for (name, paths) in &name_map {
        if paths.len() < 2 {
            continue;
        }

        let duplicates = find_duplicates_in_different_dirs(paths);
        if duplicates.is_empty() {
            continue;
        }

        for (file_a, file_b) in &duplicates {
            violations.push(duplicate_violation(file_a, file_b, name, config.severity));
        }
    }

    violations
}

fn find_duplicates_in_different_dirs(
    paths: &[std::path::PathBuf],
) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut pairs = Vec::new();

    for i in 0..paths.len() {
        for j in (i + 1)..paths.len() {
            let dir_a = paths[i].parent();
            let dir_b = paths[j].parent();

            if let (Some(dir_a), Some(dir_b)) = (dir_a, dir_b) {
                if dir_a != dir_b {
                    pairs.push((paths[i].clone(), paths[j].clone()));
                }
            }
        }
    }

    pairs
}

fn duplicate_violation(file: &Path, other: &Path, name: &str, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: None,
        column: None,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: Some(format!("Also exists at: {}", other.display())),
        subject: Some(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::check_files;
    use crate::config::{NoDuplicateFileNamesRuleConfig, Severity};
    use std::path::PathBuf;

    #[test]
    fn reports_duplicate_names_in_different_dirs() {
        let files = vec![
            PathBuf::from("src/components/Button.ts"),
            PathBuf::from("src/utils/Button.ts"),
        ];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec![],
        };

        let violations = check_files(&files, &config);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("Button.ts".to_string()));
    }

    #[test]
    fn ignores_same_name_in_same_dir() {
        let files = vec![PathBuf::from("src/components/Button.ts")];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec![],
        };

        let violations = check_files(&files, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_index_files_by_default() {
        let files = vec![
            PathBuf::from("src/components/index.ts"),
            PathBuf::from("src/utils/index.ts"),
        ];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec![],
        };

        let violations = check_files(&files, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn respects_custom_ignore_names() {
        let files = vec![
            PathBuf::from("src/components/types.ts"),
            PathBuf::from("src/utils/types.ts"),
        ];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec!["types.ts".to_string()],
        };

        let violations = check_files(&files, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiple_duplicate_pairs() {
        let files = vec![
            PathBuf::from("src/a/utils.ts"),
            PathBuf::from("src/b/utils.ts"),
            PathBuf::from("src/c/utils.ts"),
        ];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec![],
        };

        let violations = check_files(&files, &config);

        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn ignores_different_extensions() {
        let files = vec![
            PathBuf::from("src/components/Button.ts"),
            PathBuf::from("src/utils/Button.tsx"),
        ];

        let config = NoDuplicateFileNamesRuleConfig {
            severity: Severity::Warn,
            ignore_names: vec![],
        };

        let violations = check_files(&files, &config);

        assert!(violations.is_empty());
    }
}
