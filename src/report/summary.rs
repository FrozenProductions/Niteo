use crate::config::Severity;
use crate::rules::Violation;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    let mut groups: HashMap<(Severity, &'static str), RuleGroup<'a>> = HashMap::new();

    for violation in violations {
        let key = (violation.severity, violation.rule);
        let group = groups.entry(key).or_insert_with(|| RuleGroup {
            severity: violation.severity,
            rule: violation.rule,
            message: violation.message,
            violations: Vec::new(),
        });
        group.violations.push(violation);
    }

    let mut groups: Vec<RuleGroup<'a>> = groups.into_values().collect();

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
    let mut groups: HashMap<&'a Path, FileGroup<'a>> = HashMap::new();

    for violation in violations {
        let group = groups
            .entry(violation.file.as_path())
            .or_insert_with(|| FileGroup {
                file: violation.file.clone(),
                violations: Vec::new(),
            });
        group.violations.push(violation);
    }

    let mut groups: Vec<FileGroup<'a>> = groups.into_values().collect();
    groups.sort_by(|left, right| left.file.cmp(&right.file));
    groups
}

pub fn visible_rule_group_count(group_count: usize, verbose: u8) -> usize {
    if verbose >= 1 {
        return group_count;
    }

    const DEFAULT_MAX_RULE_GROUPS: usize = 6;
    group_count.min(DEFAULT_MAX_RULE_GROUPS)
}

pub fn visible_file_count(file_count: usize, verbose: u8) -> usize {
    if verbose >= 1 {
        return file_count;
    }

    const DEFAULT_MAX_FILES_PER_RULE: usize = 6;
    file_count.min(DEFAULT_MAX_FILES_PER_RULE)
}

pub fn visible_line_count(line_count: usize, verbose: u8) -> usize {
    if verbose >= 1 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn violation(
        file: &str,
        line: usize,
        rule: &'static str,
        message: &'static str,
        severity: Severity,
    ) -> Violation {
        Violation {
            file: PathBuf::from(file),
            span: None,
            line: Some(line),
            column: Some(1),
            rule,
            message,
            severity,
            detail: None,
            subject: None,
        }
    }

    #[test]
    fn group_by_rule_groups_by_severity_and_rule() {
        let first = violation("a.ts", 1, "no-any", "message a", Severity::Error);
        let second = violation("b.ts", 2, "no-any", "message a", Severity::Error);
        let third = violation("c.ts", 3, "no-console", "message b", Severity::Warn);
        let fourth = violation("d.ts", 4, "no-any", "message a", Severity::Warn);

        let violations = [first, second, third, fourth];
        let groups = group_by_rule(&violations);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].rule, "no-any");
        assert_eq!(groups[0].severity, Severity::Error);
        assert_eq!(groups[0].violations.len(), 2);
        assert_eq!(groups[1].rule, "no-any");
        assert_eq!(groups[1].severity, Severity::Warn);
        assert_eq!(groups[1].violations.len(), 1);
        assert_eq!(groups[2].rule, "no-console");
        assert_eq!(groups[2].severity, Severity::Warn);
        assert_eq!(groups[2].violations.len(), 1);
    }

    #[test]
    fn group_by_file_groups_by_file_sorted() {
        let first = violation("b.ts", 2, "no-any", "message a", Severity::Error);
        let second = violation("a.ts", 1, "no-any", "message a", Severity::Error);
        let third = violation("b.ts", 3, "no-console", "message b", Severity::Warn);

        let groups = group_by_file(&[&first, &second, &third]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].file, PathBuf::from("a.ts"));
        assert_eq!(groups[0].violations.len(), 1);
        assert_eq!(groups[1].file, PathBuf::from("b.ts"));
        assert_eq!(groups[1].violations.len(), 2);
    }
}
