mod max_directory_depth;
mod max_file_exports;
mod max_items_per_directory;
mod min_items_per_directory;
mod no_barrel_files;
mod no_comments;
mod no_console;
mod no_debugger;
mod no_default_export;
mod no_duplicate_file_names;
mod no_empty_directories;
mod no_empty_interface;
mod no_enums;
mod no_eval;
mod no_export_star;
mod no_inline_types;
mod no_interface;
mod no_large_file;
mod no_logic_in_barrel;
mod no_logic_in_domain;
mod no_mutable_exports;
mod no_upward_import;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::{
    CommentsRuleConfig, FileExportsRuleConfig, FileLengthRuleConfig, MaxDirectoryDepthRuleConfig,
    MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig, NoConsoleRuleConfig,
    NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig, NoInterfaceRuleConfig,
    NoLogicInDomainRuleConfig, RuleConfig, Severity, UpwardImportRuleConfig,
};

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub rule: &'static str,
    pub message: &'static str,
    pub severity: Severity,
    pub detail: Option<String>,
    pub subject: Option<String>,
}

pub fn check_files(
    files: &[PathBuf],
    no_comments: CommentsRuleConfig,
    no_logic_in_barrel: RuleConfig,
    no_default_export: RuleConfig,
    no_export_star: RuleConfig,
    no_inline_types: RuleConfig,
    max_file_exports: FileExportsRuleConfig,
    no_upward_import: UpwardImportRuleConfig,
    no_large_file: FileLengthRuleConfig,
    no_enums: RuleConfig,
    no_barrel_files: RuleConfig,
    no_console: NoConsoleRuleConfig,
    no_debugger: RuleConfig,
    no_eval: RuleConfig,
    no_logic_in_domain: NoLogicInDomainRuleConfig,
    no_empty_interface: RuleConfig,
    no_interface: NoInterfaceRuleConfig,
    no_mutable_exports: RuleConfig,
) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    if !no_comments.severity.is_enabled()
        && !no_logic_in_barrel.severity.is_enabled()
        && !no_default_export.severity.is_enabled()
        && !no_export_star.severity.is_enabled()
        && !no_inline_types.severity.is_enabled()
        && !max_file_exports.severity.is_enabled()
        && !no_upward_import.severity.is_enabled()
        && !no_large_file.severity.is_enabled()
        && !no_enums.severity.is_enabled()
        && !no_barrel_files.severity.is_enabled()
        && !no_console.severity.is_enabled()
        && !no_debugger.severity.is_enabled()
        && !no_eval.severity.is_enabled()
        && !no_logic_in_domain.severity.is_enabled()
        && !no_empty_interface.severity.is_enabled()
        && !no_interface.severity.is_enabled()
        && !no_mutable_exports.severity.is_enabled()
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
        if no_export_star.severity.is_enabled() {
            violations.extend(no_export_star::check_file(file, &source, &no_export_star));
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
        if no_enums.severity.is_enabled() {
            violations.extend(no_enums::check_file(file, &source, &no_enums));
        }
        if no_barrel_files.severity.is_enabled() {
            violations.extend(no_barrel_files::check_file(file, &source, &no_barrel_files));
        }
        if no_console.severity.is_enabled() {
            violations.extend(no_console::check_file(file, &source, &no_console));
        }
        if no_debugger.severity.is_enabled() {
            violations.extend(no_debugger::check_file(file, &source, &no_debugger));
        }
        if no_eval.severity.is_enabled() {
            violations.extend(no_eval::check_file(file, &source, &no_eval));
        }
        if no_logic_in_domain.severity.is_enabled() {
            violations.extend(no_logic_in_domain::check_file(
                file,
                &source,
                &no_logic_in_domain,
            ));
        }
        if no_empty_interface.severity.is_enabled() {
            violations.extend(no_empty_interface::check_file(
                file,
                &source,
                &no_empty_interface,
            ));
        }
        if no_interface.severity.is_enabled() {
            violations.extend(no_interface::check_file(file, &source, &no_interface));
        }
        if no_mutable_exports.severity.is_enabled() {
            violations.extend(no_mutable_exports::check_file(
                file,
                &source,
                &no_mutable_exports,
            ));
        }
    }

    Ok(violations)
}

pub fn check_directories(
    root: &std::path::Path,
    no_empty_directories: NoEmptyDirectoriesRuleConfig,
) -> Vec<Violation> {
    if !no_empty_directories.severity.is_enabled() {
        return Vec::new();
    }

    no_empty_directories::check_directories(root, &no_empty_directories)
}

pub fn check_duplicate_file_names(
    files: &[PathBuf],
    no_duplicate_file_names: NoDuplicateFileNamesRuleConfig,
) -> Vec<Violation> {
    if !no_duplicate_file_names.severity.is_enabled() {
        return Vec::new();
    }

    no_duplicate_file_names::check_files(files, &no_duplicate_file_names)
}

pub fn check_max_items_per_directory(
    root: &std::path::Path,
    config: MaxItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_items_per_directory::check_directories(root, &config)
}

pub fn check_min_items_per_directory(
    root: &std::path::Path,
    config: MinItemsPerDirectoryRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    min_items_per_directory::check_directories(root, &config)
}

pub fn check_max_directory_depth(
    root: &std::path::Path,
    config: MaxDirectoryDepthRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    max_directory_depth::check_directories(root, &config)
}
