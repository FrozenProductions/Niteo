use anyhow::Result;
use serde_json::json;

use crate::report::json::summary_json;
use crate::report::model::{Report, path_to_string, severity_label, with_record_type};
use crate::report::suppressions::suppression_report_json;
use crate::rules::Violation;

impl Report {
    pub fn render_ndjson(&self) -> Result<String> {
        let mut lines = Vec::new();

        let summary = with_record_type(summary_json(self), "summary");
        lines.push(serde_json::to_string(&summary)?);

        for file in &self.files {
            let record = json!({
                "type": "file",
                "file": path_to_string(file),
            });
            lines.push(serde_json::to_string(&record)?);
        }

        for violation in &self.violations {
            let record = with_record_type(violation_json(violation), "violation");
            lines.push(serde_json::to_string(&record)?);
        }

        if let Some(ref suppression_report) = self.suppression_report {
            let suppression =
                with_record_type(suppression_report_json(suppression_report), "suppressions");
            lines.push(serde_json::to_string(&suppression)?);
        }

        Ok(lines.join("\n"))
    }
}

fn violation_json(violation: &Violation) -> serde_json::Value {
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
