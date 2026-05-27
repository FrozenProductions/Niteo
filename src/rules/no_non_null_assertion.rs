use std::path::Path;

use oxc_ast::ast::TSNonNullExpression;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_NON_NULL_ASSERTION_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Avoid non-null assertions. Use proper null checks or optional chaining instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = NonNullAssertionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NonNullAssertionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NonNullAssertionVisitor<'a, 'f> {
    fn visit_ts_non_null_expression(&mut self, expr: &TSNonNullExpression<'a>) {
        let pos = self.line_index.position_for(expr.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_NON_NULL_ASSERTION_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_non_null_expression(self, expr);
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
            Path::new("Component.tsx"),
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
    fn reports_non_null_assertion() {
        let violations = run_check("const value = obj!.prop;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_non_null_assertion_on_function_call() {
        let violations = run_check("const result = getValue()!;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_non_null_assertion_on_array_access() {
        let violations = run_check("const item = array[0]!;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_non_null_assertions() {
        let violations = run_check("const a = x!; const b = y!;\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn allows_optional_chaining() {
        let violations = run_check("const value = obj?.prop;\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_nullish_coalescing() {
        let violations = run_check("const value = obj ?? 'default';\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_guard() {
        let violations = run_check("if (obj) { const value = obj.prop; }\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_null_in_comments() {
        let source = "// const value = obj!.prop;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_null_in_strings() {
        let source = r#"const text = "obj!.prop";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_nested_non_null_assertion() {
        let violations = run_check("const value = obj!.nested!.prop;\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn allows_regular_property_access() {
        let violations = run_check("const value = obj.prop;\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_assertion() {
        let violations = run_check("const value = obj as string;\n");
        assert!(violations.is_empty());
    }
}
