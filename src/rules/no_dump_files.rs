use std::path::Path;

use crate::config::{NoDumpFilesRuleConfig, Severity};
use crate::rules::{NO_DUMP_FILES_RULE_ID, Violation};
const MESSAGE: &str = "Generic file names like utils.ts, helpers.ts, and types.ts hide intent and become dumping grounds.";

const DEFAULT_FORBIDDEN_NAMES: &[&str] = &["utils", "helpers", "types"];

pub fn check_files(files: &[std::path::PathBuf], config: &NoDumpFilesRuleConfig) -> Vec<Violation> {
    let mut forbidden: Vec<String> = DEFAULT_FORBIDDEN_NAMES
        .iter()
        .map(|default_name| default_name.to_string())
        .collect();
    forbidden.extend(config.extra_names.clone());

    let mut violations = Vec::new();

    for file in files {
        let stem = match file.file_stem().and_then(|os_str| os_str.to_str()) {
            Some(stem) => stem,
            None => continue,
        };

        if forbidden.iter().any(|name| stem.eq_ignore_ascii_case(name)) {
            violations.push(dump_violation(file, stem, config.severity));
        }
    }

    violations
}

fn dump_violation(file: &Path, stem: &str, severity: Severity) -> Violation {
    let file_name = file
        .file_name()
        .map(|os_name| os_name.to_string_lossy().to_string())
        .unwrap_or_default();

    Violation {
        file: file.to_path_buf(),
        line: None,
        column: None,
        rule: NO_DUMP_FILES_RULE_ID,
        message: MESSAGE,
        severity,
        detail: Some(format!(
            "File stem '{}' matches a forbidden generic name.",
            stem
        )),
        subject: Some(file_name),
    }
}

#[cfg(test)]
mod tests {
    use super::check_files;
    use crate::config::{NoDumpFilesRuleConfig, Severity};
    use std::path::PathBuf;

    fn test_config() -> NoDumpFilesRuleConfig {
        NoDumpFilesRuleConfig {
            severity: Severity::Warn,
            extra_names: vec![],
        }
    }

    #[test]
    fn reports_default_forbidden_files() {
        for path in ["src/utils.ts", "src/helpers.tsx", "src/types.ts"] {
            let files = vec![PathBuf::from(path)];
            let violations = check_files(&files, &test_config());
            assert_eq!(violations.len(), 1, "expected 1 violation for: {path}");
            assert_eq!(violations[0].rule, crate::rules::NO_DUMP_FILES_RULE_ID);
        }
    }

    #[test]
    fn ignores_specific_files() {
        let files = vec![
            PathBuf::from("src/Button.tsx"),
            PathBuf::from("src/useAuth.ts"),
        ];
        let violations = check_files(&files, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_custom_extra_names() {
        let config = NoDumpFilesRuleConfig {
            severity: Severity::Warn,
            extra_names: vec!["constants".to_string()],
        };
        let files = vec![PathBuf::from("src/constants.ts")];
        let violations = check_files(&files, &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn case_insensitive_match() {
        let files = vec![PathBuf::from("src/Utils.ts")];
        let violations = check_files(&files, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn does_not_match_partial_names() {
        let files = vec![PathBuf::from("src/authUtils.ts")];
        let violations = check_files(&files, &test_config());
        assert!(violations.is_empty());
    }
}
