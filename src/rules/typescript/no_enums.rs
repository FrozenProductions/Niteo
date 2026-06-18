use std::path::Path;

use oxc_ast::ast::TSEnumDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_ENUMS_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Use union types or const objects instead of enums.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = EnumVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct EnumVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for EnumVisitor<'a, 'f> {
    fn visit_ts_enum_declaration(&mut self, decl: &TSEnumDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(decl.span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_ENUMS_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_enum_declaration(self, decl);
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

    #[test]
    fn reports_enum_declaration() -> Result<()> {
        let violations = run_check("enum Status { Active, Inactive }\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_const_enum() -> Result<()> {
        let violations = run_check("const enum Color { Red, Green, Blue }\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_multiple_enums() -> Result<()> {
        let source = r#"enum A { X }
enum B { Y }
"#;
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    
        Ok(())}

    #[test]
    fn ignores_enum_in_comments_and_strings() -> Result<()> {
        let source = r#"// enum Status { Active }
const text = "enum Status";
/* enum Color { Red } */
"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragments() -> Result<()> {
        let source = r#"const enumeration = true;
const value = "before enum after";
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
