use std::path::Path;

use oxc_ast::ast::{ArrowFunctionExpression, Expression, Function, NewExpression, ReturnStatement};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_PROMISE_EXECUTOR_RETURN_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Return values from Promise executors cannot be handled; use resolve() or reject() instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = NoPromiseExecutorReturnVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        executor_depth: 0,
        function_depth: 0,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NoPromiseExecutorReturnVisitor<'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    executor_depth: usize,
    function_depth: usize,
}

impl<'a> Visit<'a> for NoPromiseExecutorReturnVisitor<'_> {
    fn visit_new_expression(&mut self, expr: &NewExpression<'a>) {
        if is_new_promise(expr) {
            self.executor_depth += 1;
        }
        oxc_ast_visit::walk::walk_new_expression(self, expr);
        if is_new_promise(expr) {
            self.executor_depth -= 1;
        }
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if self.executor_depth > 0 {
            self.function_depth += 1;
        }
        oxc_ast_visit::walk::walk_function(self, func, flags);
        if self.executor_depth > 0 {
            self.function_depth -= 1;
        }
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        if self.executor_depth > 0 {
            if self.function_depth == 0 && arrow.expression {
                let pos = self.line_index.position_for(arrow.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    span: Some(arrow.span),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: NO_PROMISE_EXECUTOR_RETURN_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: None,
                    subject: None,
                });
            }
            self.function_depth += 1;
        }
        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);
        if self.executor_depth > 0 {
            self.function_depth -= 1;
        }
    }

    fn visit_return_statement(&mut self, stmt: &ReturnStatement<'a>) {
        if self.executor_depth > 0 && self.function_depth <= 1 && stmt.argument.is_some() {
            let pos = self.line_index.position_for(stmt.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(stmt.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_PROMISE_EXECUTOR_RETURN_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_return_statement(self, stmt);
    }
}

fn is_new_promise(expr: &NewExpression<'_>) -> bool {
    if let Expression::Identifier(id) = &expr.callee {
        id.name == "Promise"
    } else {
        false
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
        check_file(Path::new("lib.ts"), &program, &line_index, &test_config())
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_return_value_in_arrow_executor() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => { return 42; });\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_return_value_in_function_executor() -> Result<()> {
        let violations = run_check(
            "new Promise(function(resolve, reject) { return 42; });\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_concise_arrow_executor() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => 42);\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn does_not_report_return_without_value() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => { return; });\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn does_not_report_return_in_nested_function() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => { setTimeout(() => { return resolve(42); }, 1000); });\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn does_not_report_outside_executor() -> Result<()> {
        let violations = run_check(
            "function foo() { return 42; }\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn does_not_report_non_promise_new() -> Result<()> {
        let violations = run_check(
            "new Foo((x) => { return 42; });\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_deep_nested_return_at_executor_level() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => { if (true) { return 42; } });\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_multiple_returns_in_executor() -> Result<()> {
        let violations = run_check(
            "new Promise((resolve, reject) => { if (a) { return 1; } return 2; });\n",
        );
        assert_eq!(violations.len(), 2);
        Ok(())
    }
}
