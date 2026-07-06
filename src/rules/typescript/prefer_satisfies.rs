use std::path::Path;

use oxc_ast::ast::{Expression, TSAsExpression, TSType};
use oxc_ast_visit::Visit;

use crate::config::{RuleConfig, Severity};
use crate::rules::{Fix, PREFER_SATISFIES_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;
use oxc_span::GetSpan;
const MESSAGE: &str =
    "Prefer 'satisfies' over 'as' for type validation without changing the inferred type.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = PreferSatisfiesVisitor {
        violations: Vec::new(),
        file,
        line_index,
        source: program.source_text,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

pub fn fix_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    source: &str,
    config: &RuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let mut collector = AsExpressionCollector {
        ranges: Vec::new(),
        source,
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);

    let mut edits = Vec::new();
    for (expr_end, type_start) in &collector.ranges {
        edits.push(TextEdit {
            start: *expr_end as usize,
            end: *type_start as usize,
            replacement: " satisfies ".to_string(),
        });
    }

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: PREFER_SATISFIES_RULE_ID,
            edits,
        }]
    }
}

struct AsExpressionCollector<'a> {
    ranges: Vec<(u32, u32)>,
    source: &'a str,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for AsExpressionCollector<'a> {
    fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
        if should_prefer_satisfies(expr, self.source) {
            self.ranges
                .push((expr.expression.span().end, expr.type_annotation.span().start));
        }
        oxc_ast_visit::walk::walk_ts_as_expression(self, expr);
    }
}

struct PreferSatisfiesVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    source: &'a str,
    severity: Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for PreferSatisfiesVisitor<'a, 'f> {
    fn visit_ts_as_expression(&mut self, expr: &TSAsExpression<'a>) {
        if should_prefer_satisfies(expr, self.source) {
            let pos = self.line_index.position_for(expr.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(expr.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: PREFER_SATISFIES_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_ts_as_expression(self, expr);
    }
}

fn should_prefer_satisfies(expr: &TSAsExpression<'_>, source: &str) -> bool {
    if is_excluded_type(&expr.type_annotation, source) {
        return false;
    }

    is_literal_expression(&expr.expression)
}

fn is_excluded_type(ts_type: &TSType<'_>, source: &str) -> bool {
    match ts_type {
        TSType::TSAnyKeyword(_) | TSType::TSUnknownKeyword(_) => true,
        TSType::TSTypeReference(type_ref) => {
            let type_text = source
                .get(type_ref.span.start as usize..type_ref.span.end as usize)
                .unwrap_or("");
            type_text.trim() == "const"
        }
        _ => false,
    }
}

fn is_literal_expression(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::BooleanLiteral(_)
    )
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::{check_file, fix_file};
    use crate::config::{RuleConfig, Severity};
    use crate::rules::{Fix, TextEdit, Violation};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("value.ts"), &program, &line_index, &test_config())
    }

    #[test]
    fn reports_literal_as_cast() -> Result<()> {
        for source in [
            "const config = { port: 3000 } as Config;\n",
            "const items = [1, 2, 3] as number[];\n",
            "const event = \"click\" as EventName;\n",
            "const code = 404 as StatusCode;\n",
        ] {
            let violations = run_check(source);
            assert_eq!(violations.len(), 1, "expected 1 violation for: {source:?}");
            assert_eq!(violations[0].line, Some(1));
        }
    
        Ok(())}

    #[test]
    fn allows_as_const() -> Result<()> {
        let violations = run_check("const config = { port: 3000 } as const;\n");

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_as_any() -> Result<()> {
        let violations = run_check("const value = something as any;\n");

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_as_unknown() -> Result<()> {
        let violations = run_check("const value = something as unknown as Target;\n");

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_variable_as_cast() -> Result<()> {
        let violations = run_check("const value = someVar as Target;\n");

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_function_call_as_cast() -> Result<()> {
        let violations = run_check("const result = getData() as Response;\n");

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_as_in_comments_and_strings() -> Result<()> {
        let source = r#"// const x = {} as Config;
const text = "as Config";
/* const x = {} as Config; */
"#;
        let violations = run_check(source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_as_in_identifier() -> Result<()> {
        let source = "const task = 'hello';\n";
        let violations = run_check(source);

        assert!(violations.is_empty());
    
        Ok(())}

    fn run_fix(source: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        fix_file(Path::new("value.ts"), &program, source, &test_config())
    }

    fn apply_fix_edits(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|fix| fix.edits.clone()).collect();
        crate::fix::apply_edits(source, &edits)
    }

    #[test]
    fn fix_converts_as_to_satisfies() -> Result<()> {
        let source = "const config = { port: 3000 } as Config;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "const config = { port: 3000 } satisfies Config;\n");
    
        Ok(())}

    #[test]
    fn fix_does_not_change_as_const() -> Result<()> {
        let source = "const config = { port: 3000 } as const;\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_converts_array_as_cast() -> Result<()> {
        let source = "const items = [1, 2, 3] as number[];\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "const items = [1, 2, 3] satisfies number[];\n");
    
        Ok(())}

    #[test]
    fn fix_converts_string_literal_as_cast() -> Result<()> {
        let source = "const event = \"click\" as EventName;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "const event = \"click\" satisfies EventName;\n");
    
        Ok(())}

    #[test]
    fn fix_converts_multiple_as_casts() -> Result<()> {
        let source = "const a = { x: 1 } as A;\nconst b = { y: 2 } as B;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "const a = { x: 1 } satisfies A;\nconst b = { y: 2 } satisfies B;\n");
    
        Ok(())}

    #[test]
    fn fix_disabled_returns_empty() -> Result<()> {
        let source = "const config = { port: 3000 } as Config;\n";
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let config = RuleConfig {
            severity: Severity::Off,
        };
        let edits = fix_file(Path::new("value.ts"), &program, source, &config);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fixed_source_parses() -> Result<()> {
        let source = "const config = { port: 3000 } as Config;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &fixed, SourceType::ts()).parse();
        assert!(!parser_return.panicked);
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
