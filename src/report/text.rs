use crate::config::Severity;
use crate::report::model::Report;
use crate::report::summary::{
    BOLD, CYAN, DIM, GREEN, RESET, TextSummary, YELLOW, group_by_file, group_by_rule,
    pluralized_header, pluralized_label, score, score_color, severity_color, status_color,
    status_label, visible_file_count, visible_line_count, visible_rule_group_count,
};
use crate::report::suppressions::render_suppression_report_text;

impl Report {
    pub fn render_text(&self, verbose: bool) -> String {
        let warning_count = self.warning_count();
        let error_count = self.error_count();
        let info_count = self.info_count();
        let calc_score = score(error_count, warning_count, self.files.len());
        let mut output = String::new();

        output.push_str(&render_header());
        output.push_str(&render_diagnostics(&self.diagnostics));

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
                score: calc_score,
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
}

fn render_header() -> String {
    format!("{BOLD}Niteo Structure Health{RESET}\n\n")
}

fn render_diagnostics(diagnostics: &[crate::diagnostics::Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return String::new();
    }

    let mut output = format!("{BOLD}Diagnostics{RESET}\n");
    for diagnostic in diagnostics {
        output.push_str(&format!(
            "  {YELLOW}warning{RESET}: {message}\n",
            message = diagnostic.message
        ));
    }
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::{Diagnostic, DiagnosticCategory};
    use crate::report::model::Report;

    #[test]
    fn text_report_renders_diagnostics_section() {
        let report = Report::new(vec![], vec![]).with_diagnostics(vec![Diagnostic::new(
            DiagnosticCategory::Cache,
            "failed to prepare cache",
        )]);

        let rendered = report.render_text(false);
        assert!(rendered.contains("Diagnostics"));
        assert!(rendered.contains("failed to prepare cache"));
    }

    #[test]
    fn text_report_omits_diagnostics_when_empty() {
        let report = Report::new(vec![], vec![]);
        let rendered = report.render_text(false);
        assert!(!rendered.contains("Diagnostics"));
    }
}

fn render_end_summary(
    summary: &TextSummary,
    rule_groups: &[super::summary::RuleGroup<'_>],
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

fn render_rule_overview(rule_groups: &[super::summary::RuleGroup<'_>], verbose: bool) -> String {
    let visible_count = visible_rule_group_count(rule_groups.len(), verbose);
    let visible_groups = rule_groups.split_at(visible_count.min(rule_groups.len())).0;
    let hidden_count = rule_groups.len().saturating_sub(visible_count);

    let max_count_width = visible_groups
        .iter()
        .map(|group| group.violations.len().to_string().len())
        .max()
        .unwrap_or(1);
    let max_rule_width = visible_groups
        .iter()
        .map(|group| group.rule.len())
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

fn render_findings(rule_groups: &[super::summary::RuleGroup<'_>], verbose: bool) -> String {
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

fn render_rule_group(group: &super::summary::RuleGroup<'_>, verbose: bool) -> String {
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
            .any(|violation| violation.detail.is_some() || violation.subject.is_some());

        if has_details {
            let visible_violations = file_group.violations.iter().take(line_count);
            for violation in visible_violations {
                let subject = violation
                    .subject
                    .as_ref()
                    .map(|subject| format!("{BOLD}{subject}{RESET} "))
                    .unwrap_or_default();
                let detail = violation
                    .detail
                    .as_ref()
                    .map(|detail| format!(" {DIM}{detail}{RESET}"))
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

fn render_line_numbers<'a>(
    violations: impl Iterator<Item = &'a crate::rules::Violation>,
    total_count: usize,
    verbose: bool,
) -> String {
    let violations: Vec<&'a crate::rules::Violation> = violations.collect();
    let positioned: Vec<&'a crate::rules::Violation> = violations
        .iter()
        .filter(|violation| violation.line.is_some())
        .copied()
        .collect();
    let ranges = if verbose {
        positioned
            .iter()
            .filter_map(|violation| {
                violation
                    .line
                    .map(|line| format!("{}:{}", line, violation.column.unwrap_or(1)))
            })
            .collect::<Vec<String>>()
    } else {
        super::summary::group_line_ranges(&positioned)
    };
    let lines = ranges.join(", ");

    let suffix = if !verbose && total_count > super::summary::visible_line_count(total_count, false)
    {
        format!(
            ", {DIM}...and {} more{RESET}",
            total_count.saturating_sub(super::summary::visible_line_count(total_count, false))
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
