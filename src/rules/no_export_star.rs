use std::path::Path;

use oxc_ast::ast::ExportAllDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_EXPORT_STAR_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str =
    "Avoid export * because it hides the public API shape. Use explicit named re-exports.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = ExportStarVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ExportStarVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ExportStarVisitor<'a, 'f> {
    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        if decl.exported.is_none() {
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_EXPORT_STAR_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: None,
            });
        }
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
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
        check_file(Path::new("index.ts"), &program, &line_index, &test_config())
    }

    #[test]
    fn reports_export_star() -> Result<()> {
        let violations = run_check("export * from './module';\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_multiline_export_star() -> Result<()> {
        let violations = run_check("export\n  * from './module';\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn allows_named_re_exports() -> Result<()> {
        let violations = run_check("export { foo, bar } from './module';\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_namespace_re_exports() -> Result<()> {
        let violations = run_check("export * as utils from './utils';\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_export_star_in_comments_and_strings() -> Result<()> {
        let source = r#"// export * from './module';
const text = "export * from './module'";
/* export * from './module'; */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
