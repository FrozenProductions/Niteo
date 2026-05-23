mod no_comments;
mod no_logic_in_barrel;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::{CommentsRuleConfig, RuleConfig, Severity};

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub rule: &'static str,
    pub severity: Severity,
}

pub fn check_files(
    files: &[PathBuf],
    no_comments: CommentsRuleConfig,
    no_logic_in_barrel: RuleConfig,
) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    if !no_comments.severity.is_enabled() && !no_logic_in_barrel.severity.is_enabled() {
        return Ok(violations);
    }

    for file in files {
        let source = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        if no_comments.severity.is_enabled() {
            violations.extend(no_comments::check_file(file, &source, &no_comments));
        }
        if no_logic_in_barrel.severity.is_enabled() {
            violations.extend(no_logic_in_barrel::check_file(
                file,
                &source,
                &no_logic_in_barrel,
            ));
        }
    }

    Ok(violations)
}
