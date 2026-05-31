use std::path::Path;

use oxc_ast::ast::BindingIdentifier;
use oxc_ast_visit::Visit;

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
        .map(|a| a.to_string())
        .collect();
    abbreviations.extend(config.extra_abbreviations.clone());

    let mut visitor = NoAbbreviationsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        abbreviations,
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
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NoAbbreviationsVisitor<'a, 'f> {
    fn visit_binding_identifier(&mut self, ident: &BindingIdentifier<'a>) {
        let name = ident.name.as_str();
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
    fn reports_btn_variable() {
        let source = "const btn = document.querySelector('button');\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("btn"));
        assert!(violations[0]
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("btn")));
    }

    #[test]
    fn reports_ctx_variable() {
        let source = "const ctx = getContext();\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("ctx"));
    }

    #[test]
    fn reports_mgr_variable() {
        let source = "const mgr = new Manager();\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("mgr"));
    }

    #[test]
    fn reports_camelcase_abbreviation() {
        let source = "const btnLabel = 'Click me';\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("btnLabel"));
    }

    #[test]
    fn reports_abbreviation_in_function_name() {
        let source = "function getCtx() { return {}; }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("getCtx"));
    }

    #[test]
    fn reports_abbreviation_in_parameter() {
        let source = "function render(btnElement: HTMLElement) {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("btnElement"));
    }

    #[test]
    fn reports_abbreviation_in_arrow_function() {
        let source = "const handler = (ctxArg: unknown) => {};\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("ctxArg"));
    }

    #[test]
    fn reports_abbreviation_in_class_name() {
        let source = "class BtnFactory {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("BtnFactory"));
    }

    #[test]
    fn allows_normal_identifiers() {
        let source = "const button = document.querySelector('button');\nconst context = getContext();\nconst manager = new Manager();\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_identifiers_without_abbreviations() {
        let source = "const label = 'Click me';\nconst count = 42;\nfunction render() {}\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiple_in_same_line() {
        let source = "const btn = document.querySelector('button'), ctx = getContext();\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn reports_custom_extra_abbreviation() {
        let config = NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec!["req".to_string(), "res".to_string()],
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
    }

    #[test]
    fn ignores_abbreviations_in_strings() {
        let source = r#"const text = "use btn or ctx";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_abbreviations_in_comments() {
        let source = "// const btn = null;\n/* const ctx = null; */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    fn test_config() -> NoAbbreviationsRuleConfig {
        NoAbbreviationsRuleConfig {
            severity: Severity::Warn,
            extra_abbreviations: vec![],
        }
    }
}
