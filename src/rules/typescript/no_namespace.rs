use std::path::Path;

use oxc_ast::ast::{TSModuleDeclaration, TSModuleDeclarationKind};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_NAMESPACE_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Use ES modules instead of TypeScript namespaces.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = NamespaceVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NamespaceVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for NamespaceVisitor<'a, 'f> {
    fn visit_ts_module_declaration(&mut self, decl: &TSModuleDeclaration<'a>) {
        if decl.kind == TSModuleDeclarationKind::Namespace {
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_NAMESPACE_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_ts_module_declaration(self, decl);
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
        check_file(Path::new("lib.ts"), &program, &line_index, &test_config())
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_namespace_declaration() -> Result<()> {
        let violations = run_check("namespace Foo {}\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn does_not_report_module_declaration() -> Result<()> {
        let violations = run_check("declare module 'foo' {}\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_namespace_with_export() -> Result<()> {
        let violations = run_check("export namespace Utils {}\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_nested_namespaces() -> Result<()> {
        let violations = run_check(
            r#"namespace Outer {
    namespace Inner {}
}
"#,
        );
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn does_not_report_type_only_import() -> Result<()> {
        let source = r#"
import type { ComponentType } from 'react';

const Example: ComponentType = () => null;
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}
}
