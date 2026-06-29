use std::collections::HashMap;

use crate::config::Severity;
use crate::config::rule_metadata::{RuleCategory, rule_by_id};
use crate::rules::RuleId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FailureThreshold {
    Error,
    Warn,
    #[default]
    Any,
}

impl FailureThreshold {
    pub fn includes(self, severity: Severity) -> bool {
        match self {
            FailureThreshold::Error => severity == Severity::Error,
            FailureThreshold::Warn => matches!(severity, Severity::Warn | Severity::Error),
            FailureThreshold::Any => severity.is_enabled(),
        }
    }
}

impl std::str::FromStr for FailureThreshold {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "any" => Ok(Self::Any),
            _ => Err(format!(
                "invalid failure threshold '{value}'; must be one of: error, warn, any"
            )),
        }
    }
}

impl FailureThreshold {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FailurePolicy {
    pub default: FailureThreshold,
    pub rules: HashMap<String, FailureThreshold>,
    pub categories: HashMap<RuleCategory, FailureThreshold>,
}

impl FailurePolicy {
    pub fn threshold_for(&self, rule_id: RuleId) -> FailureThreshold {
        if let Some(threshold) = self.rules.get(rule_id) {
            return *threshold;
        }

        if let Some(metadata) = rule_by_id(rule_id)
            && let Some(threshold) = self.categories.get(&metadata.category)
        {
            return *threshold;
        }

        self.default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_includes_expected_severities() {
        assert!(FailureThreshold::Error.includes(Severity::Error));
        assert!(!FailureThreshold::Error.includes(Severity::Warn));
        assert!(!FailureThreshold::Error.includes(Severity::Info));

        assert!(FailureThreshold::Warn.includes(Severity::Error));
        assert!(FailureThreshold::Warn.includes(Severity::Warn));
        assert!(!FailureThreshold::Warn.includes(Severity::Info));

        assert!(FailureThreshold::Any.includes(Severity::Error));
        assert!(FailureThreshold::Any.includes(Severity::Warn));
        assert!(FailureThreshold::Any.includes(Severity::Info));
        assert!(!FailureThreshold::Any.includes(Severity::Off));
    }

    #[test]
    fn policy_falls_back_to_default() {
        let policy = FailurePolicy {
            default: FailureThreshold::Warn,
            rules: HashMap::new(),
            categories: HashMap::new(),
        };

        assert_eq!(policy.threshold_for("no-console"), FailureThreshold::Warn);
    }

    #[test]
    fn policy_prefers_rule_override() {
        let mut rules = HashMap::new();
        rules.insert("no-console".to_string(), FailureThreshold::Error);

        let policy = FailurePolicy {
            default: FailureThreshold::Any,
            rules,
            categories: HashMap::new(),
        };

        assert_eq!(policy.threshold_for("no-console"), FailureThreshold::Error);
    }

    #[test]
    fn rule_override_beats_category_override() {
        let mut categories = HashMap::new();
        categories.insert(RuleCategory::SourceHygiene, FailureThreshold::Warn);

        let mut rules = HashMap::new();
        rules.insert("no-console".to_string(), FailureThreshold::Error);

        let policy = FailurePolicy {
            default: FailureThreshold::Any,
            rules,
            categories,
        };

        assert_eq!(policy.threshold_for("no-console"), FailureThreshold::Error);
    }

    #[test]
    fn category_override_used_when_no_rule_override() {
        let mut categories = HashMap::new();
        categories.insert(RuleCategory::SourceHygiene, FailureThreshold::Error);

        let policy = FailurePolicy {
            default: FailureThreshold::Any,
            rules: HashMap::new(),
            categories,
        };

        assert_eq!(policy.threshold_for("no-console"), FailureThreshold::Error);
    }
}
