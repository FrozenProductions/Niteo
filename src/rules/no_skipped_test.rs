use std::path::Path;

use crate::config::RuleConfig;
use crate::rules::{
    Fix, NO_SKIPPED_TEST_RULE_ID, TextEdit, Violation, test_call_utils,
};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Disallow skipped tests (describe.skip, it.skip, test.skip).";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let calls = test_call_utils::collect_test_property_calls(program, "skip");
    calls
        .into_iter()
        .map(|call| {
            let pos = line_index.position_for(call.member_span);
            let subject = format!("{}.skip", call.function_name);
            Violation {
                file: file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_SKIPPED_TEST_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: None,
                subject: Some(subject),
            }
        })
        .collect()
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

    let calls = test_call_utils::collect_test_property_calls(program, "skip");
    let edits: Vec<TextEdit> = calls
        .iter()
        .map(|call| {
            crate::fix::remove_span(
                call.object_span.end as usize,
                call.property_span.end as usize,
            )
        })
        .collect();

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: NO_SKIPPED_TEST_RULE_ID,
            edits,
        }]
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
            Path::new("auth.test.ts"),
            &program,
            &line_index,
            &test_config(),
        )
    }

    fn run_fix(source: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        fix_file(Path::new("auth.test.ts"), &program, source, &test_config())
    }

    fn apply_fix_edits(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|fix| fix.edits.clone()).collect();
        crate::fix::apply_edits(source, &edits)
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_describe_skip() -> Result<()> {
        let violations = run_check("describe.skip('suite', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("describe.skip"));
    
        Ok(())}

    #[test]
    fn reports_it_skip() -> Result<()> {
        let violations = run_check("it.skip('works', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("it.skip"));
    
        Ok(())}

    #[test]
    fn reports_test_skip() -> Result<()> {
        let violations = run_check("test.skip('works', () => {});");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("test.skip"));
    
        Ok(())}

    #[test]
    fn reports_multiple_skipped_tests() -> Result<()> {
        let source = "describe.skip('suite', () => { it.skip('works', () => {}); });";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn ignores_regular_describe() -> Result<()> {
        let violations = run_check("describe('suite', () => {});");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_regular_it() -> Result<()> {
        let violations = run_check("it('works', () => {});");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_regular_test() -> Result<()> {
        let violations = run_check("test('works', () => {});");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_describe_only() -> Result<()> {
        let violations = run_check("describe.only('suite', () => {});");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_it_only() -> Result<()> {
        let violations = run_check("it.only('works', () => {});");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_other_member_calls() -> Result<()> {
        let violations = run_check("foo.skip('something');");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_in_comments() -> Result<()> {
        let source = "// describe.skip('suite', () => {});";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_in_strings() -> Result<()> {
        let source = r#"const text = "it.skip('works', () => {})";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_correct_line() -> Result<()> {
        let source = "const x = 1;\nit.skip('works', () => {});\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    
        Ok(())}

    #[test]
    fn fix_removes_describe_skip() -> Result<()> {
        let source = "describe.skip('suite', () => {});";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "describe('suite', () => {});");
    
        Ok(())}

    #[test]
    fn fix_removes_it_skip() -> Result<()> {
        let source = "it.skip('works', () => {});";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "it('works', () => {});");
    
        Ok(())}

    #[test]
    fn fix_removes_test_skip() -> Result<()> {
        let source = "test.skip('works', () => {});";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "test('works', () => {});");
    
        Ok(())}

    #[test]
    fn fix_removes_multiple_skipped_tests() -> Result<()> {
        let source = "describe.skip('suite', () => { it.skip('works', () => {}); });";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "describe('suite', () => { it('works', () => {}); });");
    
        Ok(())}

    #[test]
    fn fix_leaves_non_test_skip() -> Result<()> {
        let source = "foo.skip('something');";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_no_skipped_test_returns_empty() -> Result<()> {
        let source = "describe('suite', () => {});";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_disabled_returns_empty() -> Result<()> {
        let source = "it.skip('works', () => {});";
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let config = RuleConfig {
            severity: Severity::Off,
        };
        let edits = fix_file(Path::new("auth.test.ts"), &program, source, &config);
        assert!(edits.is_empty());
    
        Ok(())}
}
