mod hook_no_jsx;
mod max_directory_depth;
mod max_file_exports;
mod max_items_per_directory;
mod min_items_per_directory;
mod no_barrel_files;
mod no_comments;
mod no_console;
mod no_debugger;
mod no_default_export;
mod no_dump_files;
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
mod prefer_satisfies;

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::{
    MaxDirectoryDepthRuleConfig, MaxItemsPerDirectoryRuleConfig, MinItemsPerDirectoryRuleConfig,
    NoDumpFilesRuleConfig, NoDuplicateFileNamesRuleConfig, NoEmptyDirectoriesRuleConfig,
    ProjectConfig, Severity,
};
use crate::ignore;

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

pub fn check_files(files: &[PathBuf], config: &ProjectConfig) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();

    if !config.no_comments.severity.is_enabled()
        && !config.no_logic_in_barrel.severity.is_enabled()
        && !config.no_default_export.severity.is_enabled()
        && !config.no_export_star.severity.is_enabled()
        && !config.no_inline_types.severity.is_enabled()
        && !config.max_file_exports.severity.is_enabled()
        && !config.no_upward_import.severity.is_enabled()
        && !config.no_large_file.severity.is_enabled()
        && !config.no_enums.severity.is_enabled()
        && !config.no_barrel_files.severity.is_enabled()
        && !config.no_console.severity.is_enabled()
        && !config.no_debugger.severity.is_enabled()
        && !config.no_eval.severity.is_enabled()
        && !config.no_logic_in_domain.severity.is_enabled()
        && !config.no_empty_interface.severity.is_enabled()
        && !config.no_interface.severity.is_enabled()
        && !config.no_mutable_exports.severity.is_enabled()
        && !config.prefer_satisfies.severity.is_enabled()
        && !config.hook_no_jsx.severity.is_enabled()
    {
        return Ok(violations);
    }

    let type_location_style = no_inline_types::TypeLocationStyle::detect(files);

    for file in files {
        let source = fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let directives = ignore::parse_ignore_directives(&source);
        let mut file_violations = Vec::new();

        if config.no_comments.severity.is_enabled() {
            file_violations.extend(no_comments::check_file(file, &source, &config.no_comments));
        }
        if config.no_logic_in_barrel.severity.is_enabled() {
            file_violations.extend(no_logic_in_barrel::check_file(
                file,
                &source,
                &config.no_logic_in_barrel,
            ));
        }
        if config.no_default_export.severity.is_enabled() {
            file_violations.extend(no_default_export::check_file(
                file,
                &source,
                &config.no_default_export,
            ));
        }
        if config.no_export_star.severity.is_enabled() {
            file_violations.extend(no_export_star::check_file(
                file,
                &source,
                &config.no_export_star,
            ));
        }
        if config.no_inline_types.severity.is_enabled() {
            file_violations.extend(no_inline_types::check_file(
                file,
                &source,
                &config.no_inline_types,
                type_location_style,
            ));
        }
        if config.max_file_exports.severity.is_enabled() {
            file_violations.extend(max_file_exports::check_file(
                file,
                &source,
                &config.max_file_exports,
            ));
        }
        if config.no_upward_import.severity.is_enabled() {
            file_violations.extend(no_upward_import::check_file(
                file,
                &source,
                &config.no_upward_import,
            ));
        }
        if config.no_large_file.severity.is_enabled() {
            file_violations.extend(no_large_file::check_file(
                file,
                &source,
                &config.no_large_file,
            ));
        }
        if config.no_enums.severity.is_enabled() {
            file_violations.extend(no_enums::check_file(file, &source, &config.no_enums));
        }
        if config.no_barrel_files.severity.is_enabled() {
            file_violations.extend(no_barrel_files::check_file(
                file,
                &source,
                &config.no_barrel_files,
            ));
        }
        if config.no_console.severity.is_enabled() {
            file_violations.extend(no_console::check_file(file, &source, &config.no_console));
        }
        if config.no_debugger.severity.is_enabled() {
            file_violations.extend(no_debugger::check_file(file, &source, &config.no_debugger));
        }
        if config.no_eval.severity.is_enabled() {
            file_violations.extend(no_eval::check_file(file, &source, &config.no_eval));
        }
        if config.no_logic_in_domain.severity.is_enabled() {
            file_violations.extend(no_logic_in_domain::check_file(
                file,
                &source,
                &config.no_logic_in_domain,
            ));
        }
        if config.no_empty_interface.severity.is_enabled() {
            file_violations.extend(no_empty_interface::check_file(
                file,
                &source,
                &config.no_empty_interface,
            ));
        }
        if config.no_interface.severity.is_enabled() {
            file_violations.extend(no_interface::check_file(
                file,
                &source,
                &config.no_interface,
            ));
        }
        if config.no_mutable_exports.severity.is_enabled() {
            file_violations.extend(no_mutable_exports::check_file(
                file,
                &source,
                &config.no_mutable_exports,
            ));
        }
        if config.prefer_satisfies.severity.is_enabled() {
            file_violations.extend(prefer_satisfies::check_file(
                file,
                &source,
                &config.prefer_satisfies,
            ));
        }
        if config.hook_no_jsx.severity.is_enabled() {
            file_violations.extend(hook_no_jsx::check_file(file, &source, &config.hook_no_jsx));
        }

        file_violations.retain(|v| !ignore::should_suppress_violation(&directives, v.line, v.rule));

        violations.extend(file_violations);
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

pub fn check_dump_files(files: &[PathBuf], config: NoDumpFilesRuleConfig) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    no_dump_files::check_files(files, &config)
}
