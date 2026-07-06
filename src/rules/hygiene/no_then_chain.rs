use std::collections::HashSet;
use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;
use oxc_span::Span;

use crate::config::NoThenChainRuleConfig;
use crate::rules::{NO_THEN_CHAIN_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Prefer async/await over .then() chains.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoThenChainRuleConfig,
) -> Vec<Violation> {
    let mut visitor = ThenChainVisitor {
        then_spans: Vec::new(),
        has_chain_after: HashSet::new(),
        follows_then: HashSet::new(),
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);

    let mut violations = Vec::new();
    for span in &visitor.then_spans {
        let is_chained = visitor.has_chain_after.contains(span) || visitor.follows_then.contains(span);
        if !config.allow_single || is_chained {
            let pos = line_index.position_for(*span);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(*span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_THEN_CHAIN_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: None,
                subject: None,
            });
        }
    }
    violations
}

struct ThenChainVisitor<'a> {
    then_spans: Vec<Span>,
    has_chain_after: HashSet<Span>,
    follows_then: HashSet<Span>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for ThenChainVisitor<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = &call.callee {
            let name = &member.property.name;

            if is_chain_method_name(name) {
                // The object before the dot is the thing being chained FROM.
                if let Expression::CallExpression(inner) = &member.object
                    && is_chain_method(&inner.callee)
                {
                    self.has_chain_after.insert(inner.span);
                }
            }

            if name == "then" {
                self.then_spans.push(call.span);

                if let Expression::CallExpression(inner) = &member.object
                    && is_then_call(&inner.callee)
                {
                    self.follows_then.insert(call.span);
                }
            }
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }
}

fn is_chain_method_name(name: &str) -> bool {
    name == "then" || name == "catch" || name == "finally"
}

fn is_chain_method(callee: &Expression) -> bool {
    if let Expression::StaticMemberExpression(member) = callee {
        is_chain_method_name(&member.property.name)
    } else {
        false
    }
}

fn is_then_call(callee: &Expression) -> bool {
    if let Expression::StaticMemberExpression(member) = callee {
        member.property.name == "then"
    } else {
        false
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NoThenChainRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, allow_single: bool) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(Path::new("lib.ts"), &program, &line_index, &test_config(allow_single))
    }

    fn test_config(allow_single: bool) -> NoThenChainRuleConfig {
        NoThenChainRuleConfig {
            severity: Severity::Warn,
            allow_single,
        }
    }

    #[test]
    fn skips_single_then_when_allow_single() -> Result<()> {
        let violations = run_check("fetch('/api').then(res => res.json());\n", true);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_single_then_when_not_allow_single() -> Result<()> {
        let violations = run_check("fetch('/api').then(res => res.json());\n", false);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        Ok(())
    }

    #[test]
    fn reports_chained_then_with_allow_single() -> Result<()> {
        let violations = run_check(
            "fetch('/api')\n  .then(res => res.json())\n  .then(data => console.log(data));\n",
            true,
        );
        assert_eq!(violations.len(), 2);
        Ok(())
    }

    #[test]
    fn reports_chained_then_without_allow_single() -> Result<()> {
        let violations = run_check(
            "fetch('/api')\n  .then(res => res.json())\n  .then(data => console.log(data));\n",
            false,
        );
        assert_eq!(violations.len(), 2);
        Ok(())
    }

    #[test]
    fn reports_then_with_catch_allow_single() -> Result<()> {
        let violations =
            run_check("fetch('/api').then(res => res.json()).catch(err => console.error(err));\n", true);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_then_with_finally_allow_single() -> Result<()> {
        let violations =
            run_check("fetch('/api').then(res => res.json()).finally(() => cleanup());\n", true);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn skips_then_after_catch_when_allow_single() -> Result<()> {
        let violations =
            run_check("fetch('/api').catch(err => fallback).then(res => res.json());\n", true);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn does_not_report_other_member_calls() -> Result<()> {
        let violations = run_check("[1, 2, 3].map(x => x * 2);\n", true);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn skips_plain_thenable_when_allow_single() -> Result<()> {
        let violations = run_check("const obj = { then: () => {} }; obj.then();\n", true);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_plain_thenable_when_not_allow_single() -> Result<()> {
        let violations = run_check("const obj = { then: () => {} }; obj.then();\n", false);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn skips_single_then_inside_callback_when_allow_single() -> Result<()> {
        let violations =
            run_check("function doWork() { return fetch('/api').then(res => res.json()); }\n", true);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_then_inside_callback_when_not_allow_single() -> Result<()> {
        let violations =
            run_check("function doWork() { return fetch('/api').then(res => res.json()); }\n", false);
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_triple_then_chain() -> Result<()> {
        let violations = run_check(
            "fetch('/api').then(a).then(b).then(c);\n",
            true,
        );
        assert_eq!(violations.len(), 3);
        Ok(())
    }
}
