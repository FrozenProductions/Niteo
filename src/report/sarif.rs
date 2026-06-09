use anyhow::Result;
use serde_json::json;

use crate::report::json::summary_json;
use crate::report::model::{Report, path_to_string, sarif_level, severity_label};
use crate::report::summary::group_by_rule;
use crate::rules::Violation;

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
                        "summary": summary_json(self),
                    },
                },
            ],
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }
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
