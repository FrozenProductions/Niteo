use crate::config::Severity;

pub struct ConfiguredRule {
    pub name: &'static str,
    pub severity: Severity,
}

pub struct RuleExplanation {
    pub name: &'static str,
    pub severity: Severity,
    pub intent: &'static str,
    pub examples: Vec<RuleExplanationExample>,
    pub options: Vec<RuleExplanationOption>,
    pub current_severity: Severity,
    pub current_options: Vec<String>,
}

pub struct RuleExplanationExample {
    pub label: &'static str,
    pub code: &'static str,
}

pub struct RuleExplanationOption {
    pub name: &'static str,
    pub description: &'static str,
}
