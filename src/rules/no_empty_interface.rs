use std::path::Path;

use oxc_ast::ast::TSInterfaceDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_EMPTY_INTERFACE_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Use a type alias instead of an empty interface.";

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

    #[test]
    fn reports_empty_interface() {
        let violations = run_check("interface Empty {}\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_empty_interface_with_newline() {
        let source = "interface Empty {\n}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_empty_interface_with_whitespace() {
        let source = "interface Empty {   }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_empty_interfaces() {
        let source = r#"interface A {}
interface B {}
"#;
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    }

    #[test]
    fn allows_interface_with_members() {
        let source = "interface User { name: string }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_interface_with_comment_in_body() {
        let source = "interface User { /* todo */ name: string }\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_interface_in_comments_and_strings() {
        let source = r#"// interface Empty {}
const text = "interface Empty {}";
/* interface Empty {} */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragments() {
        let source = r#"const interfacex = true;
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
