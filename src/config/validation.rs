use crate::report::{BOLD, DIM, GREEN, RED, RESET, YELLOW};
use crate::rules::known_rule_ids;

pub struct ConfigValidationReport {
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl ConfigValidationReport {
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
    pub rule: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigDiagnosticSeverity {
    Error,
    Warn,
}

fn known_option_names_for_rule(rule_name: &str) -> Option<Vec<&'static str>> {
    crate::config::rule_metadata::known_option_names_for_rule(rule_name)
}

pub fn validate_config_source(source: &str) -> ConfigValidationReport {
    let mut diagnostics = Vec::new();

    let value: toml::Value = match toml::from_str(source) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message: format!("failed to parse TOML: {error}"),
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

    if let Some(fix_value) = value.get("fix") {
        if let Some(fix) = fix_value.as_table() {
            validate_fix_table(fix, &mut diagnostics);
        } else {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message: "[fix] must be a table of rule booleans".to_string(),
                rule: None,
            });
        }
    }

    ConfigValidationReport { diagnostics }
}

fn validate_fix_table(fix: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    use crate::config::rule_metadata::rule_by_id;

    let known = known_rule_ids();
    let known_set: std::collections::HashSet<_> = known.iter().copied().collect();

    for (name, value) in fix {
        if !value.is_bool() {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message: format!("[fix] \"{name}\" must be a boolean"),
                rule: None,
            });
            continue;
        }

        if !known_set.contains(name.as_str()) {
            let closest = find_closest(name, &known_set);
            let mut message = format!("unknown rule \"{name}\" in [fix]");
            if let Some(hint) = closest {
                message.push_str(&format!(". Did you mean \"{hint}\"?"));
            }
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Error,
                message,
                rule: None,
            });
            continue;
        }

        let fixable = rule_by_id(name).is_some_and(|meta| meta.fixable);
        if !fixable {
            diagnostics.push(ConfigDiagnostic {
                severity: ConfigDiagnosticSeverity::Warn,
                message: format!(
                    "rule \"{name}\" does not support autofix; [fix] entry has no effect"
                ),
                rule: None,
            });
        }
    }
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
                rule: Some(rule_name.clone()),
            });
        }
    }
}

fn validate_contradictions(rules: &toml::Table, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let metadata = crate::config::rule_metadata::all_rule_metadata();
    for meta in metadata {
        for conflict_id in meta.conflicts {
            let a_enabled = is_rule_enabled(rules, meta.name);
            let b_enabled = is_rule_enabled(rules, conflict_id);
            if a_enabled && b_enabled {
                diagnostics.push(ConfigDiagnostic {
                    severity: ConfigDiagnosticSeverity::Warn,
                    message: format!("\"{id}\" conflicts with \"{conflict_id}\"", id = meta.name),
                    rule: Some(meta.name.to_string()),
                });
            }
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

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];
    for (a_index, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        if let Some(cell) = row.first_mut() {
            *cell = a_index;
        }
    }
    if let Some(first_row) = matrix.first_mut() {
        for (b_index, cell) in first_row.iter_mut().enumerate().take(b_len + 1) {
            *cell = b_index;
        }
    }

    for a_index in 1..=a_len {
        for b_index in 1..=b_len {
            let cost = if a_chars[a_index - 1] == b_chars[b_index - 1] {
                0
            } else {
                1
            };
            let deletion = matrix[a_index - 1][b_index] + 1;
            let insertion = matrix[a_index][b_index - 1] + 1;
            let substitution = matrix[a_index - 1][b_index - 1] + cost;
            matrix[a_index][b_index] = deletion.min(insertion).min(substitution);

            if a_index > 1
                && b_index > 1
                && a_chars[a_index - 1] == b_chars[b_index - 2]
                && a_chars[a_index - 2] == b_chars[b_index - 1]
            {
                matrix[a_index][b_index] =
                    matrix[a_index][b_index].min(matrix[a_index - 2][b_index - 2] + cost);
            }
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    fn has_errors(report: &ConfigValidationReport) -> bool {
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == ConfigDiagnosticSeverity::Error)
    }

    #[test]
    fn valid_default_config_passes() -> Result<()> {
        let source = crate::config::defaults::DEFAULT_CONFIG_SOURCE;
        let report = validate_config_source(source);
        assert!(!has_errors(&report));

        Ok(())
    }

    #[test]
    fn unknown_rule_name_reports_error() -> Result<()> {
        let source = r#"
[rules.non-existent-rule]
severity = "warn"
"#;
        let report = validate_config_source(source);
        assert!(has_errors(&report));

        Ok(())
    }

    #[test]
    fn unknown_option_reports_warning() -> Result<()> {
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

        Ok(())
    }

    #[test]
    fn unknown_severity_reports_error() -> Result<()> {
        let source = r#"
[rules.no-console]
severity = "warning"
"#;
        let report = validate_config_source(source);
        assert!(has_errors(&report));

        Ok(())
    }

    #[test]
    fn conflicting_rules_report_warning() -> Result<()> {
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

        Ok(())
    }

    #[test]
    fn disabled_conflicting_rule_does_not_report() -> Result<()> {
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

        Ok(())
    }

    #[test]
    fn shorthand_severity_validation() -> Result<()> {
        let source = r#"
[rules.no-console]
severity = "warn"
"#;
        let report = validate_config_source(source);
        assert!(!has_errors(&report));

        Ok(())
    }

    #[test]
    fn unknown_severity_via_shorthand() -> Result<()> {
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

        Ok(())
    }

    #[test]
    fn valid_fix_table_passes() -> Result<()> {
        let source = r#"
[fix]
no-debugger = false
no-focused-test = true
"#;
        let report = validate_config_source(source);
        assert!(!has_errors(&report));

        Ok(())
    }

    #[test]
    fn unknown_fix_rule_reports_error() -> Result<()> {
        let source = r#"
[fix]
no-debuggr = false
"#;
        let report = validate_config_source(source);
        let has_unknown_fix_rule = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown rule") && d.message.contains("[fix]"));
        assert!(has_unknown_fix_rule);
        assert!(has_errors(&report));

        Ok(())
    }

    #[test]
    fn non_boolean_fix_value_reports_error() -> Result<()> {
        let source = r#"
[fix]
no-debugger = "false"
"#;
        let report = validate_config_source(source);
        let has_boolean_error = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("must be a boolean"));
        assert!(has_boolean_error);
        assert!(has_errors(&report));

        Ok(())
    }

    #[test]
    fn non_fixable_rule_reports_warning() -> Result<()> {
        let source = r#"
[fix]
no-console = false
"#;
        let report = validate_config_source(source);
        let has_non_fixable_warning = report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("does not support autofix"));
        assert!(has_non_fixable_warning);
        assert!(!has_errors(&report));

        Ok(())
    }
}
