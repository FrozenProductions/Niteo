use std::path::Path;

use oxc_ast::ast::TSNonNullExpression;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{Fix, NO_NON_NULL_ASSERTION_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;
use oxc_span::GetSpan;

const MESSAGE: &str =
    "Avoid non-null assertions. Use proper null checks or optional chaining instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = NonNullAssertionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

pub fn fix_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    _source: &str,
    config: &RuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let mut collector = NonNullCollector {
        ranges: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);

    let edits: Vec<TextEdit> = collector
        .ranges
        .iter()
        .map(|(expr_end, span_end)| TextEdit {
            start: *expr_end as usize,
            end: *span_end as usize,
            replacement: String::new(),
        })
        .collect();

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: NO_NON_NULL_ASSERTION_RULE_ID,
            edits,
        }]
    }
}

struct NonNullCollector<'a> {
    ranges: Vec<(u32, u32)>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for NonNullCollector<'a> {
    fn visit_ts_non_null_expression(&mut self, expr: &TSNonNullExpression<'a>) {
        let expr_end = expr.expression.span().end;
        let span_end = expr.span.end;
        if span_end > expr_end {
            self.ranges.push((expr_end, span_end));
        }
        oxc_ast_visit::walk::walk_ts_non_null_expression(self, expr);
    }
}

struct NonNullAssertionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NonNullAssertionVisitor<'a, 'f> {
    fn visit_ts_non_null_expression(&mut self, expr: &TSNonNullExpression<'a>) {
        let pos = self.line_index.position_for(expr.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(expr.span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_NON_NULL_ASSERTION_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_non_null_expression(self, expr);
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

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_non_null_assertion() -> Result<()> {
        let violations = run_check("const value = obj!.prop;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_non_null_assertion_on_function_call() -> Result<()> {
        let violations = run_check("const result = getValue()!;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_non_null_assertion_on_array_access() -> Result<()> {
        let violations = run_check("const item = array[0]!;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_multiple_non_null_assertions() -> Result<()> {
        let violations = run_check("const a = x!; const b = y!;\n");
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn allows_optional_chaining() -> Result<()> {
        let violations = run_check("const value = obj?.prop;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_nullish_coalescing() -> Result<()> {
        let violations = run_check("const value = obj ?? 'default';\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_type_guard() -> Result<()> {
        let violations = run_check("if (obj) { const value = obj.prop; }\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_non_null_in_comments() -> Result<()> {
        let source = "// const value = obj!.prop;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_non_null_in_strings() -> Result<()> {
        let source = r#"const text = "obj!.prop";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_nested_non_null_assertion() -> Result<()> {
        let violations = run_check("const value = obj!.nested!.prop;\n");
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    fn run_fix(source: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        fix_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &test_config(),
        )
    }

    fn apply_fix_edits(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|fix| fix.edits.clone()).collect();
        crate::fix::apply_edits(source, &edits)
    }

    #[test]
    fn fix_removes_non_null_assertion() -> Result<()> {
        let source = "const value = obj!.prop;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const value = obj.prop;\n");
    
        Ok(())}

    #[test]
    fn fix_removes_non_null_on_function_call() -> Result<()> {
        let source = "const result = getValue()!;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const result = getValue();\n");
    
        Ok(())}

    #[test]
    fn fix_removes_non_null_on_array_access() -> Result<()> {
        let source = "const item = array[0]!;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const item = array[0];\n");
    
        Ok(())}

    #[test]
    fn fix_removes_nested_non_null_assertions() -> Result<()> {
        let source = "const value = obj!.nested!.prop;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const value = obj.nested.prop;\n");
    
        Ok(())}

    #[test]
    fn fix_does_nothing_when_no_non_null() -> Result<()> {
        let source = "const value = obj?.prop;\n";
        let fixes = run_fix(source);
        assert!(fixes.is_empty());
    
        Ok(())}

    #[test]
    fn fix_disabled_returns_empty() -> Result<()> {
        let source = "const value = obj!.prop;\n";
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let config = RuleConfig {
            severity: Severity::Off,
        };
        let fixes = fix_file(Path::new("Component.tsx"), &program, source, &config);
        assert!(fixes.is_empty());
    
        Ok(())}

    #[test]
    fn fix_preserves_surrounding_code() -> Result<()> {
        let source = "const x = 1;\nconst value = obj!.prop;\nconst y = 2;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert!(fixed.contains("const x = 1;"));
        assert!(fixed.contains("const y = 2;"));
        assert!(fixed.contains("const value = obj.prop;"));
    
        Ok(())}

    #[test]
    fn fixed_source_parses() -> Result<()> {
        let source = "const value = obj!.prop;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &fixed, SourceType::tsx()).parse();
        assert!(!parser_return.panicked);
    
        Ok(())}
}
