use std::path::Path;

use oxc_ast::ast::{Declaration, ExportNamedDeclaration, VariableDeclarationKind};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_MUTABLE_EXPORTS_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Only export const, never export let.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = MutableExportsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct MutableExportsVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for MutableExportsVisitor<'a, 'f> {
    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(Declaration::VariableDeclaration(var_decl)) = &decl.declaration
            && matches!(
                var_decl.kind,
                VariableDeclarationKind::Let | VariableDeclarationKind::Var
            )
        {
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_MUTABLE_EXPORTS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
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
        check_file(Path::new("value.ts"), &program, &line_index, &test_config())
    }

    #[test]
    fn reports_export_let() -> Result<()> {
        let violations = run_check("export let count = 0;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_multiline_export_let() -> Result<()> {
        let violations = run_check("export\n  let count = 0;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_export_var() -> Result<()> {
        let violations = run_check("export var count = 0;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_export_const() -> Result<()> {
        let violations = run_check("export const count = 0;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_named_function_export() -> Result<()> {
        let violations = run_check("export function foo() {}\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_export_let_in_comments_and_strings() -> Result<()> {
        let source = r#"// export let count = 0;
const text = "export let count = 0";
/* export let count = 0; */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_export_letting() -> Result<()> {
        let source = "export letting foo = 1;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
