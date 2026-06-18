use std::path::Path;

use oxc_ast::ast::{CallExpression, Expression, ImportDeclaration};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{NO_TEST_CODE_IN_PRODUCTION_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Disallow test code (describe, it, test, expect, test library imports) outside test files.";

const TEST_GLOBALS: &[&str] = &[
    "describe",
    "it",
    "test",
    "expect",
    "beforeEach",
    "afterEach",
    "beforeAll",
    "afterAll",
];

const TEST_LIBRARY_SOURCES: &[&str] = &[
    "jest",
    "vitest",
    "mocha",
    "@testing-library",
    "@jest",
    "@vitest",
    "chai",
    "sinon",
    "cypress",
    "playwright",
    "@playwright/test",
];

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

    let mut visitor = TestCodeVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct TestCodeVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for TestCodeVisitor<'a, 'f> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(id) = &call.callee
            && TEST_GLOBALS.contains(&id.name.as_str())
        {
            let pos = self.line_index.position_for(id.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(id.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_TEST_CODE_IN_PRODUCTION_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: Some(format!("test global `{}`", id.name)),
                subject: Some(id.name.to_string()),
            });
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.as_str();
        if is_test_library_source(source) {
            let pos = self.line_index.position_for(decl.source.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(decl.source.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_TEST_CODE_IN_PRODUCTION_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: Some(format!("test library import `{}`", source)),
                subject: Some(source.to_string()),
            });
        }
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }
}

fn is_test_library_source(source: &str) -> bool {
    TEST_LIBRARY_SOURCES
        .iter()
        .any(|lib| source == *lib || source.starts_with(&format!("{}/", lib)))
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::structure::DomainConfig;
    use crate::config::{RuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

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

    #[test]
    fn reports_test_globals() -> Result<()> {
        for (source, expected_subject) in [
            ("describe('suite', () => {});", "describe"),
            ("it('works', () => {});", "it"),
            ("test('works', () => {});", "test"),
            ("expect(1).toBe(1);", "expect"),
            ("beforeEach(() => {});", "beforeEach"),
        ] {
            let violations = run_check(source, "src/auth.ts");
            assert_eq!(violations.len(), 1, "expected 1 violation for: {source:?}");
            assert_eq!(
                violations[0].subject.as_deref(),
                Some(expected_subject),
                "wrong subject for: {source:?}",
            );
        }
    
        Ok(())}

    #[test]
    fn reports_jest_import() -> Result<()> {
        let violations = run_check("import { jest } from 'jest';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("jest"));
    
        Ok(())}

    #[test]
    fn reports_vitest_import() -> Result<()> {
        let violations = run_check("import { describe } from 'vitest';", "src/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("vitest"));
    
        Ok(())}

    #[test]
    fn reports_testing_library_import() -> Result<()> {
        let violations = run_check(
            "import { render } from '@testing-library/react';",
            "src/auth.ts",
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_test_code_in_test_files_by_suffix() -> Result<()> {
        let violations = run_check(
            "describe('suite', () => { it('works', () => {}); });",
            "src/auth.test.ts",
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_test_code_in_test_files_by_folder() -> Result<()> {
        let violations = run_check("describe('suite', () => {});", "tests/auth.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_non_test_code_in_production() -> Result<()> {
        let source = "export function add(a: number, b: number) { return a + b; }";
        let violations = run_check(source, "src/math.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_non_test_imports() -> Result<()> {
        let source = "import { useState } from 'react';";
        let violations = run_check(source, "src/Component.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_partial_identifiers() -> Result<()> {
        let source = "const describe_something = true; const expectError = false;";
        let violations = run_check(source, "src/auth.ts");
        assert!(violations.is_empty());
    
        Ok(())}
}
