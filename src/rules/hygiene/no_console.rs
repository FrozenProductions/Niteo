use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;

use crate::config::NoConsoleRuleConfig;
use crate::rules::{NO_CONSOLE_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Disallow console statements outside allowed file patterns.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoConsoleRuleConfig,
) -> Vec<Violation> {
    if is_file_allowed(file, config) {
        return Vec::new();
    }

    let mut visitor = ConsoleVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ConsoleVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ConsoleVisitor<'a, 'f> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(static_member) = &call.callee
            && matches!(&static_member.object, Expression::Identifier(id) if id.name == "console")
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                static_member.span,
                self.severity,
            ));
        }
        if let Expression::ComputedMemberExpression(computed) = &call.callee
            && matches!(&computed.object, Expression::Identifier(id) if id.name == "console")
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                computed.span,
                self.severity,
            ));
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
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
        rule: NO_CONSOLE_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn is_file_allowed(file: &Path, config: &NoConsoleRuleConfig) -> bool {
    let file_str = file.to_string_lossy();
    config
        .allow_patterns
        .iter()
        .any(|pattern| file_str.contains(pattern))
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NoConsoleRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, config: &NoConsoleRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(Path::new("Component.tsx"), &program, &line_index, config)
    }

    #[test]
    fn reports_console_methods() -> Result<()> {
        for source in [
            "console.log('hello');\n",
            "console.warn('warning');\n",
            "console.error('error');\n",
        ] {
            let violations = run_check(source, &test_config());
            assert_eq!(violations.len(), 1, "expected 1 violation for: {source:?}");
        }
    
        Ok(())}

    #[test]
    fn reports_console_bracket_access() -> Result<()> {
        let violations = run_check("console['log']('hello');\n", &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_console_in_service_files() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec![".service.ts".to_string()],
        };
        let allocator = Allocator::default();
        let source = "console.log('hello');\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let violations = check_file(Path::new("api.service.ts"), &program, &line_index, &config);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_console_in_comments() -> Result<()> {
        let source = "// console.log('hello');\n/* console.warn('test'); */\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_console_in_strings() -> Result<()> {
        let source = r#"const text = "console.log('hello')";"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_log_property_on_non_console_object() -> Result<()> {
        let source = r#"const logger = { log: console.log }; logger.log('hello');"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> NoConsoleRuleConfig {
        NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec![],
        }
    }
}
