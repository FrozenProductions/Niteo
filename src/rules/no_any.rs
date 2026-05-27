use std::path::Path;

use oxc_ast::ast::TSAnyKeyword;
use oxc_ast_visit::Visit;

use crate::config::NoAnyRuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{NO_ANY_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Avoid using `any`. Use a more specific type or `unknown` instead.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoAnyRuleConfig,
    generated: &DomainConfig,
) -> Vec<Violation> {
    if is_file_allowed(file, config, generated) {
        return Vec::new();
    }

    let mut visitor = AnyKeywordVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

fn is_file_allowed(file: &Path, config: &NoAnyRuleConfig, generated: &DomainConfig) -> bool {
    if generated.matches_file(file) {
        return true;
    }

    let file_str = file.to_string_lossy();
    config
        .allowed_folders
        .iter()
        .any(|folder| file_str.contains(folder))
}

struct AnyKeywordVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for AnyKeywordVisitor<'a, 'f> {
    fn visit_ts_any_keyword(&mut self, keyword: &TSAnyKeyword) {
        let pos = self.line_index.position_for(keyword.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_ANY_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_ts_any_keyword(self, keyword);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{NoAnyRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        run_check_with_config(source, &test_config(), &default_generated())
    }

    fn run_check_with_config(
        source: &str,
        config: &NoAnyRuleConfig,
        generated: &DomainConfig,
    ) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("Component.tsx"),
            &program,
            &line_index,
            config,
            generated,
        )
    }

    fn test_config() -> NoAnyRuleConfig {
        NoAnyRuleConfig {
            severity: Severity::Warn,
            allowed_folders: vec![],
        }
    }

    fn default_generated() -> DomainConfig {
        DomainConfig {
            folders: vec!["generated".to_string(), "__generated__".to_string()],
            file_suffixes: vec![".generated.ts".to_string(), ".generated.tsx".to_string()],
        }
    }

    #[test]
    fn reports_explicit_any_type_annotation() {
        let violations = run_check("const value: any = 'test';\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_any_in_function_parameter() {
        let violations = run_check("function foo(arg: any) {}\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_function_return_type() {
        let violations = run_check("function foo(): any { return null; }\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_type_assertion() {
        let violations = run_check("const value = obj as any;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_generic_type_parameter() {
        let violations = run_check("const arr: Array<any> = [];\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_interface_property() {
        let violations = run_check("interface Foo { bar: any; }\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_type_alias() {
        let violations = run_check("type Foo = any;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_any_usages() {
        let violations = run_check("const a: any = 1; const b: any = 2;\n");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn allows_unknown_type() {
        let violations = run_check("const value: unknown = 'test';\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_specific_types() {
        let violations = run_check("const value: string = 'test';\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_any_in_generated_folder() {
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("src/generated/types.ts"),
            &program,
            &line_index,
            &test_config(),
            &default_generated(),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_any_in_generated_file_suffix() {
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("src/api.generated.ts"),
            &program,
            &line_index,
            &test_config(),
            &default_generated(),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_any_in_custom_allowed_folder() {
        let config = NoAnyRuleConfig {
            severity: Severity::Warn,
            allowed_folders: vec!["legacy".to_string()],
        };
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("src/legacy/oldCode.ts"),
            &program,
            &line_index,
            &config,
            &default_generated(),
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_any_in_comments() {
        let source = "// const value: any = 'test';\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_any_in_strings() {
        let source = r#"const text = "const value: any = 'test';";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_record_with_unknown() {
        let violations = run_check("const obj: Record<string, unknown> = {};\n");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_any_in_union_type() {
        let violations = run_check("type Foo = string | any;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_intersection_type() {
        let violations = run_check("type Foo = object & any;\n");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_any_in_conditional_type() {
        let violations = run_check("type Foo<T> = T extends any ? T : never;\n");
        assert_eq!(violations.len(), 1);
    }
}
