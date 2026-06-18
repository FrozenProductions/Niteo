use std::collections::HashMap;
use std::path::Path;

use oxc_ast::ast::TSInterfaceDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{Fix, NO_EMPTY_INTERFACE_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Use a type alias instead of an empty interface.";
const INTERFACE_KEYWORD_LEN: usize = 9;

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = EmptyInterfaceVisitor {
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

    if is_declaration_file(file) {
        return Vec::new();
    }

    let mut collector = EmptyInterfaceCollector {
        interfaces: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);

    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for info in &collector.interfaces {
        *name_counts.entry(info.name.clone()).or_default() += 1;
    }

    let mut edits: Vec<TextEdit> = Vec::new();
    for info in &collector.interfaces {
        if !info.is_empty || info.has_extends || info.is_declare {
            continue;
        }
        if *name_counts.get(&info.name).unwrap_or(&0) > 1 {
            continue;
        }
        if !is_body_effectively_empty(source, info.body_span) {
            continue;
        }

        let keyword_end = info.decl_start + INTERFACE_KEYWORD_LEN as u32;
        edits.push(crate::fix::span_edit(
            info.decl_start as usize,
            keyword_end as usize,
            "type",
        ));

        let body_end = crate::fix::extend_end_through_optional_semicolon(
            source,
            info.body_span.end as usize,
        );
        edits.push(crate::fix::span_edit(
            info.body_span.start as usize,
            body_end,
            "= Record<string, never>;",
        ));
    }

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: NO_EMPTY_INTERFACE_RULE_ID,
            edits,
        }]
    }
}

fn is_declaration_file(file: &Path) -> bool {
    file.to_string_lossy().ends_with(".d.ts")
}

fn is_body_effectively_empty(source: &str, body_span: oxc_span::Span) -> bool {
    let inner_start = (body_span.start as usize + 1).min(source.len());
    let inner_end = (body_span.end as usize).saturating_sub(1).min(source.len());
    if inner_start >= inner_end {
        return true;
    }
    let inner = source.get(inner_start..inner_end).unwrap_or("");
    inner.chars().all(|char| char.is_whitespace())
}

struct EmptyInterfaceInfo {
    name: String,
    is_empty: bool,
    has_extends: bool,
    is_declare: bool,
    decl_start: u32,
    body_span: oxc_span::Span,
}

struct EmptyInterfaceCollector<'a> {
    interfaces: Vec<EmptyInterfaceInfo>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for EmptyInterfaceCollector<'a> {
    fn visit_ts_interface_declaration(&mut self, decl: &TSInterfaceDeclaration<'a>) {
        self.interfaces.push(EmptyInterfaceInfo {
            name: decl.id.name.to_string(),
            is_empty: decl.body.body.is_empty(),
            has_extends: !decl.extends.is_empty(),
            is_declare: decl.declare,
            decl_start: decl.span.start,
            body_span: decl.body.span,
        });
        oxc_ast_visit::walk::walk_ts_interface_declaration(self, decl);
    }
}

struct EmptyInterfaceVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for EmptyInterfaceVisitor<'a, 'f> {
    fn visit_ts_interface_declaration(&mut self, decl: &TSInterfaceDeclaration<'a>) {
        if decl.body.body.is_empty() {
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(decl.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_EMPTY_INTERFACE_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_ts_interface_declaration(self, decl);
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
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("types.ts"), &program, &line_index, &test_config())
    }

    fn run_fix_with_path(source: &str, path: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        fix_file(Path::new(path), &program, source, &test_config())
    }

    fn run_fix(source: &str) -> Vec<Fix> {
        run_fix_with_path(source, "types.ts")
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
    fn reports_empty_interface() -> Result<()> {
        let violations = run_check("interface Empty {}\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_empty_interface_with_newline() -> Result<()> {
        let source = "interface Empty {\n}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_empty_interface_with_whitespace() -> Result<()> {
        let source = "interface Empty {   }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_multiple_empty_interfaces() -> Result<()> {
        let source = r#"interface A {}
interface B {}
"#;
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    
        Ok(())}

    #[test]
    fn allows_interface_with_members() -> Result<()> {
        let source = "interface User { name: string }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_interface_with_comment_in_body() -> Result<()> {
        let source = "interface User { /* todo */ name: string }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_interface_in_comments_and_strings() -> Result<()> {
        let source = r#"// interface Empty {}
const text = "interface Empty {}";
/* interface Empty {} */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragments() -> Result<()> {
        let source = r#"const interfacex = true;
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn fix_converts_basic_empty_interface() -> Result<()> {
        let source = "interface Empty {}\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "type Empty = Record<string, never>;\n");
    
        Ok(())}

    #[test]
    fn fix_converts_exported_empty_interface() -> Result<()> {
        let source = "export interface Empty {}\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "export type Empty = Record<string, never>;\n");
    
        Ok(())}

    #[test]
    fn fix_does_not_convert_interface_with_extends() -> Result<()> {
        let source = "interface Empty extends Base {}\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_does_not_convert_declaration_file() -> Result<()> {
        let source = "interface Empty {}\n";
        let edits = run_fix_with_path(source, "types.d.ts");
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_does_not_convert_declared_interface() -> Result<()> {
        let source = "declare interface Empty {}\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_does_not_convert_merged_interface() -> Result<()> {
        let source = "interface Empty {}\ninterface Empty { name: string }\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_does_not_convert_interface_with_comment_in_body() -> Result<()> {
        let source = "interface Empty { /* todo */ }\n";
        let edits = run_fix(source);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fix_converts_multiple_empty_interfaces() -> Result<()> {
        let source = "interface A {}\ninterface B {}\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        assert_eq!(fixed, "type A = Record<string, never>;\ntype B = Record<string, never>;\n");
    
        Ok(())}

    #[test]
    fn fix_disabled_returns_empty() -> Result<()> {
        let source = "interface Empty {}\n";
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let config = RuleConfig {
            severity: Severity::Off,
        };
        let edits = fix_file(Path::new("types.ts"), &program, source, &config);
        assert!(edits.is_empty());
    
        Ok(())}

    #[test]
    fn fixed_source_parses() -> Result<()> {
        let source = "interface Empty {}\n";
        let edits = run_fix(source);
        let fixed = apply_fix_edits(source, &edits);
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &fixed, SourceType::ts()).parse();
        assert!(!parser_return.panicked);
    
        Ok(())}
}
