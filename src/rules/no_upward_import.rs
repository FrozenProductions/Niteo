use std::path::Path;

use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, Expression, ImportDeclaration, ImportExpression,
    StringLiteral,
};
use oxc_ast_visit::Visit;

use crate::config::{Severity, UpwardImportRuleConfig};
use crate::rules::{NO_UPWARD_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Replace upward relative imports with local or project-root imports.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &UpwardImportRuleConfig,
) -> Vec<Violation> {
    let mut visitor = UpwardImportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        config,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct UpwardImportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    config: &'f UpwardImportRuleConfig,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for UpwardImportVisitor<'a, 'f> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        if should_report(&decl.source, self.config) {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                decl.span,
                self.config.severity,
            ));
        }
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        if should_report(&decl.source, self.config) {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                decl.span,
                self.config.severity,
            ));
        }
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source
            && should_report(source, self.config)
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                decl.span,
                self.config.severity,
            ));
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(source) = &expr.source
            && should_report(source, self.config)
        {
            self.violations.push(make_violation(
                self.file,
                self.line_index,
                expr.span,
                self.config.severity,
            ));
        }
        oxc_ast_visit::walk::walk_import_expression(self, expr);
    }
}

fn should_report(source: &StringLiteral, config: &UpwardImportRuleConfig) -> bool {
    upward_depth(source.value.as_bytes()) > config.max_depth
}

fn upward_depth(specifier: &[u8]) -> usize {
    specifier
        .split(|byte| *byte == b'/')
        .take_while(|segment| *segment == b"..")
        .count()
}

fn make_violation(
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
        rule: NO_UPWARD_IMPORT_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Severity, UpwardImportRuleConfig};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn test_config() -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            severity: Severity::Warn,
            max_depth: 0,
        }
    }

    fn test_config_with_depth(max_depth: usize) -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            max_depth,
            ..test_config()
        }
    }

    fn run_check(source: &str, config: &UpwardImportRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("Button.ts"), &program, &line_index, config)
    }

    #[test]
    fn reports_upward_relative_imports() {
        let source = r#"import { shared } from "../../../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_upward_relative_export_from() {
        let source = r#"export { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_dynamic_upward_imports() {
        let source = r#"const shared = await import("../../shared");
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn keeps_line_positions_after_multiline_imports() {
        let source = r#"import {
  local,
} from "./local";
import { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(4));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_same_folder_and_downward_imports() {
        let source = r#"import { value } from "./value";
export { other } from "./other";
const shared = import("shared");
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_treat_export_default_as_export_from() {
        let source = r#"export default function Component() {}
import { shared } from "../shared";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn allows_configured_upward_depth() {
        let source = r#"import { shared } from "../shared";
import { other } from "../../other";
"#;
        let violations = run_check(source, &test_config_with_depth(1));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn ignores_comments_and_strings() {
        let source = r#"// import { shared } from "../shared";
const text = "export { shared } from '../shared'";
/* import x from "../shared" */
"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_export_all_upward() {
        let source = r#"export * from "../other";
"#;
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }
}
