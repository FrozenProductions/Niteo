use std::path::Path;

use oxc_ast::ast::TSAnyKeyword;
use oxc_ast_visit::Visit;

use crate::config::NoAnyRuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{Fix, NO_ANY_RULE_ID, TextEdit, Violation};
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

pub fn fix_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    _source: &str,
    config: &NoAnyRuleConfig,
    generated: &DomainConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() || is_file_allowed(file, config, generated) {
        return Vec::new();
    }

    let mut collector = AnyKeywordCollector {
        spans: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);

    let edits: Vec<TextEdit> = collector
        .spans
        .iter()
        .map(|span| TextEdit {
            start: span.start as usize,
            end: span.end as usize,
            replacement: "unknown".to_string(),
        })
        .collect();

    if edits.is_empty() {
        Vec::new()
    } else {
        vec![Fix {
            file: file.to_path_buf(),
            rule: NO_ANY_RULE_ID,
            edits,
        }]
    }
}

struct AnyKeywordCollector<'a> {
    spans: Vec<oxc_span::Span>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for AnyKeywordCollector<'a> {
    fn visit_ts_any_keyword(&mut self, keyword: &TSAnyKeyword) {
        self.spans.push(keyword.span);
        oxc_ast_visit::walk::walk_ts_any_keyword(self, keyword);
    }
}

fn is_file_allowed(file: &Path, config: &NoAnyRuleConfig, generated: &DomainConfig) -> bool {
    if generated.matches_file(file) {
        return true;
    }

    file.components().any(|component| {
        matches!(
            component,
            std::path::Component::Normal(name)
                if config.allowed_folders.iter().any(|folder| name.to_str() == Some(folder))
        )
    })
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
            span: Some(keyword.span),
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

    use anyhow::Result;
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
    fn reports_explicit_any_type_annotation() -> Result<()> {
        let violations = run_check("const value: any = 'test';\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_any_in_function_parameter() -> Result<()> {
        let violations = run_check("function foo(arg: any) {}\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_function_return_type() -> Result<()> {
        let violations = run_check("function foo(): any { return null; }\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_type_assertion() -> Result<()> {
        let violations = run_check("const value = obj as any;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_generic_type_parameter() -> Result<()> {
        let violations = run_check("const arr: Array<any> = [];\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_interface_property() -> Result<()> {
        let violations = run_check("interface Foo { bar: any; }\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_type_alias() -> Result<()> {
        let violations = run_check("type Foo = any;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_multiple_any_usages() -> Result<()> {
        let violations = run_check("const a: any = 1; const b: any = 2;\n");
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn allows_unknown_type() -> Result<()> {
        let violations = run_check("const value: unknown = 'test';\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_specific_types() -> Result<()> {
        let violations = run_check("const value: string = 'test';\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_any_in_generated_folder() -> Result<()> {
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
    
        Ok(())}

    #[test]
    fn allows_any_in_generated_file_suffix() -> Result<()> {
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
    
        Ok(())}

    #[test]
    fn allows_any_in_custom_allowed_folder() -> Result<()> {
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
    
        Ok(())}

    #[test]
    fn reports_any_in_folder_containing_allowed_name_as_substring() -> Result<()> {
        let config = NoAnyRuleConfig {
            severity: Severity::Warn,
            allowed_folders: vec!["api".to_string()],
        };
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;

        let violations = check_file(
            Path::new("src/api-legacy/foo.ts"),
            &program,
            &line_index,
            &config,
            &default_generated(),
        );
        assert_eq!(violations.len(), 1);

        Ok(())}

    #[test]
    fn reports_any_in_file_with_folder_in_parent_name() -> Result<()> {
        let config = NoAnyRuleConfig {
            severity: Severity::Warn,
            allowed_folders: vec!["api".to_string()],
        };
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;

        let violations = check_file(
            Path::new("src/blog/api-helpers.ts"),
            &program,
            &line_index,
            &config,
            &default_generated(),
        );
        assert_eq!(violations.len(), 1);

        Ok(())}

    #[test]
    fn allows_any_in_exact_allowed_folder() -> Result<()> {
        let config = NoAnyRuleConfig {
            severity: Severity::Warn,
            allowed_folders: vec!["api".to_string()],
        };
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;

        let violations = check_file(
            Path::new("src/api/client.ts"),
            &program,
            &line_index,
            &config,
            &default_generated(),
        );
        assert!(violations.is_empty());

        Ok(())}

    #[test]
    fn ignores_any_in_comments() -> Result<()> {
        let source = "// const value: any = 'test';\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_any_in_strings() -> Result<()> {
        let source = r#"const text = "const value: any = 'test';";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_record_with_unknown() -> Result<()> {
        let violations = run_check("const obj: Record<string, unknown> = {};\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_any_in_union_type() -> Result<()> {
        let violations = run_check("type Foo = string | any;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_intersection_type() -> Result<()> {
        let violations = run_check("type Foo = object & any;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_any_in_conditional_type() -> Result<()> {
        let violations = run_check("type Foo<T> = T extends any ? T : never;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    fn run_fix(source: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        fix_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &test_config(),
            &default_generated(),
        )
    }

    fn apply_fix_edits(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|fix| fix.edits.clone()).collect();
        crate::fix::apply_edits(source, &edits)
    }

    #[test]
    fn fix_replaces_any_with_unknown() -> Result<()> {
        let source = "const value: any = 'test';\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const value: unknown = 'test';\n");
    
        Ok(())}

    #[test]
    fn fix_replaces_multiple_any_usages() -> Result<()> {
        let source = "const a: any = 1; const b: any = 2;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const a: unknown = 1; const b: unknown = 2;\n");
    
        Ok(())}

    #[test]
    fn fix_replaces_any_in_function_param() -> Result<()> {
        let source = "function foo(arg: any) {}\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "function foo(arg: unknown) {}\n");
    
        Ok(())}

    #[test]
    fn fix_replaces_any_in_generic() -> Result<()> {
        let source = "const arr: Array<any> = [];\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "const arr: Array<unknown> = [];\n");
    
        Ok(())}

    #[test]
    fn fix_replaces_any_in_type_alias() -> Result<()> {
        let source = "type Foo = any;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        assert_eq!(fixed, "type Foo = unknown;\n");
    
        Ok(())}

    #[test]
    fn fix_disabled_returns_empty() -> Result<()> {
        let source = "const value: any = 'test';\n";
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let disabled_config = NoAnyRuleConfig {
            severity: Severity::Off,
            allowed_folders: vec![],
        };
        let fixes = fix_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &disabled_config,
            &default_generated(),
        );
        assert!(fixes.is_empty());
    
        Ok(())}

    #[test]
    fn fix_skips_generated_files() -> Result<()> {
        let allocator = Allocator::default();
        let source = "const value: any = 'test';\n";
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let fixes = fix_file(
            Path::new("src/generated/types.ts"),
            &program,
            source,
            &test_config(),
            &default_generated(),
        );
        assert!(fixes.is_empty());
    
        Ok(())}

    #[test]
    fn fixed_source_parses() -> Result<()> {
        let source = "const value: any = 'test';\n";
        let fixes = run_fix(source);
        let fixed = apply_fix_edits(source, &fixes);
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, &fixed, SourceType::tsx()).parse();
        assert!(!parser_return.panicked);
    
        Ok(())}
}
