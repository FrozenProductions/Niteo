use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression, NewExpression};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_EVAL_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Disallow eval() and new Function() as they execute arbitrary code.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = EvalVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct EvalVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for EvalVisitor<'a, 'f> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(id) = &call.callee
            && id.name == "eval"
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                id.span,
                self.severity,
            ));
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, expr: &NewExpression<'a>) {
        if let Expression::Identifier(id) = &expr.callee
            && id.name == "Function"
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                id.span,
                self.severity,
            ));
        }
        oxc_ast_visit::walk::walk_new_expression(self, expr);
    }
}

fn make_violation(
    file: &Path,
    line_index: &LineIndex,
    span: oxc_span::Span,
    severity: crate::config::Severity,
) -> Violation {
    let pos = line_index.position_for(span);
    Violation {
        file: file.to_path_buf(),
        line: Some(pos.line),
        column: Some(pos.column),
        rule: NO_EVAL_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
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

    #[test]
    fn reports_eval_call() -> Result<()> {
        let violations = run_check("eval('code');\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_new_function() -> Result<()> {
        let violations = run_check("new Function('return 1');\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_new_function_with_space() -> Result<()> {
        let violations = run_check("new  Function('return 1');\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_eval_in_comments() -> Result<()> {
        let source = "// eval('code');\n/* new Function('test'); */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_eval_in_strings() -> Result<()> {
        let source = r#"const text = "eval('hello')";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragment() -> Result<()> {
        let source = "const evaluate = true;\nconst FunctionBuilder = class {};\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
