use std::path::Path;

use oxc_ast::ast::ImportDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_SIDE_EFFECT_IMPORTS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Avoid side-effect imports; import named bindings or types instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = SideEffectImportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct SideEffectImportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for SideEffectImportVisitor<'a, 'f> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        if decl.specifiers.is_none() {
            let source = decl.source.value.as_str();
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(decl.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_SIDE_EFFECT_IMPORTS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: Some(format!("\"{source}\" is imported only for side effects")),
                subject: Some(source.to_string()),
            });
        }
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use super::*;
    use crate::config::{RuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

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
    fn reports_bare_css_import() -> Result<()> {
        let violations = run_check("import \"./styles.css\";\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].subject.as_deref(), Some("./styles.css"));
        Ok(())
    }

    #[test]
    fn reports_bare_package_import() -> Result<()> {
        let violations = run_check("import \"polyfill\";\n");
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn allows_named_imports() -> Result<()> {
        let violations = run_check("import { foo } from \"bar\";\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_default_import() -> Result<()> {
        let violations = run_check("import foo from \"bar\";\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_namespace_import() -> Result<()> {
        let violations = run_check("import * as foo from \"bar\";\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_empty_named_import_block() -> Result<()> {
        let violations = run_check("import {} from \"bar\";\n");
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_multiple_side_effect_imports() -> Result<()> {
        let source = "import \"./a.css\";\nimport \"./b.css\";\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
        Ok(())
    }
}
