use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_SKIPPED_TEST_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Disallow skipped tests (describe.skip, it.skip, test.skip).";

const SKIPPED_TEST_NAMES: &[&str] = &["describe", "it", "test"];

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = SkippedTestVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct SkippedTestVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for SkippedTestVisitor<'a, 'f> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = &call.callee
            && let Expression::Identifier(object) = &member.object
            && SKIPPED_TEST_NAMES.contains(&object.name.as_str())
            && member.property.name == "skip"
        {
            let pos = self.line_index.position_for(member.span);
            let subject = format!("{}.skip", object.name);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_SKIPPED_TEST_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: Some(subject),
            });
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RuleConfig, Severity};
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
            Path::new("auth.test.ts"),
            &program,
            &line_index,
            &test_config(),
        )
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_describe_skip() {
        let violations = run_check("describe.skip('suite', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("describe.skip"));
    }

    #[test]
    fn reports_it_skip() {
        let violations = run_check("it.skip('works', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("it.skip"));
    }

    #[test]
    fn reports_test_skip() {
        let violations = run_check("test.skip('works', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("test.skip"));
    }

    #[test]
    fn reports_multiple_skipped_tests() {
        let source = "describe.skip('suite', () => { it.skip('works', () => {}); });";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn ignores_regular_describe() {
        let violations = run_check("describe('suite', () => {});");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_regular_it() {
        let violations = run_check("it('works', () => {});");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_regular_test() {
        let violations = run_check("test('works', () => {});");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_describe_only() {
        let violations = run_check("describe.only('suite', () => {});");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_it_only() {
        let violations = run_check("it.only('works', () => {});");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_other_member_calls() {
        let violations = run_check("foo.skip('something');");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_in_comments() {
        let source = "// describe.skip('suite', () => {});";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_in_strings() {
        let source = r#"const text = "it.skip('works', () => {})";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_correct_line() {
        let source = "const x = 1;\nit.skip('works', () => {});\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }
}
