use anyhow::Result;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::config::Severity;
use crate::ignore::SuppressionReport;
use crate::rules::Violation;

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
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
    suppression_report: Option<SuppressionReport>,
}

impl Report {
    pub fn new(files: Vec<PathBuf>, violations: Vec<Violation>) -> Self {
        Self {
            files,
            violations,
            suppression_report: None,
        }
    }

    pub fn with_suppression_report(mut self, report: SuppressionReport) -> Self {
        self.suppression_report = Some(report);
        self
    }

    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn render_text(&self, verbose: bool) -> String {
        let warning_count = self.count_by_severity(Severity::Warn);
        let error_count = self.count_by_severity(Severity::Error);
        let info_count = self.count_by_severity(Severity::Info);
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
            let summary = TextSummary {
                file_count: self.files.len(),
                violation_count: self.violations.len(),
                error_count,
                warning_count,
                info_count,
                score,
            };
            output.push_str(&render_end_summary(&summary, &rule_groups, verbose));
        }

        if let Some(ref suppression_report) = self.suppression_report
            && !suppression_report.is_empty()
        {
            let rendered = render_suppression_report_text(suppression_report);
            if !rendered.is_empty() {
                output.push('\n');
                output.push_str(&rendered);
            }
        }

        output
    }

    pub fn render_json(&self) -> Result<String> {
        let mut report = serde_json::Map::new();
        report.insert("summary".to_string(), self.summary_json());
        report.insert(
            "files".to_string(),
            json!(
                self.files
                    .iter()
                    .map(|file| path_to_string(file))
                    .collect::<Vec<String>>()
            ),
        );
        report.insert(
            "violations".to_string(),
            json!(
                self.violations
                    .iter()
                    .map(violation_json)
                    .collect::<Vec<Value>>()
            ),
        );

        if let Some(ref suppression_report) = self.suppression_report {
            report.insert(
                "suppressions".to_string(),
                suppression_report_json(suppression_report),
            );
        }

        Ok(serde_json::to_string_pretty(&Value::Object(report))?)
    }

    pub fn render_sarif(&self) -> Result<String> {
        let rules = group_by_rule(&self.violations)
            .iter()
            .map(|group| {
                json!({
                    "id": group.rule,
                    "name": group.rule,
                    "shortDescription": {
                        "text": group.message,
                    },
                    "defaultConfiguration": {
                        "level": sarif_level(group.severity),
                    },
                })
            })
            .collect::<Vec<Value>>();

        let results = self
            .violations
            .iter()
            .map(sarif_result_json)
            .collect::<Vec<Value>>();

        let report = json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [
                {
                    "tool": {
                        "driver": {
                            "name": "Niteo",
                            "informationUri": "https://github.com/FrozenProductions/Niteo",
                            "rules": rules,
                        },
                    },
                    "results": results,
                    "properties": {
                        "summary": self.summary_json(),
                    },
                },
            ],
        });

        Ok(serde_json::to_string_pretty(&report)?)
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

    fn summary_json(&self) -> Value {
        let warning_count = self.count_by_severity(Severity::Warn);
        let error_count = self.count_by_severity(Severity::Error);
        let info_count = self.count_by_severity(Severity::Info);

        json!({
            "filesScanned": self.files.len(),
            "violations": self.violations.len(),
            "errors": error_count,
            "warnings": warning_count,
            "info": info_count,
            "score": self.score(error_count, warning_count),
            "status": status_label(error_count, warning_count, info_count),
        })
    }
}

fn violation_json(violation: &Violation) -> Value {
    json!({
        "file": path_to_string(&violation.file),
        "line": violation.line,
        "column": violation.column,
        "rule": violation.rule,
        "message": violation.message,
        "severity": severity_label(violation.severity),
        "detail": violation.detail,
        "subject": violation.subject,
    })
}

fn sarif_result_json(violation: &Violation) -> Value {
    let mut message = violation.message.to_string();
    if let Some(detail) = &violation.detail {
        message.push(' ');
        message.push_str(detail);
    }

    json!({
        "ruleId": violation.rule,
        "level": sarif_level(violation.severity),
        "message": {
            "text": message,
        },
        "locations": [
            {
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": path_to_string(&violation.file),
                    },
                    "region": {
                        "startLine": violation.line.unwrap_or(1),
                        "startColumn": violation.column.unwrap_or(1),
                    },
                },
            },
        ],
        "properties": {
            "severity": severity_label(violation.severity),
            "subject": violation.subject,
        },
    })
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
        Severity::Info => "info",
        Severity::Off => "off",
    }
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
        Severity::Info => "note",
        Severity::Off => "none",
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

#[derive(Debug)]
struct TextSummary {
    file_count: usize,
    violation_count: usize,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    score: usize,
}

fn render_header() -> String {
    format!("{BOLD}Niteo Structure Health{RESET}\n\n")
}

fn render_end_summary(
    summary: &TextSummary,
    rule_groups: &[RuleGroup<'_>],
    verbose: bool,
) -> String {
    let status = status_label(
        summary.error_count,
        summary.warning_count,
        summary.info_count,
    );
    let status_color = status_color(
        summary.error_count,
        summary.warning_count,
        summary.info_count,
    );
    let score_color = score_color(summary.score);
    let mut output = format!(
        "{score_color}{BOLD}Score {score}/100{RESET}  {status_color}{BOLD}{status}{RESET}\n\
         {DIM}{file_count} files scanned | {violation_count} findings | {error_count} errors | {warning_count} warnings | {info_count} info{RESET}\n\n",
        score = summary.score,
        file_count = summary.file_count,
        violation_count = summary.violation_count,
        error_count = summary.error_count,
        warning_count = summary.warning_count,
        info_count = summary.info_count,
    );
    output.push_str(&render_rule_overview(rule_groups, verbose));
    output
}

fn render_rule_overview(rule_groups: &[RuleGroup<'_>], verbose: bool) -> String {
    let visible_count = visible_rule_group_count(rule_groups.len(), verbose);
    let visible_groups = rule_groups.split_at(visible_count.min(rule_groups.len())).0;
    let hidden_count = rule_groups.len().saturating_sub(visible_count);

    let max_count_width = visible_groups
        .iter()
        .map(|g| g.violations.len().to_string().len())
        .max()
        .unwrap_or(1);
    let max_rule_width = visible_groups
        .iter()
        .map(|g| g.rule.len())
        .max()
        .unwrap_or(1);

    let mut output = format!("{BOLD}Rule Overview{RESET}\n");
    let mut rank = 0;
    let mut last_severity: Option<Severity> = None;

    for group in visible_groups {
        if last_severity != Some(group.severity) {
            if last_severity.is_some() {
                output.push_str(&format!("{DIM}{}{RESET}\n", "─".repeat(60)));
            }

            let color = severity_color(group.severity);
            let header = pluralized_header(group.severity);
            output.push_str(&format!("\n{color}{BOLD}{header}{RESET}\n"));
            output.push_str(&format!("{DIM}{}{RESET}\n", "─".repeat(60)));
            last_severity = Some(group.severity);
        }

        rank += 1;
        let color = severity_color(group.severity);
        let label = pluralized_label(group.severity);
        let count_str = format!("{:>cw$}", group.violations.len(), cw = max_count_width);
        let rule_str = format!("{:<rw$}", group.rule, rw = max_rule_width);
        output.push_str(&format!(
            "  {DIM}{rank:>2}.{RESET} {color}{count_str}{RESET}  {label}  \
              {rule_str} {message}\n",
            message = group.message,
        ));
    }

    if !visible_groups.is_empty() {
        output.push_str(&format!("{DIM}{}{RESET}\n", "─".repeat(60)));
    }

    if hidden_count > 0 {
        output.push_str(&format!(
            "\n{DIM}... {hidden_count} more rules hidden. Run with --verbose to show all.{RESET}\n"
        ));
    }

    output
}

fn pluralized_header(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Errors",
        Severity::Warn => "Warnings",
        Severity::Info => "Suggestions",
        Severity::Off => "Off",
    }
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
    let label = pluralized_label(group.severity);
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
        let has_details = file_group
            .violations
            .iter()
            .any(|v| v.detail.is_some() || v.subject.is_some());

        if has_details {
            let visible_violations = file_group.violations.iter().take(line_count);
            for violation in visible_violations {
                let subject = violation
                    .subject
                    .as_ref()
                    .map(|s| format!("{BOLD}{s}{RESET} "))
                    .unwrap_or_default();
                let detail = violation
                    .detail
                    .as_ref()
                    .map(|d| format!(" {DIM}{d}{RESET}"))
                    .unwrap_or_default();
                let location = match (violation.line, violation.column) {
                    (Some(line), Some(column)) => format!("lines {line}:{column}"),
                    (Some(line), None) => format!("line {line}"),
                    _ => String::new(),
                };
                let location_suffix = if location.is_empty() {
                    String::new()
                } else {
                    format!(" {location}")
                };
                output.push_str(&format!(
                    "  {CYAN}{}{RESET}  {subject}{location_suffix}{detail}\n",
                    file_group.file.display(),
                ));
            }
        } else {
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
        }

        let hidden_lines = file_group.violations.len().saturating_sub(line_count);
        if hidden_lines > 0 && !has_details {
            output.push_str(&format!(
                "    {DIM}+ {hidden_lines} more locations in this file. Use --verbose to show all.{RESET}\n"
            ));
        } else if hidden_lines > 0 {
            output.push_str(&format!(
                "    {DIM}+ {hidden_lines} more findings in this file. Use --verbose to show all.{RESET}\n"
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
    let positioned: Vec<&'a Violation> = violations
        .iter()
        .filter(|v| v.line.is_some())
        .copied()
        .collect();
    let ranges = if verbose {
        positioned
            .iter()
            .filter_map(|v| {
                v.line
                    .map(|line| format!("{}:{}", line, v.column.unwrap_or(1)))
            })
            .collect::<Vec<String>>()
    } else {
        group_line_ranges(&positioned)
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

    if lines.is_empty() {
        format!("{DIM}{suffix}{RESET}").trim().to_string()
    } else {
        format!("{DIM}lines {lines}{suffix}{RESET}")
    }
}

fn group_line_ranges(violations: &[&Violation]) -> Vec<String> {
    if violations.is_empty() {
        return Vec::new();
    }

    let mut ranges: Vec<String> = Vec::new();
    let mut start = violations[0].line.unwrap_or(1);
    let mut end = violations[0].line.unwrap_or(1);

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

fn status_label(error_count: usize, warning_count: usize, info_count: usize) -> &'static str {
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

fn status_color(error_count: usize, warning_count: usize, info_count: usize) -> &'static str {
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

fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => RED,
        Severity::Warn => YELLOW,
        Severity::Info => BLUE,
        Severity::Off => DIM,
    }
}

fn severity_rank(severity: Severity) -> usize {
    match severity {
        Severity::Error => 0,
        Severity::Warn => 1,
        Severity::Info => 2,
        Severity::Off => 3,
    }
}

fn pluralized_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "errors",
        Severity::Warn => "warnings",
        Severity::Info => "suggestions",
        Severity::Off => "off",
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

pub fn render_suppression_report_text(report: &SuppressionReport) -> String {
    if report.total_suppressed() == 0 && report.total_stale() == 0 {
        return String::new();
    }

    let mut output = String::new();
    let total_suppressed = report.total_suppressed();
    let total_stale = report.total_stale();

    output.push_str(&format!("{BOLD}Suppressions{RESET}\n"));

    if total_suppressed > 0 {
        output.push_str(&format!(
            "  {DIM}{} violations suppressed by ignore directives{RESET}\n",
            total_suppressed,
        ));
    }

    if total_stale > 0 {
        output.push_str(&format!(
            "  {YELLOW}{BOLD}{} stale directives found{RESET}\n",
            total_stale,
        ));

        for file_info in &report.files {
            if file_info.stale_directives.is_empty() {
                continue;
            }

            output.push_str(&format!("  {CYAN}{}{RESET}\n", file_info.file.display(),));

            for directive in &file_info.stale_directives {
                output.push_str(&format!(
                    "    {YELLOW}line {}{RESET}  {}\n",
                    directive.line, directive,
                ));
            }
        }
    }

    output
}

fn suppression_report_json(report: &SuppressionReport) -> Value {
    json!({
        "totalSuppressed": report.total_suppressed(),
        "totalStale": report.total_stale(),
        "files": report
            .files
            .iter()
            .map(|file_info| {
                json!({
                    "file": path_to_string(&file_info.file),
                    "suppressedCount": file_info.suppressed_count,
                    "staleDirectives": file_info
                        .stale_directives
                        .iter()
                        .map(|d| {
                            json!({
                                "kind": format!("{}", d.kind),
                                "line": d.line,
                                "rules": d.rules,
                            })
                        })
                        .collect::<Vec<Value>>(),
                })
            })
            .collect::<Vec<Value>>(),
    })
}
