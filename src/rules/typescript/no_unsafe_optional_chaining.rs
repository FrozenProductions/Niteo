use std::path::Path;

use oxc_ast::ast::{ChainElement, ChainExpression};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_UNSAFE_OPTIONAL_CHAINING_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Unsafe optional chaining on a non-nullable expression. Remove the `?.` or use a nullable type.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = UnsafeOptionalChainingVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct UnsafeOptionalChainingVisitor<'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
}

fn root_expression<'a>(element: &'a ChainElement<'a>) -> &'a oxc_ast::ast::Expression<'a> {
    match element {
        ChainElement::StaticMemberExpression(member) => {
            find_root_in_expression(&member.object)
        }
        ChainElement::ComputedMemberExpression(member) => {
            find_root_in_expression(&member.object)
        }
        ChainElement::PrivateFieldExpression(member) => {
            find_root_in_expression(&member.object)
        }
        ChainElement::CallExpression(call) => {
            find_root_in_expression(&call.callee)
        }
        ChainElement::TSNonNullExpression(non_null) => {
            find_root_in_expression(&non_null.expression)
        }
    }
}

fn find_root_in_expression<'a>(expr: &'a oxc_ast::ast::Expression<'a>) -> &'a oxc_ast::ast::Expression<'a> {
    use oxc_ast::ast::Expression;
    match expr {
        Expression::ParenthesizedExpression(paren) => {
            find_root_in_expression(&paren.expression)
        }
        Expression::ChainExpression(chain) => {
            root_expression(&chain.expression)
        }
        Expression::StaticMemberExpression(member) => {
            find_root_in_expression(&member.object)
        }
        Expression::ComputedMemberExpression(member) => {
            find_root_in_expression(&member.object)
        }
        Expression::PrivateFieldExpression(member) => {
            find_root_in_expression(&member.object)
        }
        Expression::CallExpression(call) => {
            find_root_in_expression(&call.callee)
        }
        _ => expr,
    }
}

fn is_non_nullable(expr: &oxc_ast::ast::Expression) -> bool {
    use oxc_ast::ast::Expression;
    matches!(
        expr,
        Expression::ThisExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::NewExpression(_)
            | Expression::TemplateLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TSNonNullExpression(_)
            | Expression::MetaProperty(_)
    )
}

impl<'a> Visit<'a> for UnsafeOptionalChainingVisitor<'_> {
    fn visit_chain_expression(&mut self, expr: &ChainExpression<'a>) {
        let root = root_expression(&expr.expression);
        if is_non_nullable(root) {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(expr.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_UNSAFE_OPTIONAL_CHAINING_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_chain_expression(self, expr);
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
    fn reports_optional_chaining_on_this() -> Result<()> {
        let violations = run_check("const value = this?.prop;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_literal_array() -> Result<()> {
        let violations = run_check("const length = [1, 2, 3]?.length;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_literal_object() -> Result<()> {
        let violations = run_check("const result = {}?.toString();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_literal_string() -> Result<()> {
        let violations = run_check(r#"const result = "hello"?.length;"#);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_literal_number() -> Result<()> {
        let violations = run_check("const result = 42?.toString();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_literal_boolean() -> Result<()> {
        let violations = run_check("const result = true?.valueOf();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_new_expression() -> Result<()> {
        let violations = run_check("const result = new Foo()?.bar;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_arrow_function() -> Result<()> {
        let violations = run_check("const result = (() => {})?.call();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_function_expression() -> Result<()> {
        let violations = run_check("const result = function() {}?.call();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_class_expression() -> Result<()> {
        let violations = run_check("const result = class {}?.prototype;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_template_literal() -> Result<()> {
        let violations = run_check("const result = `hello`?.length;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_regex_literal() -> Result<()> {
        let violations = run_check("const result = /test/?.exec(str);\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_bigint_literal() -> Result<()> {
        let violations = run_check("const result = 100n?.toString();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_non_null_assertion() -> Result<()> {
        let violations = run_check("const result = obj!?.prop;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_nested_non_null() -> Result<()> {
        let violations = run_check("const result = obj.prop!?.nested;\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_optional_chaining_on_meta_property() -> Result<()> {
        let violations = run_check(
            "class Foo {
                bar() {
                    const meta = new.target?.name;
                }
            }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn allows_optional_chaining_on_variable() -> Result<()> {
        let violations = run_check("const value = obj?.prop;\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_optional_chaining_on_function_call() -> Result<()> {
        let violations = run_check("const value = getObj()?.prop;\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_optional_chaining_on_chain_expression() -> Result<()> {
        let violations = run_check("const value = obj?.a?.b;\n");
        assert_eq!(violations.len(), 0);
        Ok(())
    }

    #[test]
    fn allows_optional_chaining_on_indexed_access() -> Result<()> {
        let violations = run_check("const value = obj?.prop?.nested;\n");
        assert_eq!(violations.len(), 0);
        Ok(())
    }

    #[test]
    fn allows_optional_chaining_on_null_literal() -> Result<()> {
        let violations = run_check("const value = null?.toString();\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_chained_optional_on_literal_then_variable() -> Result<()> {
        let violations = run_check("const result = 'hello'?.length?.toString();\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }
}
