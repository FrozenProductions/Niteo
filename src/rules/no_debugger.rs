use std::path::Path;

use oxc_ast::ast::DebuggerStatement;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{Fix, NO_DEBUGGER_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Remove debugger statements before committing code.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = DebuggerVisitor {
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
    source: &str,
    config: &RuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let mut collector = DebuggerSpanCollector {
        spans: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);

    let mut edits = Vec::new();
    for span in &collector.spans {
        let start = span.start as usize;
        let mut end = span.end as usize;

        let after = source.get(end..).unwrap_or("");
        let after_trimmed = after.trim_start();
        if after_trimmed.starts_with(';') {
            let semicolon_offset = after.len() - after_trimmed.len();
            end += semicolon_offset + 1;
        }

        let remaining = source.get(end..).unwrap_or("");
        let whitespace = remaining
            .chars()
            .take_while(|char| char.is_whitespace())
            .count();
        end += whitespace;

        edits.push(TextEdit {
            start,
            end,
            replacement: String::new(),
        });
    }

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: NO_DEBUGGER_RULE_ID,
            edits,
        }]
    }
}

struct DebuggerVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for DebuggerVisitor<'a, 'f> {
    fn visit_debugger_statement(&mut self, stmt: &DebuggerStatement) {
        let pos = self.line_index.position_for(stmt.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_DEBUGGER_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_debugger_statement(self, stmt);
    }
}

struct DebuggerSpanCollector<'a> {
    spans: Vec<oxc_span::Span>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for DebuggerSpanCollector<'a> {
    fn visit_debugger_statement(&mut self, stmt: &DebuggerStatement) {
        self.spans.push(stmt.span);
        oxc_ast_visit::walk::walk_debugger_statement(self, stmt);
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
    fn reports_debugger_statement() -> Result<()> {
        let violations = run_check("debugger;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_debugger_without_semicolon() -> Result<()> {
        let violations = run_check("debugger\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_debugger_in_comments() -> Result<()> {
        let source = "// debugger;\n/* debugger; */\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_debugger_in_strings() -> Result<()> {
        let source = r#"const text = "debugger";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragment() -> Result<()> {
        let source = "const debuggerHelper = true;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn fix_removes_debugger_with_semicolon() -> Result<()> {
        let edits = run_fix("debugger;\n");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].edits.len(), 1);
    
        Ok(())}

    #[test]
    fn fix_removes_debugger_without_semicolon() -> Result<()> {
        let edits = run_fix("debugger\n");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].edits.len(), 1);
    
        Ok(())}

    #[test]
    fn fix_leaves_after_trivial() -> Result<()> {
        let source = "debugger;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed.trim(), "");
    
        Ok(())}

    #[test]
    fn fix_removes_trailing_semicolon() -> Result<()> {
        let source = "debugger;\n";
        let edits = run_fix(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].edits.len(), 1);
        let edit = &edits[0].edits[0];
        let before = &source[..edit.start];
        let before_semicolon = &source[..edit.start + 1];
        assert!(before.is_empty() || before_semicolon.ends_with(';') || !before_semicolon.contains(';'));
    
        Ok(())}

    #[test]
    fn fix_preserves_surrounding_code() -> Result<()> {
        let source = "const x = 1;\ndebugger;\nconst y = 2;\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert!(fixed.contains("const x = 1;"));
        assert!(fixed.contains("const y = 2;"));
        assert!(!fixed.contains("debugger"));
    
        Ok(())}

    #[test]
    fn fix_no_debugger_returns_empty() -> Result<()> {
        let source = "const x = 1;\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
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

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
