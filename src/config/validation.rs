use std::path::PathBuf;

use crate::report::{BOLD, DIM, GREEN, RED, RESET, YELLOW};
use crate::rules::known_rule_ids;

pub struct ConfigValidationReport {
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl ConfigValidationReport {
    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == ConfigDiagnosticSeverity::Error)
    }

    pub fn render_text(&self) -> String {
        let mut output = String::new();

        if self.diagnostics.is_empty() {
            output.push_str(&format!(
                "{GREEN}{BOLD}No configuration issues found.{RESET}\n"
            ));
        } else {
            let error_count = self
                .diagnostics
                .iter()
                .filter(|d| d.severity == ConfigDiagnosticSeverity::Error)
                .count();
            let warn_count = self
                .diagnostics
                .iter()
                .filter(|d| d.severity == ConfigDiagnosticSeverity::Warn)
                .count();

            for diagnostic in &self.diagnostics {
                let (color, prefix) = match diagnostic.severity {
                    ConfigDiagnosticSeverity::Error => (RED, "error"),
                    ConfigDiagnosticSeverity::Warn => (YELLOW, "warn"),
                };
                let rule = diagnostic
                    .rule
                    .as_deref()
                    .map(|r| format!("rules.{r}"))
                    .unwrap_or_default();
                output.push_str(&format!(
                    "  {color}{prefix}{RESET}   {rule:<36} {}\n",
                    diagnostic.message,
                ));
            }

            let error_label = if error_count == 1 { "error" } else { "errors" };
            let warn_label = if warn_count == 1 {
                "warning"
            } else {
                "warnings"
            };
            let color = if error_count > 0 { RED } else { DIM };
            output.push('\n');
            output.push_str(&format!(
                "  {BOLD}{color}{error_count} {error_label}{RESET}{BOLD} · {warn_count} {warn_label}{RESET}\n",
            ));
        }

        output
    }
}

pub struct ConfigDiagnostic {
    pub severity: ConfigDiagnosticSeverity,
    pub message: String,
    #[allow(dead_code)]
    pub path: Option<PathBuf>,
    pub rule: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigDiagnosticSeverity {
    Error,
    Warn,
}

fn known_option_names_for_rule(rule_name: &str) -> Option<&'static [&'static str]> {
    match rule_name {
        "boolean-prefix" => Some(&["severity", "prefixes", "ignore-constants"]),
        "component-file-only-components" => Some(&["severity"]),
        "directory-must-have-barrel" => Some(&["severity"]),
        "entry-file-no-logic" => Some(&["severity", "entry-files"]),
        "explicit-return-type" => Some(&["severity"]),
        "hook-no-jsx" => Some(&["severity"]),
        "hook-prefix" => Some(&["severity", "prefixes"]),
        "layer-boundaries" => Some(&["severity"]),
        "max-directory-depth" => Some(&["severity", "max-depth", "ignore-dirs"]),
        "max-file-exports" => Some(&["severity", "max-exports"]),
        "max-function-params" => Some(&["severity", "max-params"]),
        "max-items-per-directory" => {
            Some(&["severity", "max-items", "ignore-dirs", "count-folders"])
        }
        "min-items-per-directory" => {
            Some(&["severity", "min-items", "ignore-dirs", "count-folders"])
        }
        "no-abbreviations" => Some(&["severity", "extra-abbreviations"]),
        "no-anemic-domain" => Some(&["severity", "max-files", "ignore-dirs"]),
        "no-any" => Some(&["severity", "allowed-folders"]),
        "no-barrel-chain" => Some(&["severity"]),
        "no-barrel-files" => Some(&["severity"]),
        "no-circular-import" => Some(&["severity"]),
        "no-comments" => Some(&["severity", "allow-doc-comments"]),
        "no-console" => Some(&["severity", "allow-patterns"]),
        "no-debugger" => Some(&["severity"]),
        "no-default-export" => Some(&["severity", "components-only"]),
        "no-dump-files" => Some(&["severity", "extra-names"]),
        "no-duplicate-file-names" => Some(&["severity", "ignore-names"]),
        "no-empty-directories" => Some(&["severity", "ignore-dirs"]),
        "no-empty-domain" => Some(&["severity", "ignore-dirs"]),
        "no-empty-interface" => Some(&["severity"]),
        "no-enums" => Some(&["severity"]),
        "no-eval" => Some(&["severity"]),
        "no-export-star" => Some(&["severity"]),
        "no-focused-test" => Some(&["severity"]),
        "no-god-domain" => Some(&["severity", "max-files", "ignore-dirs"]),
        "no-inline-types" => Some(&["severity"]),
        "no-interface" => Some(&["severity", "allow-declaration-merging"]),
        "no-large-file" => Some(&["severity", "max-lines"]),
        "no-logic-in-barrel" => Some(&["severity"]),
        "no-logic-in-domain" => Some(&["severity"]),
        "no-magic-numbers" => Some(&["severity", "allowed-numbers"]),
        "no-mutable-exports" => Some(&["severity"]),
        "no-namespace" => Some(&["severity"]),
        "no-nested-functions" => Some(&["severity", "max-depth"]),
        "no-non-null-assertion" => Some(&["severity"]),
        "no-orphan-files" => Some(&["severity", "entry-files"]),
        "no-process-env" => Some(&["severity"]),
        "no-restricted-imports" => Some(&["severity", "restricted"]),
        "no-silent-catch" => Some(&["severity"]),
        "no-skipped-test" => Some(&["severity"]),
        "no-test-code-in-production" => Some(&["severity"]),
        "no-test-import" => Some(&["severity"]),
        "no-then-chain" => Some(&["severity"]),
        "no-type-assertion" => Some(&["severity"]),
        "no-upward-import" => Some(&["severity", "max-depth"]),
        "prefer-readonly" => Some(&["severity"]),
        "prefer-satisfies" => Some(&["severity"]),
        _ => None,
    }
}

pub fn validate_config_source(source: &str) -> ConfigValidationReport {
    let mut diagnostics = Vec::new();

    let value: toml::Value = match toml::from_str(source) {
        Ok(v) => v,
        Err(e) => {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message: format!("failed to parse TOML: {e}"),
                path: None,
                rule: None,
            });
            return ConfigValidationReport { diagnostics };
        }
    };

    let rules_table = value.get("rules").and_then(|r| r.as_table());

    if let Some(rules) = rules_table {
        validate_rule_names(rules, &mut diagnostics);
        validate_rule_options(rules, &mut diagnostics);
        validate_rule_severities(rules, &mut diagnostics);
        validate_contradictions(rules, &mut diagnostics);
    }

    ConfigValidationReport { diagnostics }
}

fn validate_rule_names(rules: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let known = known_rule_ids();
    let known_set: std::collections::HashSet<_> = known.iter().copied().collect();

    for name in rules.keys() {
        if !known_set.contains(name.as_str()) {
            let closest = find_closest(name, &known_set);
            let mut message = format!("unknown rule \"{name}\"");
            if let Some(hint) = closest {
                message.push_str(&format!(". Did you mean \"{hint}\"?"));
            }
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message,
                path: None,
                rule: Some(name.clone()),
            });
        }
    }
}

fn validate_rule_options(rules: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    for (rule_name, rule_value) in rules {
        let rule_table = match rule_value {
            toml::Value::Table(t) => t,
            _ => continue,
        };

        if let Some(known_options) = known_option_names_for_rule(rule_name) {
            let known_set: std::collections::HashSet<_> = known_options.iter().copied().collect();
            for key in rule_table.keys() {
                if !known_set.contains(key.as_str()) {
                    diagnostics.push(ConfigDiagnostic {
                        severity: ConfigDiagnosticSeverity::Warn,
                        message: format!("unknown option \"{key}\" for rule \"{rule_name}\""),
                        path: None,
                        rule: Some(rule_name.clone()),
                    });
                }
            }
        }
    }
}

fn validate_rule_severities(rules: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let valid_severities: [&str; 4] = ["off", "info", "warn", "error"];

    for (rule_name, rule_value) in rules {
        let severity = match rule_value {
            toml::Value::String(s) => Some(s.as_str()),
            toml::Value::Table(t) => t.get("severity").and_then(|s| s.as_str()),
            _ => None,
        };

        if let Some(sev) = severity
            && !valid_severities.contains(&sev)
        {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message: format!(
                    "unknown severity \"{sev}\" for rule \"{rule_name}\"; use \"off\", \"info\", \"warn\", or \"error\""
                ),
                path: None,
                rule: Some(rule_name.clone()),
            });
        }
    }
}

fn validate_contradictions(rules: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let conflicts: &[(&str, &str, &str)] = &[(
        "directory-must-have-barrel",
        "no-barrel-files",
        "one requires barrels while the other rejects them",
    )];

    for (rule_a, rule_b, reason) in conflicts {
        let a_enabled = is_rule_enabled(rules, rule_a);
        let b_enabled = is_rule_enabled(rules, rule_b);

        if a_enabled && b_enabled {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Warn,
                message: format!("\"{rule_a}\" conflicts with \"{rule_b}\": {reason}"),
                path: None,
                rule: Some(rule_a.to_string()),
            });
        }
    }
}

fn is_rule_enabled(rules: &toml::Table, rule_name: &str) -> bool {
    match rules.get(rule_name) {
        None => {
            let known = known_rule_ids();
            known.contains(&rule_name)
        }
        Some(toml::Value::String(s)) => s != "off",
        Some(toml::Value::Table(t)) => match t.get("severity") {
            Some(toml::Value::String(s)) => s != "off",
            None => {
                let known = known_rule_ids();
                known.contains(&rule_name)
            }
            _ => true,
        },
        _ => true,
    }
}

fn find_closest<'a>(name: &str, known: &'a std::collections::HashSet<&str>) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for candidate in known {
        let distance = damerau_levenshtein(name, candidate);
        if distance <= 3 {
            match best {
                Some((_, best_dist)) if distance < best_dist => {
                    best = Some((candidate, distance));
                }
                None => {
                    best = Some((candidate, distance));
                }
                _ => {}
            }
        }
    }
    best.map(|(name, _)| name)
}

fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut d = vec![vec![0usize; b_len + 1]; a_len + 1];
    for (i, row) in d.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, item) in d[0].iter_mut().enumerate().take(b_len + 1) {
        *item = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            let deletion = d[i - 1][j] + 1;
            let insertion = d[i][j - 1] + 1;
            let substitution = d[i - 1][j - 1] + cost;
            d[i][j] = deletion.min(insertion).min(substitution);

            if i > 1
                && j > 1
                && a_chars[i - 1] == b_chars[j - 2]
                && a_chars[i - 2] == b_chars[j - 1]
            {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + cost);
            }
        }
    }

    d[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_default_config_passes() {
        let source = crate::config::defaults::DEFAULT_CONFIG_SOURCE;
        let report = validate_config_source(source);
        assert!(!report.has_errors());
    }

    #[test]
    fn unknown_rule_name_reports_error() {
        let source = r#"
[rules.non-existent-rule]
severity = "warn"
"#;
        let report = validate_config_source(source);
        assert!(report.has_errors());
    }

    #[test]
    fn unknown_option_reports_warning() {
        let source = r#"
[rules.no-console]
severity = "warn"
bogus-option = true
"#;
        let report = validate_config_source(source);
        let has_option_warning = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown option"));
        assert!(has_option_warning);
    }

    #[test]
    fn unknown_severity_reports_error() {
        let source = r#"
[rules.no-console]
severity = "warning"
"#;
        let report = validate_config_source(source);
        assert!(report.has_errors());
    }

    #[test]
    fn conflicting_rules_report_warning() {
        let source = r#"
[rules.directory-must-have-barrel]
severity = "warn"

[rules.no-barrel-files]
severity = "warn"
"#;
        let report = validate_config_source(source);
        let has_conflict = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("conflicts with"));
        assert!(has_conflict);
    }

    #[test]
    fn disabled_conflicting_rule_does_not_report() {
        let source = r#"
[rules.directory-must-have-barrel]
severity = "off"

[rules.no-barrel-files]
severity = "warn"
"#;
        let report = validate_config_source(source);
        let has_conflict = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("conflicts with"));
        assert!(!has_conflict);
    }

    #[test]
    fn shorthand_severity_validation() {
        let source = r#"
[rules.no-console]
severity = "warn"
"#;
        let report = validate_config_source(source);
        assert!(!report.has_errors());
    }

    #[test]
    fn unknown_severity_via_shorthand() {
        let source = r#"
[rules]
no-console = "warnign"
"#;
        let report = validate_config_source(source);
        let has_severity_error = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown severity") && d.message.contains("warnign"));
        assert!(has_severity_error);
    }
}
