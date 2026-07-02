use std::path::Path;

use oxc_ast::ast::{
    Declaration, ExportAllDeclaration, ExportDefaultDeclaration, ExportNamedDeclaration,
    ExportSpecifier, VariableDeclaration,
};
use oxc_ast_visit::Visit;

use crate::config::FileExportsRuleConfig;
use crate::rules::{MAX_FILE_EXPORTS_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Split this file or reduce its public surface area.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    _line_index: &LineIndex,
    config: &FileExportsRuleConfig,
) -> Vec<Violation> {
    let mut visitor = ExportCountVisitor {
        count: 0,
        count_default: config.count_default,
    };
    visitor.visit_program(program);

    if visitor.count <= config.max_exports {
        return Vec::new();
    }

    vec![Violation {
        file: file.to_path_buf(),
        span: None,
        line: Some(1),
        column: Some(1),
        rule: MAX_FILE_EXPORTS_RULE_ID,
        message: MESSAGE,
        severity: config.severity,
        detail: None,
        subject: None,
    }]
}

struct ExportCountVisitor {
    count: usize,
    count_default: bool,
}

impl<'a> Visit<'a> for ExportCountVisitor {
    fn visit_export_default_declaration(&mut self, _decl: &ExportDefaultDeclaration<'a>) {
        if self.count_default {
            self.count += 1;
        }
    }

    fn visit_export_all_declaration(&mut self, _decl: &ExportAllDeclaration<'a>) {
        self.count += 1;
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(Declaration::VariableDeclaration(var_decl)) = &decl.declaration {
            self.count += count_variable_bindings(var_decl);
        } else if decl.declaration.is_some() {
            self.count += 1;
        } else {
            self.count += count_named_specifiers(&decl.specifiers);
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }
}

fn count_variable_bindings(var_decl: &VariableDeclaration) -> usize {
    var_decl.declarations.len()
}

fn count_named_specifiers(specifiers: &[ExportSpecifier]) -> usize {
    specifiers.len()
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::{FileExportsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use anyhow::Result;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, max_exports: usize) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("dump.ts"),
            &program,
            &line_index,
            &test_config(max_exports),
        )
    }

    #[test]
    fn reports_files_with_too_many_named_export_declarations() -> Result<()> {
        let source = r#"export const one = 1;
export function two() {}
export class Three {}
export interface Four {}
"#;
        let violations = run_check(source, 3);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));

        Ok(())
    }

    #[test]
    fn counts_named_export_lists() -> Result<()> {
        let source = r#"const one = 1;
const two = 2;
const three = 3;
export { one, two as renamedTwo, type three };
"#;
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn counts_multiple_variable_exports() -> Result<()> {
        let violations = run_check("export const one = 1, two = 2, three = 3;\n", 2);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_files_within_limit() -> Result<()> {
        let source = r#"export const one = 1;
export { two, three };
"#;
        let violations = run_check(source, 3);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_exports_in_comments_and_strings() -> Result<()> {
        let source = r#"const text = "export const one = 1";
// export const two = 2;
/* export const three = 3; */
export const four = 4;
"#;
        let violations = run_check(source, 1);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn counts_default_and_namespace_exports() -> Result<()> {
        let source = r#"export default value;
export * from "./other";
export * as names from "./names";
"#;
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn counts_multiple_variable_declarations() -> Result<()> {
        let source = "export const one = 1, two = 2, three = 3, four = 4, five = 5;\n";
        let violations = run_check(source, 4);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn excludes_default_exports_when_count_default_is_false() -> Result<()> {
        let source = r#"export default value;
export const one = 1;
export const two = 2;
export const three = 3;
export const four = 4;
export const five = 5;
export const six = 6;
export const seven = 7;
export const eight = 8;
export const nine = 9;
"#;
        let config = FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports: 9,
            count_default: false,
        };
        let violations = run_check_with_config(source, config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn counts_default_exports_when_count_default_is_true() -> Result<()> {
        let source = r#"export default value;
export const one = 1;
export const two = 2;
export const three = 3;
export const four = 4;
export const five = 5;
export const six = 6;
export const seven = 7;
export const eight = 8;
export const nine = 9;
"#;
        let config = FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports: 9,
            count_default: true,
        };
        let violations = run_check_with_config(source, config);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn validates_when_default_export_pushes_over_limit() -> Result<()> {
        let source = r#"export default value;
export const one = 1;
export const two = 2;
"#;
        let config = FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports: 2,
            count_default: false,
        };
        let violations = run_check_with_config(source, config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn count_default_false_still_reports_named_over_limit() -> Result<()> {
        let source = r#"export default value;
export const one = 1;
export const two = 2;
export const three = 3;
"#;
        let config = FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports: 2,
            count_default: false,
        };
        let violations = run_check_with_config(source, config);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    fn test_config(max_exports: usize) -> FileExportsRuleConfig {
        FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports,
            count_default: true,
        }
    }

    fn run_check_with_config(source: &str, config: FileExportsRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("dump.ts"), &program, &line_index, &config)
    }
}
