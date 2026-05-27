mod catalog;
mod model;
mod render_json;
mod render_text;
mod summary;

pub use model::{ConfiguredRule, RuleExplanation, RuleExplanationExample, RuleExplanationOption};
pub use render_json::{render_explanation_json, render_rules_json};
pub use render_text::render_explanation_text;

use anyhow::{Result, bail};

use crate::config::ProjectConfig;

pub fn configured_rules(config: &ProjectConfig) -> Vec<ConfiguredRule> {
    catalog::all_rules()
        .iter()
        .map(|documentation| ConfiguredRule {
            name: documentation.name,
            severity: summary::summarize_rule(config, documentation.kind).severity,
        })
        .collect()
}

pub fn explain_rule(rule_name: &str, config: &ProjectConfig) -> Result<RuleExplanation> {
    let Some(documentation) = catalog::find_rule(rule_name) else {
        let names = catalog::available_rule_names();
        bail!("unknown rule '{rule_name}'. Available rules: {names}");
    };

    let summary = summary::summarize_rule(config, documentation.kind);

    Ok(RuleExplanation {
        name: documentation.name,
        severity: summary.severity,
        intent: documentation.intent,
        examples: documentation
            .examples
            .iter()
            .map(|e| RuleExplanationExample {
                label: e.label,
                code: e.code,
            })
            .collect(),
        options: documentation
            .options
            .iter()
            .map(|o| RuleExplanationOption {
                name: o.name,
                description: o.description,
            })
            .collect(),
        current_severity: summary.severity,
        current_options: summary.options,
    })
}
