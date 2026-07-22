use std::path::Path;

use oxc_ast::ast::{ArrayPattern, BindingIdentifier, ObjectPattern};
use oxc_ast_visit::Visit;
use regex::Regex;

use crate::config::NoAbbreviationsRuleConfig;
use crate::rules::{NO_ABBREVIATIONS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Avoid abbreviations in identifiers.";

const DEFAULT_ABBREVIATIONS: &[&str] = &["btn", "ctx", "mgr"];

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoAbbreviationsRuleConfig,
) -> Vec<Violation> {
    let mut abbreviations: Vec<String> = DEFAULT_ABBREVIATIONS
        .iter()
        .map(|abbreviation| abbreviation.to_string())
        .filter(|a| !config.allow_abbreviations.contains(a))
        .collect();
    abbreviations.extend(config.extra_abbreviations.clone());

    let mut regex_patterns: Vec<Regex> = Vec::new();
    for pattern in &config.abbreviation_patterns {
        match Regex::new(&format!("(?i){}", pattern)) {
            Ok(regex) => regex_patterns.push(regex),
            Err(error) => {
                eprintln!(
                    "warning: invalid abbreviation-pattern '{pattern}' in no-abbreviations \
                     rule: {error}"
                );
            }
        }
    }

    let mut visitor = NoAbbreviationsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        abbreviations,
        regex_patterns,
        ignore_properties: config.ignore_properties,
        ignore_destructured: config.ignore_destructured,
        object_pattern_depth: 0,
        destructured_depth: 0,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NoAbbreviationsVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    abbreviations: Vec<String>,
    regex_patterns: Vec<Regex>,
    ignore_properties: bool,
    ignore_destructured: bool,
    object_pattern_depth: u32,
    destructured_depth: u32,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NoAbbreviationsVisitor<'a, 'f> {
    fn visit_object_pattern(&mut self, pattern: &ObjectPattern<'a>) {
        self.object_pattern_depth += 1;
        self.destructured_depth += 1;
        oxc_ast_visit::walk::walk_object_pattern(self, pattern);
        self.destructured_depth -= 1;
        self.object_pattern_depth -= 1;
    }

    fn visit_array_pattern(&mut self, pattern: &ArrayPattern<'a>) {
        self.destructured_depth += 1;
        oxc_ast_visit::walk::walk_array_pattern(self, pattern);
        self.destructured_depth -= 1;
    }

    fn visit_binding_identifier(&mut self, ident: &BindingIdentifier<'a>) {
        let name = ident.name.as_str();

        if self.ignore_destructured && self.destructured_depth > 0 {
            return;
        }

        if self.ignore_properties && self.object_pattern_depth > 0 {
            return;
        }

        let lower = name.to_lowercase();

        for abbr in &self.abbreviations {
            if lower.contains(abbr.as_str()) {
                let pos = self.line_index.position_for(ident.span);
                let detail = Some(format!(
                    "'{}' contains the abbreviation '{}'. Spell it out instead.",
                    name, abbr
                ));
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    span: Some(ident.span),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: NO_ABBREVIATIONS_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail,
                    subject: Some(name.to_string()),
                });
                return;
            }
        }

        for regex in &self.regex_patterns {
            if regex.is_match(name) {
                let pattern_str = regex.as_str();
                let clean = pattern_str.strip_prefix("(?i)").unwrap_or(pattern_str);
                let pos = self.line_index.position_for(ident.span);
                let detail = Some(format!(
                    "'{}' matches the abbreviation pattern '{}'. Spell it out instead.",
                    name, clean
                ));
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    span: Some(ident.span),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: NO_ABBREVIATIONS_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail,
                    subject: Some(name.to_string()),
                });
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NoAbbreviationsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("Component.tsx"),
            &program,
            &line_index,
            &test_config(),
        )
    }

    #[test]
    fn reports_default_abbreviations() -> Result<()> {
        for (source, expected_subject) in [
            ("const btn = document.querySelector('button');\n", "btn"),
            ("const ctx = getContext();\n", "ctx"),
            ("const mgr = new Manager();\n", "mgr"),
        ] {
            let violations = run_check(source);
            assert_eq!(violations.len(), 1, "expected 1 violation for: {source:?}");
            assert_eq!(
                violations[0].subject.as_deref(),
                Some(expected_subject),
                "wrong subject for: {source:?}",
            );
            assert!(
                violations[0]
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains(expected_subject)),
                "detail missing abbreviation for: {source:?}",
            );
        }

        Ok(())
    }

    #[test]
    fn reports_camelcase_abbreviation() -> Result<()> {
        let source = "const btnLabel = 'Click me';\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("btnLabel"));

        Ok(())
    }

    #[test]
    fn reports_abbreviation_in_function_name() -> Result<()> {
        let source = "function getCtx() { return {}; }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("getCtx"));

        Ok(())
    }

    #[test]
    fn reports_abbreviation_in_parameter() -> Result<()> {
        let source = "function render(btnElement: HTMLElement) {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("btnElement"));

        Ok(())
    }

    #[test]
    fn reports_abbreviation_in_arrow_function() -> Result<()> {
        let source = "const handler = (ctxArg: unknown) => {};\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("ctxArg"));

        Ok(())
    }

    #[test]
    fn reports_abbreviation_in_class_name() -> Result<()> {
        let source = "class BtnFactory {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("BtnFactory"));

        Ok(())
    }

    #[test]
    fn allows_normal_identifiers() -> Result<()> {
        let source = "const button = document.querySelector('button');\nconst context = getContext();\nconst manager = new Manager();\n";
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_identifiers_without_abbreviations() -> Result<()> {
        let source = "const label = 'Click me';\nconst count = 42;\nfunction render() {}\n";
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_multiple_in_same_line() -> Result<()> {
        let source = "const btn = document.querySelector('button'), ctx = getContext();\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);

        Ok(())
    }

    #[test]
    fn reports_custom_extra_abbreviation() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec!["req".to_string(), "res".to_string()],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: false,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const req = {};\nconst res = {};\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 2);

        Ok(())
    }

    #[test]
    fn allows_abbreviations_in_allow_list() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec!["btn".to_string()],
            abbreviation_patterns: vec![],
            ignore_properties: false,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const btn = document.querySelector('button');\nconst ctx = getContext();\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("ctx"));

        Ok(())
    }

    #[test]
    fn ignores_abbreviations_in_strings() -> Result<()> {
        let source = r#"const text = "use btn or ctx";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_abbreviations_in_comments() -> Result<()> {
        let source = "// const btn = null;\n/* const ctx = null; */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_properties_when_ignore_properties_set() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: true,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const { btn, ctxLabel } = obj;\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn still_reports_non_property_destructured_when_only_ignore_properties() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: true,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const [ctx] = arr;\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignores_all_destructured_when_ignore_destructured_set() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: false,
            ignore_destructured: true,
        };
        let allocator = Allocator::default();
        let source = "const { btn } = obj;\nconst [ctx] = arr;\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_nested_destructured_when_ignore_destructured() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: false,
            ignore_destructured: true,
        };
        let allocator = Allocator::default();
        let source = "const { data: { btn } } = obj;\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_via_regex_pattern() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec!["\\b[a-z]{1,2}\\b".to_string()],
            ignore_properties: false,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const xy = 42;\nconst okay = 'fine';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("xy"));

        Ok(())
    }

    #[test]
    fn regex_pattern_does_not_match_full_words() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec!["\\b[a-z]{1,2}\\b".to_string()],
            ignore_properties: false,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const button = 'Click me';\nconst context = getContext();\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn regex_and_substring_both_report() -> Result<()> {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec!["^xy$".to_string()],
            ignore_properties: false,
            ignore_destructured: false,
        };
        let allocator = Allocator::default();
        let source = "const xy = 42;\nconst btn = document.querySelector('button');\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("Component.tsx"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 2);

        Ok(())
    }

    fn test_config() -> NoAbbreviationsRuleConfig {
        NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
            allow_abbreviations: vec![],
            abbreviation_patterns: vec![],
            ignore_properties: false,
            ignore_destructured: false,
        }
    }
}
