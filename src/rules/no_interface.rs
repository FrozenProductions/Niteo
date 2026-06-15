use std::collections::HashMap;
use std::path::Path;

use oxc_ast::ast::TSInterfaceDeclaration;
use oxc_ast_visit::Visit;

use crate::config::{NoInterfaceRuleConfig, Severity};
use crate::rules::{NO_INTERFACE_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Use a type alias instead of an interface.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoInterfaceRuleConfig,
) -> Vec<Violation> {
    let mut visitor = InterfaceVisitor {
        interfaces: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);

    if config.allow_declaration_merging {
        let name_counts = count_interface_names(&visitor.interfaces);
        visitor
            .interfaces
            .into_iter()
            .filter(|(name, _)| name_counts.get(name).copied().unwrap_or(0) <= 1)
            .map(|(_, span)| interface_violation(file, line_index, span, config.severity))
            .collect()
    } else {
        visitor
            .interfaces
            .into_iter()
            .map(|(_, span)| interface_violation(file, line_index, span, config.severity))
            .collect()
    }
}

struct InterfaceVisitor<'a> {
    interfaces: Vec<(String, oxc_span::Span)>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for InterfaceVisitor<'a> {
    fn visit_ts_interface_declaration(&mut self, decl: &TSInterfaceDeclaration<'a>) {
        let name = decl.id.name.to_string();
        self.interfaces.push((name, decl.span));
        oxc_ast_visit::walk::walk_ts_interface_declaration(self, decl);
    }
}

fn count_interface_names(interfaces: &[(String, oxc_span::Span)]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (name, _) in interfaces {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    counts
}

fn interface_violation(
    file: &Path,
    line_index: &LineIndex,
    span: oxc_span::Span,
    severity: Severity,
) -> Violation {
    let pos = line_index.position_for(span);
    Violation {
        file: file.to_path_buf(),
        line: Some(pos.line),
        column: Some(pos.column),
        rule: NO_INTERFACE_RULE_ID,
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
    use crate::config::{NoInterfaceRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn test_config() -> NoInterfaceRuleConfig {
        NoInterfaceRuleConfig {
            severity: Severity::Warn,
            allow_declaration_merging: true,
        }
    }

    fn strict_config() -> NoInterfaceRuleConfig {
        NoInterfaceRuleConfig {
            severity: Severity::Warn,
            allow_declaration_merging: false,
        }
    }

    fn run_check(source: &str, config: &NoInterfaceRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("types.ts"), &program, &line_index, config)
    }

    fn run_check_tsx(source: &str, config: &NoInterfaceRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(Path::new("component.tsx"), &program, &line_index, config)
    }

    #[test]
    fn reports_single_interface() -> Result<()> {
        let violations = run_check("interface User { name: string }\n", &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn allows_declaration_merging() -> Result<()> {
        let source = r#"interface User { name: string }
interface User { age: number }
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_all_interfaces_when_merging_disabled() -> Result<()> {
        let source = r#"interface User { name: string }
interface User { age: number }
"#;
        let violations = run_check(source, &strict_config());
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    
        Ok(())}

    #[test]
    fn reports_mixed_interfaces() -> Result<()> {
        let source = r#"interface User { name: string }
interface User { age: number }
interface Post { title: string }
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn ignores_interface_in_comments_and_strings() -> Result<()> {
        let source = r#"// interface User { name: string }
const text = "interface User";
/* interface Post { title: string } */
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragments() -> Result<()> {
        let source = r#"const interfacex = true;
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_interface_in_jsx_text() -> Result<()> {
        let source = r#"<p className="mt-1">
    Scale the full app interface for the current window.
</p>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_interface_in_jsx_text_with_bracketed_tailwind_class() -> Result<()> {
        let source = r#"<p className="mt-1 text-xs leading-[1.55] text-fumi-400">
    Scale the full app interface for the current window.
</p>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_interface_in_nested_jsx_text() -> Result<()> {
        let source = r#"<div>
    <p>The user interface is ready.</p>
    <span>interface keyword in text</span>
</div>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_interface_in_jsx_expression() -> Result<()> {
        let source = r#"<div>
    {user.name}
</div>

interface User { name: string }
"#;
        let violations = run_check_tsx(source, &strict_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(5));
    
        Ok(())}

    #[test]
    fn ignores_interface_in_jsx_attribute_values() -> Result<()> {
        let source = r#"<Component tooltip="This interface is deprecated" />
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn handles_jsx_fragments() -> Result<()> {
        let source = r#"<><p>interface text</p></>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn handles_jsx_with_expressions() -> Result<()> {
        let source = r#"<div>
    {user.name}
    <p>interface description</p>
    {count}
</div>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn handles_jsx_attribute_expression_with_nested_object() -> Result<()> {
        let source = r#"<Component
    options={{ label: "interface label", value: count }}
>
    interface text
</Component>
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_interface_after_jsx() -> Result<()> {
        let source = r#"const element = <p>interface text</p>;

interface User { name: string }
"#;
        let violations = run_check_tsx(source, &strict_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}
}
