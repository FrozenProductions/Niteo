use anyhow::Result;
use serde::Serialize;

use super::model::{ConfiguredRule, RuleExplanation};

#[derive(Serialize)]
struct ConfiguredRuleJson {
    name: &'static str,
    severity: &'static str,
}

#[derive(Serialize)]
struct RuleExplanationJson {
    name: &'static str,
    severity: &'static str,
    intent: &'static str,
    examples: Vec<RuleExampleJson>,
    options: Vec<RuleOptionJson>,
    current_config: CurrentConfigJson,
}

#[derive(Serialize)]
struct RuleExampleJson {
    label: &'static str,
    code: &'static str,
}

#[derive(Serialize)]
struct RuleOptionJson {
    name: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct CurrentConfigJson {
    severity: &'static str,
    options: Vec<String>,
}

pub fn render_rules_json(rules: &[ConfiguredRule]) -> Result<String> {
    let rules_json: Vec<ConfiguredRuleJson> = rules
        .iter()
        .map(|r| ConfiguredRuleJson {
            name: r.name,
            severity: r.severity.as_str(),
        })
        .collect();

    Ok(serde_json::to_string_pretty(&rules_json)?)
}

pub fn render_explanation_json(explanation: &RuleExplanation) -> Result<String> {
    let json = RuleExplanationJson {
        name: explanation.name,
        severity: explanation.severity.as_str(),
        intent: explanation.intent,
        examples: explanation
            .examples
            .iter()
            .map(|e| RuleExampleJson {
                label: e.label,
                code: e.code,
            })
            .collect(),
        options: explanation
            .options
            .iter()
            .map(|o| RuleOptionJson {
                name: o.name,
                description: o.description,
            })
            .collect(),
        current_config: CurrentConfigJson {
            severity: explanation.current_severity.as_str(),
            options: explanation.current_options.clone(),
        },
    };

    Ok(serde_json::to_string_pretty(&json)?)
}
