use std::collections::HashMap;
use std::path::PathBuf;

use crate::cache::store::CachedViolation;
use crate::config::Severity;
use crate::rules::{RuleId, Violation};

pub fn violation_to_cached(violation: &Violation) -> CachedViolation {
    CachedViolation {
        line: violation.line,
        column: violation.column,
        rule: violation.rule.to_string(),
        message: violation.message.to_string(),
        severity: violation.severity.as_str().to_string(),
        detail: violation.detail.clone(),
        subject: violation.subject.clone(),
    }
}

pub fn cached_violations_to_violations(
    cached: &[CachedViolation],
    file: PathBuf,
    rule_lookup: &HashMap<String, RuleId>,
    message_interner: &mut StringInterner,
) -> Vec<Violation> {
    cached
        .iter()
        .map(|cached_violation| {
            cached_violation_to_violation(
                cached_violation,
                file.clone(),
                rule_lookup,
                message_interner,
            )
        })
        .collect()
}

pub fn build_rule_lookup() -> HashMap<String, RuleId> {
    crate::rules::known_rule_ids()
        .iter()
        .map(|rule_id| (rule_id.to_string(), *rule_id))
        .collect()
}

fn cached_violation_to_violation(
    cached: &CachedViolation,
    file: PathBuf,
    rule_lookup: &HashMap<String, RuleId>,
    message_interner: &mut StringInterner,
) -> Violation {
    let rule = rule_lookup
        .get(&cached.rule)
        .copied()
        .unwrap_or_else(|| message_interner.intern(&cached.rule));
    let message = message_interner.intern(&cached.message);
    let severity = cached
        .severity
        .parse::<Severity>()
        .unwrap_or(Severity::Warn);

    Violation {
        file,
        span: None,
        line: cached.line,
        column: cached.column,
        rule,
        message,
        severity,
        detail: cached.detail.clone(),
        subject: cached.subject.clone(),
    }
}

/// Deduplicates serialized strings by leaking them as `&'static str` values.
///
/// This is used only when restoring violations from the cache. The number of
/// unique strings is bounded by the rule/message space of a single project,
/// and the leaked memory lives for the remainder of the CLI process.
#[derive(Default)]
pub struct StringInterner {
    strings: HashMap<&'static str, &'static str>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, value: &str) -> &'static str {
        if let Some(existing) = self.strings.get(value) {
            return existing;
        }
        let leaked = leak_string(value);
        self.strings.insert(leaked, leaked);
        leaked
    }
}

fn leak_string(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::cache::store::CachedViolation;
    use crate::cache::violations::{
        StringInterner, build_rule_lookup, cached_violations_to_violations, violation_to_cached,
    };
    use crate::config::Severity;
    use crate::rules::{NO_CONSOLE_RULE_ID, Violation};

    #[test]
    fn violation_to_cached_roundtrip() {
        let violation = Violation {
            file: PathBuf::from("src/a.ts"),
            span: None,
            line: Some(1),
            column: Some(2),
            rule: NO_CONSOLE_RULE_ID,
            message: "Disallow console statements.",
            severity: Severity::Error,
            detail: Some("detail".to_string()),
            subject: Some("subject".to_string()),
        };

        let cached = violation_to_cached(&violation);

        assert_eq!(cached.line, Some(1));
        assert_eq!(cached.column, Some(2));
        assert_eq!(cached.rule, "no-console");
        assert_eq!(cached.message, "Disallow console statements.");
        assert_eq!(cached.severity, "error");
        assert_eq!(cached.detail, Some("detail".to_string()));
        assert_eq!(cached.subject, Some("subject".to_string()));
    }

    #[test]
    fn cached_violation_to_violation_uses_static_rule_id() {
        let cached = CachedViolation {
            line: Some(1),
            column: Some(2),
            rule: "no-console".to_string(),
            message: "Disallow console statements.".to_string(),
            severity: "warn".to_string(),
            detail: None,
            subject: None,
        };
        let rule_lookup = build_rule_lookup();
        let mut message_interner = StringInterner::new();

        let violation = cached_violations_to_violations(
            &[cached],
            PathBuf::from("src/a.ts"),
            &rule_lookup,
            &mut message_interner,
        )
        .pop()
        .unwrap();

        assert_eq!(violation.rule, NO_CONSOLE_RULE_ID);
        assert_eq!(violation.message, "Disallow console statements.");
        assert_eq!(violation.severity, Severity::Warn);
    }

    #[test]
    fn cached_violation_with_unknown_rule_falls_back_to_interned_string() {
        let cached = CachedViolation {
            line: None,
            column: None,
            rule: "unknown-rule".to_string(),
            message: "message".to_string(),
            severity: "info".to_string(),
            detail: None,
            subject: None,
        };
        let rule_lookup = HashMap::new();
        let mut message_interner = StringInterner::new();

        let violation = cached_violations_to_violations(
            &[cached],
            PathBuf::from("src/a.ts"),
            &rule_lookup,
            &mut message_interner,
        )
        .pop()
        .unwrap();

        assert_eq!(violation.rule, "unknown-rule");
        assert_eq!(violation.message, "message");
        assert_eq!(violation.severity, Severity::Info);
    }

    #[test]
    fn string_interner_deduplicates() {
        let mut interner = StringInterner::new();
        let first = interner.intern("hello");
        let second = interner.intern("hello");

        assert_eq!(first, second);
    }
}
