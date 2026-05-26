use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_THEN_CHAIN_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Prefer async/await over .then() chains.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = ThenChainVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ThenChainVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ThenChainVisitor<'a, 'f> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = &call.callee
            && member.property.name == "then"
        {
            let pos = self.line_index.position_for(member.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_THEN_CHAIN_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
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
        check_file(Path::new("lib.ts"), &program, &line_index, &test_config())
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_basic_then_chain() {
        let violations = run_check("fetch('/api').then(res => res.json());\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_chained_then() {
        let violations = run_check(
            "fetch('/api')\n  .then(res => res.json())\n  .then(data => console.log(data));\n",
        );
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn reports_then_with_catch() {
        let violations =
            run_check("fetch('/api').then(res => res.json()).catch(err => console.error(err));\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn does_not_report_other_member_calls() {
        let violations = run_check("[1, 2, 3].map(x => x * 2);\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_then_on_any_object() {
        let violations = run_check("const obj = { then: () => {} }; obj.then();\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_then_inside_callback() {
        let violations =
            run_check("function doWork() { return fetch('/api').then(res => res.json()); }\n");
        assert_eq!(violations.len(), 1);
    }
}
