use std::path::Path;

use oxc_ast::ast::{Expression, Statement, TryStatement};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_SILENT_CATCH_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Catch blocks must log, rethrow, or return a typed fallback.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = SilentCatchVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct SilentCatchVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for SilentCatchVisitor<'a, 'f> {
    fn visit_try_statement(&mut self, try_stmt: &TryStatement<'a>) {
        if let Some(handler) = &try_stmt.handler {
            if !handler.body.body.iter().any(has_error_handling) {
                let pos = self.line_index.position_for(try_stmt.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: NO_SILENT_CATCH_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: None,
                    subject: None,
                });
            }
        }
        oxc_ast_visit::walk::walk_try_statement(self, try_stmt);
    }
}

fn has_error_handling(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ThrowStatement(_) => true,
        Statement::ReturnStatement(ret) => ret.argument.is_some(),
        Statement::ExpressionStatement(expr) => is_console_expression(&expr.expression),
        Statement::BlockStatement(block) => block.body.iter().any(has_error_handling),
        Statement::IfStatement(if_stmt) => {
            has_error_handling(&if_stmt.consequent)
                || if_stmt
                    .alternate
                    .as_ref()
                    .map_or(false, |alt| has_error_handling(alt))
        }
        Statement::SwitchStatement(switch) => switch
            .cases
            .iter()
            .any(|case| case.consequent.iter().any(has_error_handling)),
        Statement::ForStatement(for_stmt) => has_error_handling(&for_stmt.body),
        Statement::ForInStatement(for_in) => has_error_handling(&for_in.body),
        Statement::ForOfStatement(for_of) => has_error_handling(&for_of.body),
        Statement::WhileStatement(while_stmt) => has_error_handling(&while_stmt.body),
        Statement::DoWhileStatement(do_while) => has_error_handling(&do_while.body),
        Statement::TryStatement(try_stmt) => {
            try_stmt.block.body.iter().any(has_error_handling)
                || try_stmt
                    .handler
                    .as_ref()
                    .map_or(false, |h| h.body.body.iter().any(has_error_handling))
                || try_stmt
                    .finalizer
                    .as_ref()
                    .map_or(false, |f| f.body.iter().any(has_error_handling))
        }
        Statement::LabeledStatement(labeled) => has_error_handling(&labeled.body),
        _ => false,
    }
}

fn is_console_expression(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    is_console_callee(&call.callee)
}

fn is_console_callee(callee: &Expression<'_>) -> bool {
    match callee {
        Expression::StaticMemberExpression(member) => {
            matches!(&member.object, Expression::Identifier(id) if id.name == "console")
        }
        Expression::ComputedMemberExpression(member) => {
            matches!(&member.object, Expression::Identifier(id) if id.name == "console")
        }
        _ => false,
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

    #[test]
    fn reports_empty_catch() {
        let source = "try { doWork(); } catch (e) {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_catch_without_binding() {
        let source = "try { doWork(); } catch {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_silent_catch_with_statements() {
        let source = "try { doWork(); } catch (e) { const x = 1; }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_silent_catch_with_function_call() {
        let source = "try { doWork(); } catch (e) { handleError(e); }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_console_error() {
        let source = "try { doWork(); } catch (e) { console.error(e); }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_console_warn() {
        let source = "try { doWork(); } catch (e) { console.warn(e); }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_console_log() {
        let source = "try { doWork(); } catch (e) { console.log(e); }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_rethrow() {
        let source = "try { doWork(); } catch (e) { throw e; }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_rethrow_with_wrapping() {
        let source = "try { doWork(); } catch (e) { throw new Error('wrapped', { cause: e }); }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_return_with_value() {
        let source = "function f() { try { doWork(); } catch (e) { return null; } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn flags_return_without_value() {
        let source = "function f() { try { doWork(); } catch (e) { return; } }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_console_in_nested_if() {
        let source =
            "try { doWork(); } catch (e) { if (e instanceof Error) { console.error(e); } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_throw_in_nested_if_else() {
        let source = "try { doWork(); } catch (e) { if (e.code === 1) { return null; } else { throw e; } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_silent_nested_if() {
        let source = "try { doWork(); } catch (e) { if (e.code === 1) { cleanup(); } }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_console_in_switch_case() {
        let source = "try { doWork(); } catch (e) { switch (e.code) { case 1: console.error(e); break; } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_return_in_for_loop() {
        let source = "try { doWork(); } catch (e) { for (const item of []) { return item; } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_throw_in_nested_try() {
        let source = "try { doWork(); } catch (e) { try { recover(); } catch { throw e; } }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_try_without_catch() {
        let source = "try { doWork(); } finally { cleanup(); }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_catch_in_comments() {
        let source = "// try { doWork(); } catch (e) {}\n/* try { } catch (e) {} */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_catch_in_strings() {
        let source = r#"const text = "try { } catch (e) {}";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
