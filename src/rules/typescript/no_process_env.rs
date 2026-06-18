use std::path::Path;

use oxc_ast::ast::{ComputedMemberExpression, Expression, StaticMemberExpression};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_PROCESS_ENV_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Use the config module instead of direct process.env access.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = ProcessEnvVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ProcessEnvVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ProcessEnvVisitor<'a, 'f> {
    fn visit_static_member_expression(&mut self, expr: &StaticMemberExpression<'a>) {
        if matches_process_env(expr) {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_PROCESS_ENV_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
            return;
        }
        oxc_ast_visit::walk::walk_static_member_expression(self, expr);
    }

    fn visit_computed_member_expression(&mut self, expr: &ComputedMemberExpression<'a>) {
        if let Expression::StaticMemberExpression(inner) = &expr.object
            && matches_process_env(inner)
        {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_PROCESS_ENV_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
            return;
        }
        oxc_ast_visit::walk::walk_computed_member_expression(self, expr);
    }
}

fn matches_process_env(expr: &StaticMemberExpression<'_>) -> bool {
    if matches!(&expr.object, Expression::Identifier(id) if id.name == "process")
        && expr.property.name == "env"
    {
        return true;
    }
    if let Expression::StaticMemberExpression(inner) = &expr.object {
        return matches_process_env(inner);
    }
    false
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
    fn reports_direct_process_env_access() -> Result<()> {
        let violations = run_check("const env = process.env;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(13));
    
        Ok(())}

    #[test]
    fn reports_process_env_property_access() -> Result<()> {
        let violations = run_check("const key = process.env.API_KEY;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(13));
    
        Ok(())}

    #[test]
    fn reports_process_env_computed_access() -> Result<()> {
        let violations = run_check(r#"const key = process.env["API_KEY"];\n"#);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_process_env_in_conditional() -> Result<()> {
        let violations = run_check("if (process.env.NODE_ENV === 'production') {}\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_process_without_env() -> Result<()> {
        let source = "const pid = process.pid;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_env_not_on_process() -> Result<()> {
        let source = "const env = app.env;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_process_env_in_comments() -> Result<()> {
        let source = "// const key = process.env.API_KEY;\n/* process.env.NODE_ENV */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_process_env_in_strings() -> Result<()> {
        let source = r#"const text = "process.env.API_KEY";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
