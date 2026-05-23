use std::path::PathBuf;

use crate::config::Severity;
use crate::rules::Violation;

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const WHITE: &str = "\x1b[37m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const MAX_LOCATIONS_PER_FILE: usize = 5;

#[derive(Debug, Clone)]
pub struct Report {
    files: Vec<PathBuf>,
    violations: Vec<Violation>,
}

impl Report {
    pub fn new(files: Vec<PathBuf>, violations: Vec<Violation>) -> Self {
        Self { files, violations }
    }

    pub fn render_text(&self) -> String {
        let mut output = format!("{BOLD}Niteo report{RESET}\n\n");

        if self.violations.is_empty() {
            output.push_str(&format!("{GREEN}No violations found{RESET}\n\n"));
        } else {
            for group in group_violations(&self.violations) {
                output.push_str(&render_rule_group(&group));
            }
        }

        let warning_count = self.count_by_severity(Severity::Warn);
        let error_count = self.count_by_severity(Severity::Error);
        let confidence_score = self.confidence_score(error_count, warning_count);

        output.push('\n');
        output.push_str(&format!("{BOLD}Summary{RESET}\n"));
        output.push_str(&format!("Files scanned: {}\n", self.files.len()));
        output.push_str(&format!("Errors: {error_count}\n"));
        output.push_str(&format!("Warnings: {warning_count}\n"));
        output.push_str(&format!("Total violations: {}\n", self.violations.len()));
        output.push_str(&format!("Confidence score: {confidence_score}%\n"));

        output
    }

    fn count_by_severity(&self, severity: Severity) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.severity == severity)
            .count()
    }

    fn confidence_score(&self, error_count: usize, warning_count: usize) -> usize {
        if self.files.is_empty() {
            return 100;
        }

        let penalty = error_count.saturating_mul(20) + warning_count.saturating_mul(5);
        100usize.saturating_sub(penalty)
    }
}

#[derive(Debug)]
struct RuleGroup<'a> {
    severity: Severity,
    rule: &'static str,
    violations: Vec<&'a Violation>,
}

#[derive(Debug)]
struct FileGroup<'a> {
    file: PathBuf,
    violations: Vec<&'a Violation>,
}

fn group_violations(violations: &[Violation]) -> Vec<RuleGroup<'_>> {
    let mut groups: Vec<RuleGroup<'_>> = Vec::new();

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
            violations: vec![violation],
        });
    }

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

    groups
}

fn render_rule_group(group: &RuleGroup<'_>) -> String {
    let color = severity_color(group.severity);
    let mut output = format!(
        "{color}{label}{RESET} {rule} (x{count})\n",
        label = group.severity.label(),
        rule = group.rule,
        count = group.violations.len(),
    );

    for file_group in group_by_file(&group.violations) {
        output.push_str(&format!(
            "  {WHITE}{}{RESET} {DIM}(x{}){RESET}\n",
            file_group.file.display(),
            file_group.violations.len()
        ));

        for violation in file_group.violations.iter().take(MAX_LOCATIONS_PER_FILE) {
            output.push_str(&format!(
                "    {DIM}line {line}, column {column}{RESET}\n",
                line = violation.line,
                column = violation.column
            ));
        }

        let hidden_count = file_group
            .violations
            .len()
            .saturating_sub(MAX_LOCATIONS_PER_FILE);
        if hidden_count > 0 {
            output.push_str(&format!(
                "    {DIM}... {hidden_count} more omitted{RESET}\n"
            ));
        }
    }

    output.push('\n');
    output
}

fn severity_color(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => RED,
        Severity::Warn => YELLOW,
        Severity::Off => DIM,
    }
}
