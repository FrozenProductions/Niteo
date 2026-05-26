use std::path::{Component, Path, PathBuf};

use oxc_ast::ast::{TSInterfaceDeclaration, TSTypeAliasDeclaration};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{NO_INLINE_TYPES_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Move exported contracts to a colocated type file or accepted types folder.";
const TYPES_DIRECTORY_NAME: &str = "types";
const TYPE_FILE_SUFFIX: &str = ".type.ts";
const DECLARATION_FILE_SUFFIX: &str = ".d.ts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeLocationStyle {
    allows_type_files: bool,
    allows_types_directories: bool,
}

impl TypeLocationStyle {
    pub fn detect(files: &[PathBuf]) -> Self {
        let allows_type_files = files.iter().any(|file| is_type_file(file));
        let allows_types_directories = files.iter().any(|file| is_in_types_directory(file));

        Self {
            allows_type_files: allows_type_files || !allows_types_directories,
            allows_types_directories,
        }
    }

    fn allows_file(self, file: &Path) -> bool {
        is_declaration_file(file)
            || (self.allows_type_files && is_type_file(file))
            || (self.allows_types_directories && is_in_types_directory(file))
    }
}

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    location_style: TypeLocationStyle,
) -> Vec<Violation> {
    if location_style.allows_file(file) {
        return Vec::new();
    }

    let mut visitor = InlineTypesVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct InlineTypesVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for InlineTypesVisitor<'a, 'f> {
    fn visit_ts_type_alias_declaration(&mut self, decl: &TSTypeAliasDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_INLINE_TYPES_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_type_alias_declaration(self, decl);
    }

    fn visit_ts_interface_declaration(&mut self, decl: &TSInterfaceDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_INLINE_TYPES_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_interface_declaration(self, decl);
    }
}

fn is_type_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(TYPE_FILE_SUFFIX))
}

fn is_declaration_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(DECLARATION_FILE_SUFFIX))
}

fn is_in_types_directory(file: &Path) -> bool {
    file.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if name == TYPES_DIRECTORY_NAME
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{TypeLocationStyle, check_file};
    use crate::config::{RuleConfig, Severity};
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::{Path, PathBuf};

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn run_check(source: &str, file_path: &str, style: TypeLocationStyle) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new(file_path),
            &program,
            &line_index,
            &test_config(),
            style,
        )
    }

    #[test]
    fn reports_type_aliases_outside_type_files() {
        let violations = run_check(
            "type ButtonProps = { label: string };\n",
            "Button.tsx",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_interfaces_outside_type_files() {
        let source = r#"export interface ButtonProps {
  label: string;
}
"#;
        let violations = run_check(
            source,
            "Button.tsx",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_type_declarations_in_type_files() {
        let violations = run_check(
            "export type ButtonProps = { label: string };\n",
            "Button.type.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_declarations_in_detected_types_directories() {
        let violations = run_check(
            "export interface ButtonProps {}\n",
            "types/Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")]),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_outside_detected_types_directories() {
        let violations = run_check(
            "interface ButtonProps {}\n",
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")]),
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn defaults_to_type_file_style_when_no_structure_exists() {
        let violations = run_check(
            "type ButtonProps = { label: string };\n",
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.ts")]),
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_imports_re_exports_comments_and_strings() {
        let source = r#"import type { ButtonProps } from "./Button.type";
export type { ButtonProps } from "./Button.type";
const text = "type ButtonProps = {}";
// interface ButtonProps {}
/* type ButtonProps = {} */
"#;
        let violations = run_check(
            source,
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_declaration_files() {
        let violations = run_check(
            "interface Window { appVersion: string }\n",
            "global.d.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );
        assert!(violations.is_empty());
    }
}
