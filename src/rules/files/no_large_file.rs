use std::path::Path;

use crate::config::FileLengthRuleConfig;
use crate::rules::{NO_LARGE_FILE_RULE_ID, Violation};
const MESSAGE: &str = "Split this file into focused modules.";

pub fn check_file(file: &Path, source: &str, config: &FileLengthRuleConfig) -> Vec<Violation> {
    let line_count = source.lines().count();
    if line_count <= config.max_lines {
        return Vec::new();
    }

    let location = last_location(source);

    vec![Violation {
        file: file.to_path_buf(),
        span: None,
        line: Some(location.line),
        column: Some(location.column),
        rule: NO_LARGE_FILE_RULE_ID,
        message: MESSAGE,
        severity: config.severity,
        detail: None,
        subject: None,
    }]
}

#[derive(Debug, Clone, Copy)]
struct Location {
    line: usize,
    column: usize,
}

fn last_location(source: &str) -> Location {
    let mut line = 1;
    let mut column = 1;

    for byte in source.as_bytes() {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    Location { line, column }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::check_file;
    use crate::config::{FileLengthRuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn allows_files_within_limit() -> Result<()> {
        let source = "line 1\nline 2\nline 3\n";
        let violations = check_file(Path::new("file.ts"), source, &test_config(3));

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_files_over_limit() -> Result<()> {
        let source = "line 1\nline 2\nline 3\nline 4\n";
        let violations = check_file(Path::new("file.ts"), source, &test_config(3));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(5));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_last_column_when_file_has_no_trailing_newline() -> Result<()> {
        let source = "line 1\nline 2\nline 3";
        let violations = check_file(Path::new("file.ts"), source, &test_config(2));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(7));
    
        Ok(())}

    #[test]
    fn counts_empty_file_as_zero_lines() -> Result<()> {
        let violations = check_file(Path::new("file.ts"), "", &test_config(1));

        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config(max_lines: usize) -> FileLengthRuleConfig {
        FileLengthRuleConfig {
            severity: Severity::Warn,
            max_lines,
        }
    }
}
