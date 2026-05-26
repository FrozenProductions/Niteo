use std::path::Path;

use oxc_ast::ast::ExportDefaultDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_DEFAULT_EXPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Use named exports so imports stay explicit and refactorable.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = DefaultExportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct DefaultExportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for DefaultExportVisitor<'a, 'f> {
    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_DEFAULT_EXPORT_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {
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
    fn reports_default_function_export() {
        let violations = run_check("export default function Component() {}\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_default_value_export() {
        let violations = run_check("const value = 1;\nexport default value;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_multiline_default_export() {
        let violations = run_check("export\n  default value;\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_named_exports() {
        let violations =
            run_check("export function Component() {}\nexport { value } from './value';\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_export_default_in_comments_and_strings() {
        let source = r#"// export default value;
const text = "export default value";
/* export default value; */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragments() {
        let source = r#"const exportDefault = true;
const value = "before export default after";
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
