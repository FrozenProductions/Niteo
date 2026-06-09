use crate::ignore::SuppressionReport;
use serde_json::{Value, json};

use crate::report::model::path_to_string;
use crate::report::summary::{BOLD, CYAN, DIM, RESET, YELLOW};

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

pub fn suppression_report_json(report: &SuppressionReport) -> Value {
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
                        .map(|directive| {
                            json!({
                                "kind": format!("{}", directive.kind),
                                "line": directive.line,
                                "rules": directive.rules,
                            })
                        })
                        .collect::<Vec<Value>>(),
                })
            })
            .collect::<Vec<Value>>(),
    })
}
