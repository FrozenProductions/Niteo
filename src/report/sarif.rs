use anyhow::Result;
use serde_json::json;

use crate::diagnostics::{Diagnostic, DiagnosticCategory};
use crate::report::json::summary_json;
use crate::report::model::{Report, path_to_string, sarif_level, severity_label};
use crate::report::summary::group_by_rule;
use crate::rules::Violation;
use crate::syntax::ParseFailure;

impl Report {
    pub fn render_sarif(&self) -> Result<String> {
        let rule_groups = group_by_rule(&self.violations);
        let rules = rule_groups
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
            .collect::<Vec<serde_json::Value>>();

        let results = self
            .violations
            .iter()
            .map(sarif_result_json)
            .collect::<Vec<serde_json::Value>>();

        let mut notifications: Vec<serde_json::Value> = self
            .diagnostics
            .iter()
            .map(sarif_notification_json)
            .collect();
        notifications.extend(self.parse_failures.iter().map(sarif_parse_failure_json));

        let invocation = if notifications.is_empty() {
            json!({
                "executionSuccessful": true,
            })
        } else {
            json!({
                "executionSuccessful": self.parse_failures.is_empty(),
                "toolExecutionNotifications": notifications,
            })
        };

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
                    "invocations": [invocation],
                    "properties": {
                        "summary": summary_json(self),
                    },
                },
            ],
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }
}

fn sarif_notification_json(diagnostic: &Diagnostic) -> serde_json::Value {
    json!({
        "level": "warning",
        "message": {
            "text": diagnostic.message,
        },
        "descriptor": {
            "id": diagnostic.category.as_str(),
        },
    })
}

fn sarif_parse_failure_json(failure: &ParseFailure) -> serde_json::Value {
    json!({
        "level": "error",
        "message": {
            "text": format!("{}: {}", path_to_string(&failure.file), failure.message),
        },
        "descriptor": {
            "id": DiagnosticCategory::Parse.as_str(),
        },
    })
}

fn sarif_result_json(violation: &Violation) -> serde_json::Value {
    let mut message = violation.message.to_string();
    if let Some(ref detail) = violation.detail {
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

#[cfg(test)]
mod tests {
    use crate::diagnostics::{Diagnostic, DiagnosticCategory};
    use crate::report::model::Report;
    use anyhow::{Context, Result};
    use serde_json::Value;

    #[test]
    fn sarif_renders_tool_execution_notifications() -> Result<()> {
        let report = Report::new(vec![], vec![]).with_diagnostics(vec![Diagnostic::new(
            DiagnosticCategory::Git,
            "could not detect changed files",
        )]);

        let rendered = report.render_sarif()?;
        let parsed: Value = serde_json::from_str(&rendered)?;
        let notifications = parsed["runs"][0]["invocations"][0]["toolExecutionNotifications"]
            .as_array()
            .context("expected toolExecutionNotifications array")?;

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0]["level"], "warning");
        assert_eq!(
            notifications[0]["message"]["text"],
            "could not detect changed files"
        );
        assert_eq!(notifications[0]["descriptor"]["id"], "git");
        assert_eq!(
            parsed["runs"][0]["invocations"][0]["executionSuccessful"],
            true
        );
        Ok(())
    }

    #[test]
    fn sarif_renders_parse_failure_notification_and_fails_execution() -> Result<()> {
        use crate::syntax::ParseFailure;
        use std::path::PathBuf;

        let report = Report::new(vec![], vec![]).with_parse_failures(vec![ParseFailure {
            file: PathBuf::from("src/broken.ts"),
            message: "Expected a semicolon".to_string(),
            span: None,
        }]);

        let rendered = report.render_sarif()?;
        let parsed: Value = serde_json::from_str(&rendered)?;
        let notifications = parsed["runs"][0]["invocations"][0]["toolExecutionNotifications"]
            .as_array()
            .context("expected toolExecutionNotifications array")?;

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0]["level"], "error");
        assert_eq!(notifications[0]["descriptor"]["id"], "parse");
        assert!(
            notifications[0]["message"]["text"]
                .as_str()
                .context("expected message text")?
                .contains("src/broken.ts")
        );
        assert_eq!(
            parsed["runs"][0]["invocations"][0]["executionSuccessful"],
            false
        );
        Ok(())
    }
}
