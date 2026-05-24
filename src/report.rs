use std::path::PathBuf;

use crate::config::Severity;
use crate::rules::Violation;

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const DEFAULT_MAX_RULE_GROUPS: usize = 6;
const DEFAULT_MAX_FILES_PER_RULE: usize = 6;
const DEFAULT_MAX_LINES_PER_FILE: usize = 8;

#[derive(Debug, Clone)]
pub struct Report {
    files: Vec<PathBuf>,
    violations: Vec<Violation>,
}

impl Report {
    pub fn new(files: Vec<PathBuf>, violations: Vec<Violation>) -> Self {
        Self { files, violations }
    }

    pub fn render_text(&self, verbose: bool) -> String {
        let warning_count = self.count_by_severity(Severity::Warn);
        let error_count = self.count_by_severity(Severity::Error);
        let score = self.score(error_count, warning_count);
        let mut output = String::new();

        output.push_str(&render_header());

        if self.violations.is_empty() {
            output.push_str(&format!(
                "{GREEN}{BOLD}No structural issues found.{RESET}\n"
            ));
        } else {
            let rule_groups = group_by_rule(&self.violations);
            output.push_str(&render_findings(&rule_groups, verbose));
            output.push('\n');
            output.push_str(&render_end_summary(
                self.files.len(),
                self.violations.len(),
                error_count,
                warning_count,
                score,
                &rule_groups,
                verbose,
            ));
        }

        output
    }

    fn count_by_severity(&self, severity: Severity) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.severity == severity)
            .count()
    }

    fn score(&self, error_count: usize, warning_count: usize) -> usize {
        if self.files.is_empty() || self.violations.is_empty() {
            return 100;
        }

        let weighted_findings = error_count.saturating_mul(2) + warning_count;
        let files_scanned = self.files.len().max(1);
        let penalty = weighted_findings.saturating_mul(100) / files_scanned;

        100usize.saturating_sub(penalty)
    }
}

#[derive(Debug)]
struct RuleGroup<'a> {
    severity: Severity,
    rule: &'static str,
    message: &'static str,
    violations: Vec<&'a Violation>,
}

#[derive(Debug)]
struct FileGroup<'a> {
    file: PathBuf,
    violations: Vec<&'a Violation>,
}

fn render_header() -> String {
    format!("{BOLD}Niteo Structure Health{RESET}\n\n")
}

fn render_end_summary(
    file_count: usize,
    violation_count: usize,
    error_count: usize,
    warning_count: usize,
    score: usize,
    rule_groups: &[RuleGroup<'_>],
    verbose: bool,
) -> String {
    let status = status_label(error_count, warning_count);
    let status_color = status_color(error_count, warning_count);
    let score_color = score_color(score);
    let mut output = format!(
        "{score_color}{BOLD}Score {score}/100{RESET}  {status_color}{BOLD}{status}{RESET}\n\
         {DIM}{file_count} files scanned | {violation_count} findings | {error_count} errors | {warning_count} warnings{RESET}\n\n"
    );
    output.push_str(&render_rule_overview(rule_groups, verbose));
    output
}

fn render_rule_overview(rule_groups: &[RuleGroup<'_>], verbose: bool) -> String {
    let visible_count = visible_rule_group_count(rule_groups.len(), verbose);
    let hidden_count = rule_groups.len().saturating_sub(visible_count);
    let mut output = format!("{BOLD}Rule Overview{RESET}\n");

    for (index, group) in rule_groups.iter().take(visible_count).enumerate() {
        let rank = index + 1;
        let color = severity_color(group.severity);
        let label = pluralized_label(group.severity, group.violations.len());
        output.push_str(&format!(
            "  {DIM}{rank:>2}.{RESET} {color}{count:>3} {label:<8}{RESET}  \
             {rule:<22} {message}\n",
            count = group.violations.len(),
            rule = group.rule,
            message = group.message,
        ));
    }

    if hidden_count > 0 {
        output.push_str(&format!(
            "  {DIM}... {hidden_count} more rules hidden. Run with --verbose to show all.{RESET}\n"
        ));
    }

    output
}

fn render_findings(rule_groups: &[RuleGroup<'_>], verbose: bool) -> String {
    let visible_count = visible_rule_group_count(rule_groups.len(), verbose);
    let hidden_count = rule_groups.len().saturating_sub(visible_count);
    let mut output = format!("{BOLD}Findings{RESET}\n");

    for group in rule_groups.iter().take(visible_count) {
        output.push_str(&render_rule_group(group, verbose));
    }

    if hidden_count > 0 {
        output.push_str(&format!(
            "{DIM}Hidden rule groups: {hidden_count}. Run with --verbose for the full report.{RESET}\n"
        ));
    }

    output
}

fn render_rule_group(group: &RuleGroup<'_>, verbose: bool) -> String {
    let color = severity_color(group.severity);
    let label = pluralized_label(group.severity, group.violations.len());
    let file_groups = group_by_file(&group.violations);
    let visible_file_count = visible_file_count(file_groups.len(), verbose);
    let mut output = format!(
        "\n{color}{BOLD}{rule}{RESET} {DIM}{count} {label} in {file_count} files{RESET}\n\
         {DIM}{message}{RESET}\n",
        rule = group.rule,
        count = group.violations.len(),
        file_count = file_groups.len(),
        message = group.message,
    );

    for file_group in file_groups.iter().take(visible_file_count) {
        let line_count = visible_line_count(file_group.violations.len(), verbose);
        output.push_str(&format!(
            "  {CYAN}{}{RESET} {DIM}({} findings){RESET}  {}\n",
            file_group.file.display(),
            file_group.violations.len(),
            render_line_numbers(
                file_group.violations.iter().take(line_count).copied(),
                file_group.violations.len(),
                verbose,
            ),
        ));

        let hidden_lines = file_group.violations.len().saturating_sub(line_count);
        if hidden_lines > 0 {
            output.push_str(&format!(
                "    {DIM}+ {hidden_lines} more locations in this file. Use --verbose to show all.{RESET}\n"
            ));
        }
    }

    let hidden_file_count = file_groups.len().saturating_sub(visible_file_count);
    if hidden_file_count > 0 {
        let hidden_violation_count = file_groups
            .iter()
            .skip(visible_file_count)
            .map(|file_group| file_group.violations.len())
            .sum::<usize>();
        output.push_str(&format!(
            "  {DIM}+ {hidden_file_count} more files with {hidden_violation_count} findings. Use --verbose to show all files.{RESET}\n"
        ));
    }

    output
}

fn group_by_rule<'a>(violations: &'a [Violation]) -> Vec<RuleGroup<'a>> {
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

fn group_by_file<'a>(violations: &[&'a Violation]) -> Vec<FileGroup<'a>> {
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

fn visible_rule_group_count(group_count: usize, verbose: bool) -> usize {
    if verbose {
        return group_count;
    }

    group_count.min(DEFAULT_MAX_RULE_GROUPS)
}

fn visible_file_count(file_count: usize, verbose: bool) -> usize {
    if verbose {
        return file_count;
    }

    file_count.min(DEFAULT_MAX_FILES_PER_RULE)
}

fn visible_line_count(line_count: usize, verbose: bool) -> usize {
    if verbose {
        return line_count;
    }

    line_count.min(DEFAULT_MAX_LINES_PER_FILE)
}

fn render_line_numbers<'a>(
    violations: impl Iterator<Item = &'a Violation>,
    total_count: usize,
    verbose: bool,
) -> String {
    let violations: Vec<&'a Violation> = violations.collect();
    let ranges = if verbose {
        violations
            .iter()
            .map(|v| format!("{}:{}", v.line, v.column))
            .collect::<Vec<String>>()
    } else {
        group_line_ranges(&violations)
    };
    let lines = ranges.join(", ");

    let suffix = if !verbose && total_count > DEFAULT_MAX_LINES_PER_FILE {
        format!(
            ", {DIM}...and {} more{RESET}",
            total_count.saturating_sub(DEFAULT_MAX_LINES_PER_FILE)
        )
    } else {
        String::new()
    };

    format!("{DIM}lines {lines}{suffix}{RESET}")
}

fn group_line_ranges(violations: &[&Violation]) -> Vec<String> {
    if violations.is_empty() {
        return Vec::new();
    }

    let mut ranges: Vec<String> = Vec::new();
    let mut start = violations[0].line;
    let mut end = violations[0].line;

    for violation in violations.iter().skip(1) {
        if violation.line == end + 1 {
            end = violation.line;
        } else {
            if start == end {
                ranges.push(format!("{start}"));
            } else {
                ranges.push(format!("{start}-{end}"));
            }
            start = violation.line;
            end = violation.line;
        }
    }

    if start == end {
        ranges.push(format!("{start}"));
    } else {
        ranges.push(format!("{start}-{end}"));
    }

    ranges
}

fn status_label(error_count: usize, warning_count: usize) -> &'static str {
    if error_count > 0 {
        return "Needs attention";
    }

    if warning_count > 0 {
        return "Review recommended";
    }

    "Healthy"
}

fn status_color(error_count: usize, warning_count: usize) -> &'static str {
    if error_count > 0 {
        return RED;
    }

    if warning_count > 0 {
        return YELLOW;
    }

    GREEN
}

fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => RED,
        Severity::Warn => YELLOW,
        Severity::Off => DIM,
    }
}

fn severity_rank(severity: Severity) -> usize {
    match severity {
        Severity::Error => 0,
        Severity::Warn => 1,
        Severity::Off => 2,
    }
}

fn pluralized_label(severity: Severity, count: usize) -> &'static str {
    match (severity, count) {
        (Severity::Error, 1) => "error",
        (Severity::Error, _) => "errors",
        (Severity::Warn, 1) => "warning",
        (Severity::Warn, _) => "warnings",
        (Severity::Off, 1) => "off",
        (Severity::Off, _) => "off",
    }
}

fn score_color(score: usize) -> &'static str {
    if score >= 75 {
        return GREEN;
    }

    if score >= 50 {
        return YELLOW;
    }

    RED
}
