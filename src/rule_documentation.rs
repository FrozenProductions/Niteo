pub(crate) mod catalog;
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
        .map(|documentation| {
            let category = documentation.category.as_str();
            ConfiguredRule {
                name: documentation.name,
                severity: (documentation.summarize)(config).severity,
                category,
            }
        })
        .collect()
}

pub fn explain_rule(rule_name: &str, config: &ProjectConfig) -> Result<RuleExplanation> {
    let Some(documentation) = catalog::find_rule(rule_name) else {
        let names = catalog::available_rule_names();
        bail!("unknown rule '{rule_name}'. Available rules: {names}");
    };

    let summary = (documentation.summarize)(config);

    Ok(RuleExplanation {
        name: documentation.name,
        severity: summary.severity,
        intent: documentation.intent,
        examples: documentation
            .examples
            .iter()
            .map(|example| RuleExplanationExample {
                label: example.label,
                code: example.code,
            })
            .collect(),
        options: documentation
            .options
            .iter()
            .map(|option| RuleExplanationOption {
                name: option.name,
                description: option.description,
            })
            .collect(),
        current_severity: summary.severity,
        current_options: summary.options,
    })
}
