use std::path::Path;

use oxc_ast::ast::{TSInterfaceDeclaration, TSTypeAliasDeclaration};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{NO_INLINE_TYPES_RULE_ID, TypeLocationStyle, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Move exported contracts to a colocated type file or accepted types folder.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    location_style: TypeLocationStyle,
    types: &DomainConfig,
) -> Vec<Violation> {
    if location_style.allows_file(file, types) {
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

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::{TypeLocationStyle, check_file};
    use crate::config::structure::{DomainConfig, ProjectStructureConfig};
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

    fn default_types() -> DomainConfig {
        ProjectStructureConfig::default().types
    }

    fn run_check(source: &str, file_path: &str, style: TypeLocationStyle) -> Vec<Violation> {
        let types = default_types();
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
            &types,
        )
    }

    #[test]
    fn reports_type_aliases_outside_type_files() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "type ButtonProps = { label: string };\n",
            "Button.tsx",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")], &types),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_interfaces_outside_type_files() -> Result<()> {
        let types = default_types();
        let source = r#"export interface ButtonProps {
  label: string;
}
"#;
        let violations = run_check(
            source,
            "Button.tsx",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")], &types),
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_type_declarations_in_type_files() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "export type ButtonProps = { label: string };\n",
            "Button.type.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")], &types),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_type_declarations_in_detected_types_directories() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "export interface ButtonProps {}\n",
            "types/Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")], &types),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_outside_detected_types_directories() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "interface ButtonProps {}\n",
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")], &types),
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn defaults_to_type_file_style_when_no_structure_exists() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "type ButtonProps = { label: string };\n",
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.ts")], &types),
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_imports_re_exports_comments_and_strings() -> Result<()> {
        let types = default_types();
        let source = r#"import type { ButtonProps } from "./Button.type";
export type { ButtonProps } from "./Button.type";
const text = "type ButtonProps = {}";
// interface ButtonProps {}
/* type ButtonProps = {} */
"#;
        let violations = run_check(
            source,
            "Button.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")], &types),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_declaration_files() -> Result<()> {
        let types = default_types();
        let violations = run_check(
            "interface Window { appVersion: string }\n",
            "global.d.ts",
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")], &types),
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn custom_type_folder() -> Result<()> {
        let types = DomainConfig {
            folders: vec!["typings".to_string()],
            file_suffixes: vec![".type.ts".to_string()],
        };
        let allocator = Allocator::default();
        let line_index = LineIndex::new("export type Foo = string;\n");
        let parser_return =
            Parser::new(&allocator, "export type Foo = string;\n", SourceType::ts()).parse();
        let program = parser_return.program;
        let style = TypeLocationStyle::detect(&[PathBuf::from("typings/Foo.ts")], &types);
        let violations = check_file(
            Path::new("typings/Foo.ts"),
            &program,
            &line_index,
            &test_config(),
            style,
            &types,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn custom_type_suffix() -> Result<()> {
        let types = DomainConfig {
            folders: vec!["types".to_string()],
            file_suffixes: vec![".types.ts".to_string()],
        };
        let allocator = Allocator::default();
        let line_index = LineIndex::new("export type Foo = string;\n");
        let parser_return =
            Parser::new(&allocator, "export type Foo = string;\n", SourceType::ts()).parse();
        let program = parser_return.program;
        let style = TypeLocationStyle::detect(&[PathBuf::from("Foo.types.ts")], &types);
        let violations = check_file(
            Path::new("Foo.types.ts"),
            &program,
            &line_index,
            &test_config(),
            style,
            &types,
        );
        assert!(violations.is_empty());
    
        Ok(())}
}
