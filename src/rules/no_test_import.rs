use std::path::Path;

use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, Expression, ImportDeclaration, ImportExpression,
    StringLiteral,
};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{NO_TEST_IMPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Production code may not import test files.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    tests_config: &DomainConfig,
) -> Vec<Violation> {
    if tests_config.matches_file(file) {
        return Vec::new();
    }

    let mut visitor = TestImportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        tests_config,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct TestImportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    tests_config: &'f DomainConfig,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for TestImportVisitor<'a, 'f> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        self.check_source(&decl.source, decl.span);
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        self.check_source(&decl.source, decl.span);
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
            self.check_source(source, decl.span);
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(source) = &expr.source {
            self.check_source(source, expr.span);
        }
        oxc_ast_visit::walk::walk_import_expression(self, expr);
    }
}

impl TestImportVisitor<'_, '_> {
    fn check_source(&mut self, source: &StringLiteral, span: oxc_span::Span) {
        if is_test_import(source.value.as_str(), self.tests_config) {
            let pos = self.line_index.position_for(span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_TEST_IMPORT_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: Some(format!("imports `{}`", source.value)),
                subject: Some(source.value.to_string()),
            });
        }
    }
}

fn is_test_import(specifier: &str, tests_config: &DomainConfig) -> bool {
    if specifier.is_empty() || !is_relative_or_local(specifier) {
        return false;
    }

    let segments: Vec<&str> = specifier.split('/').collect();

    for segment in &segments {
        if tests_config.folders.iter().any(|f| *segment == f.as_str()) {
            return true;
        }
    }

    let last = segments.last().copied().unwrap_or(specifier);
    tests_config.file_suffixes.iter().any(|suffix| {
        let stem_suffix = strip_extension(suffix);
        last.ends_with(suffix.as_str()) || last.ends_with(stem_suffix)
    })
}

fn is_relative_or_local(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}

fn strip_extension(suffix: &str) -> &str {
    for ext in &[".ts", ".tsx", ".js", ".jsx", ".mts", ".cts", ".mjs", ".cjs"] {
        if let Some(stripped) = suffix.strip_suffix(ext) {
            return stripped;
        }
    }
    suffix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn test_domain() -> DomainConfig {
        DomainConfig {
            folders: vec!["tests".to_string()],
            file_suffixes: vec![".test.ts".to_string(), ".tests.ts".to_string()],
        }
    }

    fn run_check(source: &str, file_path: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(
            Path::new(file_path),
            &program,
            &line_index,
            &test_config(),
            &test_domain(),
        )
    }

    #[test]
    fn reports_import_from_test_suffix() {
        let violations = run_check("import { helper } from './helper.test';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("./helper.test"));
    }

    #[test]
    fn reports_import_from_tests_suffix() {
        let violations = run_check("import { setup } from './setup.tests';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_import_from_tests_folder() {
        let violations = run_check("import { mock } from '../tests/mock';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_export_from_test_file() {
        let violations = run_check("export { helper } from './helper.test';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_export_all_from_test_file() {
        let violations = run_check("export * from './helpers.tests';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_dynamic_import_from_test_file() {
        let violations = run_check(
            "const helper = await import('./helper.test');",
            "src/auth.ts",
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_import_from_production_file() {
        let violations = run_check("import { auth } from './auth';", "src/service.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_imports_in_test_files() {
        let violations = run_check(
            "import { helper } from './helper.test';",
            "src/auth.test.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_imports_in_test_folder() {
        let violations = run_check("import { helper } from './helper.test';", "tests/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_package_imports() {
        let violations = run_check(
            "import { render } from '@testing-library/react';",
            "src/auth.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_bare_specifier_with_test_name() {
        let violations = run_check("import { test } from 'vitest';", "src/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_correct_line_position() {
        let source = "import { a } from './a';\nimport { b } from './b.test';\n";
        let violations = run_check(source, "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_import_with_ts_extension() {
        let violations = run_check("import { helper } from './helper.test.ts';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_export_named_without_source() {
        let violations = run_check("const x = 1; export { x };", "src/auth.ts");
        assert!(violations.is_empty());
    }
}
