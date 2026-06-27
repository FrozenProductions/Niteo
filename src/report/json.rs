use anyhow::Result;
use serde_json::{Value, json};

use crate::diagnostics::Diagnostic;
use crate::report::model::{Report, path_to_string, severity_label};
use crate::report::summary::score as calc_score;
use crate::report::suppressions::suppression_report_json;
use crate::rules::Violation;

impl Report {
    pub fn render_json(&self) -> Result<String> {
        let mut report = serde_json::Map::new();
        report.insert("summary".to_string(), summary_json(self));
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

        report.insert(
            "diagnostics".to_string(),
            json!(
                self.diagnostics
                    .iter()
                    .map(diagnostic_json)
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
}

pub fn summary_json(report: &Report) -> Value {
    let warning_count = report.warning_count();
    let error_count = report.error_count();
    let info_count = report.info_count();

    json!({
        "filesScanned": report.files.len(),
        "violations": report.violations.len(),
        "errors": error_count,
        "warnings": warning_count,
        "info": info_count,
        "score": calc_score(error_count, warning_count, report.files.len()),
        "status": status_label(error_count, warning_count, info_count),
    })
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

fn diagnostic_json(diagnostic: &Diagnostic) -> Value {
    json!({
        "category": diagnostic.category.as_str(),
        "message": diagnostic.message,
    })
}

#[cfg(test)]
mod tests {
    use crate::diagnostics::{Diagnostic, DiagnosticCategory};
    use crate::report::model::Report;
    use serde_json::Value;

    #[test]
    fn json_report_renders_diagnostics() {
        let report = Report::new(vec![], vec![]).with_diagnostics(vec![Diagnostic::new(
            DiagnosticCategory::Cache,
            "failed to clear cache",
        )]);

        let rendered = report.render_json().unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        let diagnostics = parsed["diagnostics"].as_array().unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["category"], "cache");
        assert_eq!(diagnostics[0]["message"], "failed to clear cache");
    }
}
