use std::path::Path;

use oxc_ast::ast::{Expression, TSAsExpression, TSType};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_UNNECESSARY_TYPE_ASSERTION_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Unnecessary type assertion: the expression is already typed as the asserted type.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = UnnecessaryTypeAssertionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

fn expression_matches_type(expr: &Expression<'_>, ts_type: &TSType<'_>) -> bool {
    match (expr, ts_type) {
        (Expression::StringLiteral(_) | Expression::TemplateLiteral(_), TSType::TSStringKeyword(_)) => {
            true
        }
        (Expression::NumericLiteral(_), TSType::TSNumberKeyword(_)) => true,
        (Expression::BooleanLiteral(_), TSType::TSBooleanKeyword(_)) => true,
        (Expression::NullLiteral(_), TSType::TSNullKeyword(_)) => true,
        (Expression::BigIntLiteral(_), TSType::TSBigIntKeyword(_)) => true,
        (Expression::Identifier(ident), TSType::TSUndefinedKeyword(_)) => {
            ident.name.as_str() == "undefined"
        }
        _ => false,
    }
}

struct UnnecessaryTypeAssertionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for UnnecessaryTypeAssertionVisitor<'a, 'f> {
    fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
        if expression_matches_type(&expr.expression, &expr.type_annotation) {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(expr.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_UNNECESSARY_TYPE_ASSERTION_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_ts_as_expression(self, expr);
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
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
    fn reports_string_as_string() -> Result<()> {
        let violations = run_check("const x = \"hello\" as string;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_number_as_number() -> Result<()> {
        let violations = run_check("const x = 42 as number;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_boolean_as_boolean() -> Result<()> {
        let violations = run_check("const x = true as boolean;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_null_as_null() -> Result<()> {
        let violations = run_check("const x = null as null;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_undefined_as_undefined() -> Result<()> {
        let violations = run_check("const x = undefined as undefined;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_bigint_as_bigint() -> Result<()> {
        let violations = run_check("const x = 0n as bigint;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_template_literal_as_string() -> Result<()> {
        let violations = run_check("const x = `hello` as string;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_string_as_other_type() -> Result<()> {
        let violations = run_check("const x = \"hello\" as const;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_number_as_string() -> Result<()> {
        let violations = run_check("const x = 42 as string;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_variable_as_same_primitive() -> Result<()> {
        let violations = run_check("const x = someVar as string;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_object_as_interface() -> Result<()> {
        let violations = run_check("const x = {} as SomeInterface;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_array_as_number_array() -> Result<()> {
        let violations = run_check("const x = [1, 2, 3] as number[];\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_as_const() -> Result<()> {
        let violations = run_check("const config = { port: 3000 } as const;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_as_in_comments() -> Result<()> {
        let violations = run_check("// const x = \"hello\" as string;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_as_in_strings() -> Result<()> {
        let violations = run_check("const text = \"x as string\";\n");
        assert!(violations.is_empty());
    
        Ok(())}
}
