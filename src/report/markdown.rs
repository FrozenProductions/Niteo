use anyhow::Result;

use crate::report::model::{Report, path_to_string, severity_label};
use crate::report::summary::score as calc_score;

impl Report {
    pub fn render_markdown(&self) -> Result<String> {
        let mut output = String::new();
        let error_count = self.error_count();
        let warning_count = self.warning_count();
        let info_count = self.info_count();
        let score = calc_score(error_count, warning_count, self.files.len());

        output.push_str("# Niteo Lint Results\n\n");

        if !self.diagnostics.is_empty() {
            output.push_str("## Diagnostics\n\n");
            for diagnostic in &self.diagnostics {
                output.push_str(&format!(
                    "- **{}**: {}\n",
                    diagnostic.category.as_str(),
                    diagnostic.message
                ));
            }
            output.push('\n');
        }

        if !self.parse_failures.is_empty() {
            output.push_str("## Parse Errors\n\n");
            for failure in &self.parse_failures {
                output.push_str(&format!(
                    "- **{}**: {}\n",
                    path_to_string(&failure.file),
                    failure.message
                ));
            }
            output.push('\n');
        }

        if !self.violations.is_empty() {
            output.push_str("## Violations\n\n");
            output.push_str("| File | Line | Col | Rule | Severity | Message |\n");
            output.push_str("|------|------|-----|------|----------|--------|\n");

            for violation in &self.violations {
                let file = path_to_string(&violation.file);
                let line = violation
                    .line
                    .map_or_else(|| "-".to_string(), |l| l.to_string());
                let col = violation
                    .column
                    .map_or_else(|| "-".to_string(), |c| c.to_string());
                let detail = violation.detail.as_deref().unwrap_or(violation.message);

                output.push_str(&format!(
                    "| `{}` | {} | {} | `{}` | {} | {} |\n",
                    file,
                    line,
                    col,
                    violation.rule,
                    severity_label(violation.severity),
                    detail
                ));
            }
            output.push('\n');
        }

        let status = if error_count > 0 {
            "Needs attention"
        } else if warning_count > 0 {
            "Review recommended"
        } else if info_count > 0 {
            "Suggestions available"
        } else {
            "Healthy"
        };

        output.push_str("## Summary\n\n");
        output.push_str("| Metric | Value |\n");
        output.push_str("|--------|-------|\n");
        output.push_str(&format!("| Files scanned | {} |\n", self.files.len()));
        output.push_str(&format!("| Violations | {} |\n", self.violations.len()));
        output.push_str(&format!("| Errors | {} |\n", error_count));
        output.push_str(&format!("| Warnings | {} |\n", warning_count));
        output.push_str(&format!("| Info | {} |\n", info_count));
        output.push_str(&format!("| Score | {} |\n", score));
        output.push_str(&format!("| Status | {} |\n", status));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Severity;
    use crate::diagnostics::{Diagnostic, DiagnosticCategory};
    use crate::report::model::Report;
    use crate::rules::Violation;
    use anyhow::Result;
    use std::path::PathBuf;

    #[test]
    fn test_render_markdown_no_violations() -> Result<()> {
        let report = Report::new(vec![PathBuf::from("src/main.ts")], vec![]);
        let output = report.render_markdown()?;
        assert!(output.contains("Niteo Lint Results"));
        assert!(output.contains("Healthy"));
        assert!(output.contains("| Files scanned | 1 |"));
        assert!(output.contains("| Violations | 0 |"));
        Ok(())
    }

    #[test]
    fn test_render_markdown_with_violations() -> Result<()> {
        let violations = vec![
            Violation {
                file: PathBuf::from("src/main.ts"),
                span: None,
                line: Some(10),
                column: Some(5),
                rule: "no-any",
                message: "No `any` type",
                severity: Severity::Error,
                detail: Some("Use `unknown` instead".to_string()),
                subject: None,
            },
            Violation {
                file: PathBuf::from("src/utils.ts"),
                span: None,
                line: Some(42),
                column: Some(12),
                rule: "max-file-lines",
                message: "File has 150 lines",
                severity: Severity::Warn,
                detail: None,
                subject: None,
            },
        ];
        let report = Report::new(
            vec![PathBuf::from("src/main.ts"), PathBuf::from("src/utils.ts")],
            violations,
        );
        let output = report.render_markdown()?;
        assert!(output.contains("no-any"));
        assert!(output.contains("max-file-lines"));
        assert!(output.contains("Use `unknown` instead"));
        assert!(output.contains("| Violations | 2 |"));
        Ok(())
    }

    #[test]
    fn test_render_markdown_with_diagnostics() -> Result<()> {
        let report = Report::new(vec![], vec![]).with_diagnostics(vec![Diagnostic::new(
            DiagnosticCategory::Cache,
            "cache cleared",
        )]);
        let output = report.render_markdown()?;
        assert!(output.contains("cache"));
        assert!(output.contains("cache cleared"));
        Ok(())
    }

    #[test]
    fn test_render_markdown_with_parse_errors() -> Result<()> {
        use crate::syntax::ParseFailure;
        use std::path::PathBuf;

        let report = Report::new(vec![], vec![]).with_parse_failures(vec![ParseFailure {
            file: PathBuf::from("src/broken.ts"),
            message: "Expected a semicolon".to_string(),
            span: None,
        }]);
        let output = report.render_markdown()?;
        assert!(output.contains("Parse Errors"));
        assert!(output.contains("src/broken.ts"));
        assert!(output.contains("Expected a semicolon"));
        Ok(())
    }
}
