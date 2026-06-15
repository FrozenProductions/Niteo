use std::path::Path;

use oxc_ast_visit::Visit;

use crate::config::NoNestedFunctionsRuleConfig;
use crate::rules::{NO_NESTED_FUNCTIONS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Function is nested too deeply. Extract it to a top-level or module-scope declaration.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoNestedFunctionsRuleConfig,
) -> Vec<Violation> {
    let mut visitor = NestedFunctionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        max_depth: config.max_depth,
        current_depth: 0,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NestedFunctionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    max_depth: usize,
    current_depth: usize,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NestedFunctionVisitor<'a, 'f> {
    fn visit_function(
        &mut self,
        func: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        self.current_depth += 1;

        if self.current_depth > self.max_depth {
            let span = func.id.as_ref().map(|id| id.span).unwrap_or(func.span);
            let pos = self.line_index.position_for(span);
            let name = func
                .id
                .as_ref()
                .map(|id| id.name.as_str().to_string())
                .unwrap_or_else(|| "<anonymous>".to_string());
            let detail = Some(format!(
                "depth {} exceeds max-depth {}",
                self.current_depth, self.max_depth
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_NESTED_FUNCTIONS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some(name),
            });
        }

        oxc_ast_visit::walk::walk_function(self, func, flags);
        self.current_depth -= 1;
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        self.current_depth += 1;

        if self.current_depth > self.max_depth {
            let pos = self.line_index.position_for(arrow.span);
            let detail = Some(format!(
                "depth {} exceeds max-depth {}",
                self.current_depth, self.max_depth
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_NESTED_FUNCTIONS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some("<arrow>".to_string()),
            });
        }

        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);
        self.current_depth -= 1;
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NoNestedFunctionsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, max_depth: usize) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("test.ts"),
            &program,
            &line_index,
            &test_config(max_depth),
        )
    }

    fn test_config(max_depth: usize) -> NoNestedFunctionsRuleConfig {
        NoNestedFunctionsRuleConfig {
            severity: Severity::Warn,
            max_depth,
        }
    }

    #[test]
    fn allows_top_level_function() -> Result<()> {
        let source = "function foo() {}\n";
        let violations = run_check(source, 1);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_one_level_of_nesting_with_max_depth_2() -> Result<()> {
        let source = "function outer() { function inner() {} }\n";
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_two_levels_of_nesting_with_max_depth_2() -> Result<()> {
        let source =
            "function outer() { function middle() { function inner() {} } }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("inner"));
    
        Ok(())}

    #[test]
    fn reports_arrow_functions_as_nesting() -> Result<()> {
        let source =
            "function outer() { const inner = () => { const deep = () => {} }; }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_nested_arrows_in_arrow() -> Result<()> {
        let source = "const a = () => { const b = () => { const c = () => {} }; };\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_flat_callbacks() -> Result<()> {
        let source =
            "function outer() { [1, 2].map(x => x + 1); }\n";
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_multiple_violations() -> Result<()> {
        let source = r#"function a() {
  function b() {
    function c() {}
    function d() {}
  }
}
"#;
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn depth_resets_after_function_ends() -> Result<()> {
        let source = r#"function first() {
  function nested() {}
}
function second() {
  function nested() {}
}
"#;
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn max_depth_1_reports_any_nesting() -> Result<()> {
        let source = "function outer() { function inner() {} }\n";
        let violations = run_check(source, 1);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("inner"));
    
        Ok(())}

    #[test]
    fn mixed_function_and_arrow_nesting() -> Result<()> {
        let source =
            "function outer() { const mid = () => { function deep() {} } }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("deep"));
    
        Ok(())}
}
