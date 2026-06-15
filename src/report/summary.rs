use crate::config::Severity;
use crate::rules::Violation;
use std::path::PathBuf;

pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";
pub const GREEN: &str = "\x1b[32m";
pub const CYAN: &str = "\x1b[36m";
pub const BLUE: &str = "\x1b[34m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";

#[derive(Debug)]
pub struct RuleGroup<'a> {
    pub severity: Severity,
    pub rule: &'static str,
    pub message: &'static str,
    pub violations: Vec<&'a Violation>,
}

#[derive(Debug)]
pub struct FileGroup<'a> {
    pub file: PathBuf,
    pub violations: Vec<&'a Violation>,
}

#[derive(Debug)]
pub struct TextSummary {
    pub file_count: usize,
    pub violation_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub score: usize,
}

pub fn score(error_count: usize, warning_count: usize, file_count: usize) -> usize {
    if file_count == 0 {
        return 100;
    }

    let weighted_findings = error_count.saturating_mul(2) + warning_count;
    let files_scanned = file_count.max(1);
    let penalty = weighted_findings.saturating_mul(100) / files_scanned;

    100usize.saturating_sub(penalty)
}

pub fn status_label(error_count: usize, warning_count: usize, info_count: usize) -> &'static str {
    if error_count > 0 {
        return "Needs attention";
    }

    if warning_count > 0 {
        return "Review recommended";
    }

    if info_count > 0 {
        return "Suggestions available";
    }

    "Healthy"
}

pub fn status_color(error_count: usize, warning_count: usize, info_count: usize) -> &'static str {
    if error_count > 0 {
        return RED;
    }

    if warning_count > 0 {
        return YELLOW;
    }

    if info_count > 0 {
        return BLUE;
    }

    GREEN
}

pub fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => RED,
        Severity::Warn => YELLOW,
        Severity::Info => BLUE,
        Severity::Off => DIM,
    }
}

pub fn severity_rank(severity: Severity) -> usize {
    match severity {
        Severity::Error => 0,
        Severity::Warn => 1,
        Severity::Info => 2,
        Severity::Off => 3,
    }
}

pub fn pluralized_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "errors",
        Severity::Warn => "warnings",
        Severity::Info => "suggestions",
        Severity::Off => "off",
    }
}

pub fn pluralized_header(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Errors",
        Severity::Warn => "Warnings",
        Severity::Info => "Suggestions",
        Severity::Off => "Off",
    }
}

pub fn score_color(score: usize) -> &'static str {
    if score >= 75 {
        return GREEN;
    }

    if score >= 50 {
        return YELLOW;
    }

    RED
}

pub fn group_by_rule<'a>(violations: &'a [Violation]) -> Vec<RuleGroup<'a>> {
    let mut groups: Vec<RuleGroup<'a>> = Vec::new();

    for violation in violations {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.severity == violation.severity && group.rule == violation.rule)
        {
            group.violations.push(violation);
            continue;
        }

        groups.push(RuleGroup {
            severity: violation.severity,
            rule: violation.rule,
            message: violation.message,
            violations: vec![violation],
        });
    }

    for group in &mut groups {
        group.violations.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then(left.line.cmp(&right.line))
                .then(left.column.cmp(&right.column))
                .then(left.rule.cmp(right.rule))
        });
    }

    groups.sort_by(|left, right| {
        severity_rank(left.severity)
            .cmp(&severity_rank(right.severity))
            .then(right.violations.len().cmp(&left.violations.len()))
            .then(left.rule.cmp(right.rule))
    });
    groups
}

pub fn group_by_file<'a>(violations: &[&'a Violation]) -> Vec<FileGroup<'a>> {
    let mut groups: Vec<FileGroup<'a>> = Vec::new();

    for violation in violations {
        if let Some(group) = groups.iter_mut().find(|group| group.file == violation.file) {
            group.violations.push(violation);
            continue;
        }

        groups.push(FileGroup {
            file: violation.file.clone(),
            violations: vec![violation],
        });
    }

    groups.sort_by(|left, right| left.file.cmp(&right.file));
    groups
}

pub fn visible_rule_group_count(group_count: usize, verbose: bool) -> usize {
    if verbose {
        return group_count;
    }

    const DEFAULT_MAX_RULE_GROUPS: usize = 6;
    group_count.min(DEFAULT_MAX_RULE_GROUPS)
}

pub fn visible_file_count(file_count: usize, verbose: bool) -> usize {
    if verbose {
        return file_count;
    }

    const DEFAULT_MAX_FILES_PER_RULE: usize = 6;
    file_count.min(DEFAULT_MAX_FILES_PER_RULE)
}

pub fn visible_line_count(line_count: usize, verbose: bool) -> usize {
    if verbose {
        return line_count;
    }

    const DEFAULT_MAX_LINES_PER_FILE: usize = 8;
    line_count.min(DEFAULT_MAX_LINES_PER_FILE)
}

pub fn group_line_ranges(violations: &[&Violation]) -> Vec<String> {
    if violations.is_empty() {
        return Vec::new();
    }

    let mut ranges: Vec<String> = Vec::new();
    let first = match violations.first() {
        Some(violation) => violation,
        None => return Vec::new(),
    };
    let mut start = first.line.unwrap_or(1);
    let mut end = first.line.unwrap_or(1);

    for violation in violations.iter().skip(1) {
        let line = violation.line.unwrap_or(1);
        if line == end + 1 {
            end = line;
        } else {
            if start == end {
                ranges.push(format!("{start}"));
            } else {
                ranges.push(format!("{start}-{end}"));
            }
            start = line;
            end = line;
        }
    }

    if start == end {
        ranges.push(format!("{start}"));
    } else {
        ranges.push(format!("{start}-{end}"));
    }

    ranges
}
