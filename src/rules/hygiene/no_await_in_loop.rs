use std::path::Path;

use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_AWAIT_IN_LOOP_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Avoid await inside loops; use Promise.all or extract to a separate async function.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = NoAwaitInLoopVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        loop_depth: 0,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NoAwaitInLoopVisitor<'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    loop_depth: usize,
}

impl<'a> Visit<'a> for NoAwaitInLoopVisitor<'_> {
    fn visit_await_expression(&mut self, expr: &oxc_ast::ast::AwaitExpression<'a>) {
        if self.loop_depth > 0 {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(expr.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_AWAIT_IN_LOOP_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_await_expression(self, expr);
    }

    fn visit_for_statement(&mut self, stmt: &oxc_ast::ast::ForStatement<'a>) {
        self.loop_depth += 1;
        oxc_ast_visit::walk::walk_for_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_while_statement(&mut self, stmt: &oxc_ast::ast::WhileStatement<'a>) {
        self.loop_depth += 1;
        oxc_ast_visit::walk::walk_while_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_do_while_statement(&mut self, stmt: &oxc_ast::ast::DoWhileStatement<'a>) {
        self.loop_depth += 1;
        oxc_ast_visit::walk::walk_do_while_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_for_in_statement(&mut self, stmt: &oxc_ast::ast::ForInStatement<'a>) {
        self.loop_depth += 1;
        oxc_ast_visit::walk::walk_for_in_statement(self, stmt);
        self.loop_depth -= 1;
    }

    fn visit_for_of_statement(&mut self, stmt: &oxc_ast::ast::ForOfStatement<'a>) {
        if !stmt.r#await {
            self.loop_depth += 1;
        }
        oxc_ast_visit::walk::walk_for_of_statement(self, stmt);
        if !stmt.r#await {
            self.loop_depth -= 1;
        }
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
    fn reports_await_in_for_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { for (let i = 0; i < 10; i++) { await delay(i); } }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_await_in_for_of_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { for (const item of items) { await process(item); } }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_await_in_for_in_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { for (const key in obj) { await save(key); } }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_await_in_while_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { while (hasMore()) { await next(); } }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_await_in_do_while_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { do { await step(); } while (cond()); }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_await_in_nested_loops() -> Result<()> {
        let violations = run_check(
            "async function f() { for (const row of rows) { for (const col of cols) { await cell(row, col); } } }\n",
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn does_not_report_await_outside_loop() -> Result<()> {
        let violations = run_check(
            "async function f() { await delay(); const x = await fetch(); }\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn does_not_report_non_loop_await() -> Result<()> {
        let violations = run_check(
            "async function f() { const result = await fetch('/api'); return result; }\n",
        );
        assert!(violations.is_empty());
        Ok(())
    }
}
