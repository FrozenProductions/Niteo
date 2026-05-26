use std::path::Path;

use oxc_ast::ast::DebuggerStatement;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_DEBUGGER_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Remove debugger statements before committing code.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = DebuggerVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct DebuggerVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for DebuggerVisitor<'a, 'f> {
    fn visit_debugger_statement(&mut self, stmt: &DebuggerStatement) {
        let pos = self.line_index.position_for(stmt.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_DEBUGGER_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_debugger_statement(self, stmt);
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
    fn reports_debugger_statement() {
        let violations = run_check("debugger;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_debugger_without_semicolon() {
        let violations = run_check("debugger\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_debugger_in_comments() {
        let source = "// debugger;\n/* debugger; */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_debugger_in_strings() {
        let source = r#"const text = "debugger";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragment() {
        let source = "const debuggerHelper = true;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
