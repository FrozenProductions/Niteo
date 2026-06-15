use std::path::Path;

use oxc_ast_visit::Visit;
use oxc_span::GetSpan;

use crate::config::MaxFunctionParamsRuleConfig;
use crate::rules::{MAX_FUNCTION_PARAMS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Function has too many parameters. Consider using an object parameter instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &MaxFunctionParamsRuleConfig,
) -> Vec<Violation> {
    let mut visitor = MaxFunctionParamsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        max_params: config.max_params,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct MaxFunctionParamsVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    max_params: usize,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> MaxFunctionParamsVisitor<'a, 'f> {
    fn check_params(
        &mut self,
        params: &oxc_ast::ast::FormalParameters<'a>,
        span: oxc_span::Span,
        name: Option<&str>,
    ) {
        let param_count = params.items.len();
        if param_count > self.max_params {
            let pos = self.line_index.position_for(span);
            let detail = Some(format!(
                "{} parameters exceeds max-params {}",
                param_count, self.max_params
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: MAX_FUNCTION_PARAMS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: name.map(|n| n.to_string()),
            });
        }
    }
}

impl<'a, 'f> Visit<'a> for MaxFunctionParamsVisitor<'a, 'f> {
    fn visit_function(
        &mut self,
        func: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        let name = func.id.as_ref().map(|id| id.name.as_str());
        let span = func.id.as_ref().map(|id| id.span).unwrap_or(func.span);
        self.check_params(&func.params, span, name);
        oxc_ast_visit::walk::walk_function(self, func, flags);
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        self.check_params(&arrow.params, arrow.span, None);
        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_method_definition(&mut self, method: &oxc_ast::ast::MethodDefinition<'a>) {
        let name = match &method.key {
            oxc_ast::ast::PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        };
        let span = method.key.span();
        self.check_params(&method.value.params, span, name);
        if let Some(body) = &method.value.body {
            oxc_ast_visit::walk::walk_function_body(self, body);
        }
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{MaxFunctionParamsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, max_params: usize) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("test.ts"),
            &program,
            &line_index,
            &test_config(max_params),
        )
    }

    fn test_config(max_params: usize) -> MaxFunctionParamsRuleConfig {
        MaxFunctionParamsRuleConfig {
            severity: Severity::Warn,
            max_params,
        }
    }

    #[test]
    fn allows_function_within_limit() -> Result<()> {
        let violations = run_check("function add(a: number, b: number) { return a + b; }", 3);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_function_exceeding_limit() -> Result<()> {
        let violations = run_check(
            "function createUser(name: string, age: number, email: string, role: string) {}",
            3,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("createUser"));
    
        Ok(())}

    #[test]
    fn allows_arrow_within_limit() -> Result<()> {
        let violations = run_check("const add = (a: number, b: number) => a + b;", 3);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_arrow_exceeding_limit() -> Result<()> {
        let violations = run_check(
            "const createUser = (name: string, age: number, email: string, role: string) => {};",
            3,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_method_within_limit() -> Result<()> {
        let violations = run_check(
            "class User { greet(name: string, greeting: string) { return `${greeting}, ${name}`; } }",
            3,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_method_exceeding_limit() -> Result<()> {
        let violations = run_check(
            "class User { create(name: string, age: number, email: string, role: string) {} }",
            3,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("create"));
    
        Ok(())}

    #[test]
    fn allows_function_at_exact_limit() -> Result<()> {
        let violations = run_check(
            "function foo(a: number, b: number, c: number) {}",
            3,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_multiple_violations() -> Result<()> {
        let source = r#"
function foo(a: number, b: number, c: number, d: number) {}
const bar = (x: number, y: number, z: number, w: number) => {};
"#;
        let violations = run_check(source, 3);
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn reports_correct_line() -> Result<()> {
        let source = "const x = 1;\nfunction foo(a: number, b: number, c: number, d: number) {}\n";
        let violations = run_check(source, 3);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    
        Ok(())}

    #[test]
    fn allows_function_expression_within_limit() -> Result<()> {
        let violations = run_check(
            "const add = function(a: number, b: number) { return a + b; };",
            3,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_function_expression_exceeding_limit() -> Result<()> {
        let violations = run_check(
            "const createUser = function(name: string, age: number, email: string, role: string) {};",
            3,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_in_comments() -> Result<()> {
        let source = "// function foo(a: number, b: number, c: number, d: number) {}";
        let violations = run_check(source, 3);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_zero_params() -> Result<()> {
        let violations = run_check("function foo() {}", 0);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_one_param_when_max_is_zero() -> Result<()> {
        let violations = run_check("function foo(a: number) {}", 0);
        assert_eq!(violations.len(), 1);
    
        Ok(())}
}
