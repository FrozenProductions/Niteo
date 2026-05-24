mod max_file_exports;
mod no_comments;
mod no_default_export;
mod no_inline_types;
mod no_large_file;
mod no_logic_in_barrel;
mod no_upward_import;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::{
    CommentsRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig, RuleConfig, Severity,
    UpwardImportRuleConfig,
};

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
    no_default_export: RuleConfig,
    no_inline_types: RuleConfig,
    max_file_exports: FileExportsRuleConfig,
    no_upward_import: UpwardImportRuleConfig,
    no_large_file: FileLengthRuleConfig,
) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    if !no_comments.severity.is_enabled()
        && !no_logic_in_barrel.severity.is_enabled()
        && !no_default_export.severity.is_enabled()
        && !no_inline_types.severity.is_enabled()
        && !max_file_exports.severity.is_enabled()
        && !no_upward_import.severity.is_enabled()
        && !no_large_file.severity.is_enabled()
    {
        return Ok(violations);
    }

    let type_location_style = no_inline_types::TypeLocationStyle::detect(files);

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
        if no_default_export.severity.is_enabled() {
            violations.extend(no_default_export::check_file(
                file,
                &source,
                &no_default_export,
            ));
        }
        if no_inline_types.severity.is_enabled() {
            violations.extend(no_inline_types::check_file(
                file,
                &source,
                &no_inline_types,
                type_location_style,
            ));
        }
        if max_file_exports.severity.is_enabled() {
            violations.extend(max_file_exports::check_file(
                file,
                &source,
                &max_file_exports,
            ));
        }
        if no_upward_import.severity.is_enabled() {
            violations.extend(no_upward_import::check_file(
                file,
                &source,
                &no_upward_import,
            ));
        }
        if no_large_file.severity.is_enabled() {
            violations.extend(no_large_file::check_file(file, &source, &no_large_file));
        }
    }

    Ok(violations)
}
