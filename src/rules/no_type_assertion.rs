use std::path::Path;

use oxc_ast::ast::TSAsExpression;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_TYPE_ASSERTION_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Avoid type assertions. Use type narrowing or `satisfies` instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = TypeAssertionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct TypeAssertionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for TypeAssertionVisitor<'a, 'f> {
    fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
        let pos = self.line_index.position_for(expr.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_TYPE_ASSERTION_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_as_expression(self, expr);
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
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("test.ts"), &program, &line_index, &test_config())
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_as_cast() {
        let violations = run_check("const value = something as string;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_as_cast_with_literal() {
        let violations = run_check("const config = { port: 3000 } as Config;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_as_const() {
        let violations = run_check("const value = { x: 1 } as const;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_type_assertions() {
        let violations = run_check("const a = x as string; const b = y as number;\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn reports_nested_type_assertion() {
        let violations = run_check("const value = (obj as any).prop as string;\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn allows_satisfies() {
        let violations = run_check("const config = { port: 3000 } satisfies Config;\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_narrowing() {
        let violations = run_check("if (typeof value === 'string') { const s = value; }\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_annotations() {
        let violations = run_check("const value: string = 'test';\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_as_in_comments() {
        let source = "// const value = x as string;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_as_in_strings() {
        let source = r#"const text = "x as string";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }
}
