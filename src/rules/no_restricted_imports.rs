use std::path::Path;

use oxc_ast::ast::{ExportAllDeclaration, ExportNamedDeclaration, ImportDeclaration};
use oxc_ast_visit::Visit;

use crate::config::NoRestrictedImportsRuleConfig;
use crate::rules::{NO_RESTRICTED_IMPORTS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import from a restricted package or path.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoRestrictedImportsRuleConfig,
) -> Vec<Violation> {
    if config.restricted.is_empty() {
        return Vec::new();
    }

    let mut visitor = RestrictedImportsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        restricted: &config.restricted,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct RestrictedImportsVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    restricted: &'f [String],
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl RestrictedImportsVisitor<'_, '_> {
    fn is_restricted(&self, source: &str) -> bool {
        self.restricted.iter().any(|pattern| {
            source == pattern || source.starts_with(&format!("{pattern}/"))
        })
    }

    fn report(&mut self, span: oxc_span::Span, source: &str) {
        let pos = self.line_index.position_for(span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_RESTRICTED_IMPORTS_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: Some(format!("\"{source}\" matches a restricted pattern")),
            subject: Some(source.to_string()),
        });
    }
}

impl<'a, 'f> Visit<'a> for RestrictedImportsVisitor<'a, 'f> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.as_str();
        if self.is_restricted(source) {
            self.report(decl.span, source);
        }
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
            let value = source.value.as_str();
            if self.is_restricted(value) {
                self.report(decl.span, value);
            }
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let source = decl.source.value.as_str();
        if self.is_restricted(source) {
            self.report(decl.span, source);
        }
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{NoRestrictedImportsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, config: &NoRestrictedImportsRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("test.ts"), &program, &line_index, config)
    }

    fn test_config(restricted: &[&str]) -> NoRestrictedImportsRuleConfig {
        NoRestrictedImportsRuleConfig {
            severity: Severity::Warn,
            restricted: restricted.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn reports_exact_match() {
        let config = test_config(&["lodash"]);
        let violations = run_check("import { merge } from 'lodash';\n", &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].subject, Some("lodash".to_string()));
    }

    #[test]
    fn reports_subpath_match() {
        let config = test_config(&["lodash"]);
        let violations = run_check("import merge from 'lodash/fp/merge';\n", &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("lodash/fp/merge".to_string()));
    }

    #[test]
    fn reports_scoped_package() {
        let config = test_config(&["@internal/legacy"]);
        let violations = run_check("import { foo } from '@internal/legacy';\n", &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_scoped_package_subpath() {
        let config = test_config(&["@internal/legacy"]);
        let violations = run_check("import { foo } from '@internal/legacy/utils';\n", &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_re_export_named() {
        let config = test_config(&["moment"]);
        let violations = run_check("export { format } from 'moment';\n", &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_re_export_all() {
        let config = test_config(&["moment"]);
        let violations = run_check("export * from 'moment';\n", &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_non_restricted_import() {
        let config = test_config(&["lodash"]);
        let violations = run_check("import { useState } from 'react';\n", &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_partial_name_that_is_not_subpath() {
        let config = test_config(&["lodash"]);
        let violations = run_check("import { x } from 'lodash-es';\n", &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiple_violations() {
        let config = test_config(&["lodash", "moment"]);
        let source = "import { merge } from 'lodash';\nimport moment from 'moment';\n";
        let violations = run_check(source, &config);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn ignores_imports_in_comments() {
        let config = test_config(&["lodash"]);
        let source = "// import { merge } from 'lodash';\n";
        let violations = run_check(source, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_relative_path_restriction() {
        let config = test_config(&["../internal"]);
        let violations = run_check("import { secret } from '../internal';\n", &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_type_only_import() {
        let config = test_config(&["legacy-types"]);
        let violations = run_check("import type { Foo } from 'legacy-types';\n", &config);
        assert_eq!(violations.len(), 1);
    }
}
