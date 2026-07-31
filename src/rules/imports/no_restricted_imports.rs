use std::path::Path;

use globset::{GlobBuilder, GlobMatcher};
use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, ExportSpecifier, ImportDeclaration,
    ImportDeclarationSpecifier,
};
use oxc_ast_visit::Visit;

use crate::config::{NoRestrictedImportsRuleConfig, RestrictedImportPattern};
use crate::rules::{NO_RESTRICTED_IMPORTS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import from a restricted package or path.";

struct CompiledPattern {
    pattern: RestrictedImportPattern,
    glob_matcher: Option<GlobMatcher>,
}

fn compile_patterns(config: &NoRestrictedImportsRuleConfig) -> Vec<CompiledPattern> {
    config
        .restricted
        .iter()
        .map(|pat| {
            let pattern_str = pat.pattern();
            let glob_matcher = if has_glob_chars(pattern_str) {
                GlobBuilder::new(pattern_str)
                    .literal_separator(true)
                    .build()
                    .ok()
                    .map(|g| g.compile_matcher())
            } else {
                None
            };
            CompiledPattern {
                pattern: pat.clone(),
                glob_matcher,
            }
        })
        .collect()
}

fn has_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn source_matches(pattern_str: &str, glob_matcher: Option<&GlobMatcher>, source: &str) -> bool {
    if let Some(matcher) = glob_matcher {
        matcher.is_match(source)
    } else {
        source == pattern_str || source.starts_with(&format!("{pattern_str}/"))
    }
}

fn extract_imported_names<'a>(
    specifiers: &Option<oxc_allocator::Vec<'a, ImportDeclarationSpecifier<'a>>>,
) -> Vec<String> {
    let Some(specifiers) = specifiers else {
        return Vec::new();
    };
    specifiers
        .iter()
        .filter_map(|s| match s {
            ImportDeclarationSpecifier::ImportSpecifier(spec) => local_as_str(&spec.imported),
            _ => None,
        })
        .collect()
}

fn extract_exported_names<'a>(
    specifiers: &oxc_allocator::Vec<'a, ExportSpecifier<'a>>,
) -> Vec<String> {
    specifiers
        .iter()
        .filter_map(|s| local_as_str(&s.local))
        .collect()
}

fn local_as_str(local: &oxc_ast::ast::ModuleExportName) -> Option<String> {
    match local {
        oxc_ast::ast::ModuleExportName::IdentifierReference(r) => Some(r.name.to_string()),
        oxc_ast::ast::ModuleExportName::IdentifierName(n) => Some(n.name.to_string()),
        oxc_ast::ast::ModuleExportName::StringLiteral(s) => Some(s.value.to_string()),
    }
}

fn named_intersects(pattern_named: &[String], import_names: &[String]) -> bool {
    import_names
        .iter()
        .any(|name| pattern_named.iter().any(|n| n == name))
}

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoRestrictedImportsRuleConfig,
) -> Vec<Violation> {
    if config.restricted.is_empty() {
        return Vec::new();
    }

    let compiled = compile_patterns(config);

    let mut visitor = RestrictedImportsVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        patterns: &compiled,
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
    patterns: &'f [CompiledPattern],
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl RestrictedImportsVisitor<'_, '_> {
    fn find_matching_index(&self, source: &str, import_names: &[String]) -> Option<usize> {
        self.patterns.iter().position(|compiled| {
            if !source_matches(
                compiled.pattern.pattern(),
                compiled.glob_matcher.as_ref(),
                source,
            ) {
                return false;
            }
            match compiled.pattern.named() {
                Some(named) => named_intersects(named, import_names),
                None => true,
            }
        })
    }

    fn push_violation(
        &mut self,
        span: oxc_span::Span,
        source: &str,
        pattern_index: usize,
    ) {
        let pos = self.line_index.position_for(span);
        let pattern = &self.patterns[pattern_index].pattern;
        let detail = if let Some(custom) = pattern.message() {
            format!("\"{source}\" matches a restricted pattern: {custom}")
        } else {
            format!("\"{source}\" matches a restricted pattern")
        };

        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_RESTRICTED_IMPORTS_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: Some(detail),
            subject: Some(source.to_string()),
        });
    }
}

impl<'a, 'f> Visit<'a> for RestrictedImportsVisitor<'a, 'f> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.as_str();
        let import_names = extract_imported_names(&decl.specifiers);
        if let Some(index) = self.find_matching_index(source, &import_names) {
            self.push_violation(decl.span, source, index);
        }
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source_node) = &decl.source {
            let source = source_node.value.as_str();
            let export_names = extract_exported_names(&decl.specifiers);
            if let Some(index) = self.find_matching_index(source, &export_names) {
                self.push_violation(decl.span, source, index);
            }
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        let source = decl.source.value.as_str();
        if let Some(index) = self.find_matching_index(source, &[]) {
            self.push_violation(decl.span, source, index);
        }
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
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

    fn config(restricted: Vec<RestrictedImportPattern>) -> NoRestrictedImportsRuleConfig {
        NoRestrictedImportsRuleConfig {
            severity: Severity::Warn,
            restricted,
        }
    }

    fn simple(pat: &str) -> RestrictedImportPattern {
        RestrictedImportPattern::Simple(pat.to_string())
    }

    fn full(
        pat: &str,
        named: Option<Vec<&str>>,
        message: Option<&str>,
    ) -> RestrictedImportPattern {
        RestrictedImportPattern::Full {
            pattern: pat.to_string(),
            named: named.map(|v| v.into_iter().map(String::from).collect()),
            message: message.map(String::from),
        }
    }

    #[test]
    fn reports_exact_match() -> Result<()> {
        let violations = run_check(
            "import { merge } from 'lodash';\n",
            &config(vec![simple("lodash")]),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].subject.as_deref(), Some("lodash"));
        Ok(())
    }

    #[test]
    fn reports_subpath_match() -> Result<()> {
        let violations = run_check(
            "import merge from 'lodash/fp/merge';\n",
            &config(vec![simple("lodash")]),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("lodash/fp/merge"));
        Ok(())
    }

    #[test]
    fn reports_scoped_package() -> Result<()> {
        let violations = run_check(
            "import { foo } from '@internal/legacy';\n",
            &config(vec![simple("@internal/legacy")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_scoped_package_subpath() -> Result<()> {
        let violations = run_check(
            "import { foo } from '@internal/legacy/utils';\n",
            &config(vec![simple("@internal/legacy")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_re_export_named() -> Result<()> {
        let violations = run_check(
            "export { format } from 'moment';\n",
            &config(vec![simple("moment")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_re_export_all() -> Result<()> {
        let violations = run_check(
            "export * from 'moment';\n",
            &config(vec![simple("moment")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn allows_non_restricted_import() -> Result<()> {
        let violations = run_check(
            "import { useState } from 'react';\n",
            &config(vec![simple("lodash")]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn allows_partial_name_that_is_not_subpath() -> Result<()> {
        let violations = run_check(
            "import { x } from 'lodash-es';\n",
            &config(vec![simple("lodash")]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_multiple_violations() -> Result<()> {
        let violations = run_check(
            "import { merge } from 'lodash';\nimport moment from 'moment';\n",
            &config(vec![simple("lodash"), simple("moment")]),
        );
        assert_eq!(violations.len(), 2);
        Ok(())
    }

    #[test]
    fn ignores_imports_in_comments() -> Result<()> {
        let violations = run_check(
            "// import { merge } from 'lodash';\n",
            &config(vec![simple("lodash")]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_relative_path_restriction() -> Result<()> {
        let violations = run_check(
            "import { secret } from '../internal';\n",
            &config(vec![simple("../internal")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn reports_type_only_import() -> Result<()> {
        let violations = run_check(
            "import type { Foo } from 'legacy-types';\n",
            &config(vec![simple("legacy-types")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn glob_wildcard_matches_any_single_segment() -> Result<()> {
        let violations = run_check(
            "import { x } from '@internal/foo';\n",
            &config(vec![simple("@internal/*")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn glob_wildcard_does_not_match_no_segment() -> Result<()> {
        let violations = run_check(
            "import { x } from '@internal';\n",
            &config(vec![simple("@internal/*")]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn glob_double_star_matches_deep() -> Result<()> {
        let violations = run_check(
            "import { x } from '@internal/a/b/c';\n",
            &config(vec![simple("@internal/**")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn glob_question_mark_matches_single_char() -> Result<()> {
        let violations = run_check(
            "import { x } from 'lib-v1';\n",
            &config(vec![simple("lib-v?")]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn named_restriction_blocks_specific_import() -> Result<()> {
        let violations = run_check(
            "import { deprecated } from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn named_restriction_allows_non_restricted_import_from_same_module() -> Result<()> {
        let violations = run_check(
            "import { allowed } from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn named_restriction_allows_deep_import_from_same_module() -> Result<()> {
        let violations = run_check(
            "import { x } from 'my-lib/sub';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn named_restriction_blocks_multiple_names() -> Result<()> {
        let violations = run_check(
            "import { deprecated, old } from 'my-lib';\n",
            &config(vec![full(
                "my-lib",
                Some(vec!["deprecated", "old"]),
                None,
            )]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn named_restriction_does_not_match_default_import() -> Result<()> {
        let violations = run_check(
            "import myLib from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn named_restriction_does_not_match_namespace_import() -> Result<()> {
        let violations = run_check(
            "import * as myLib from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn named_restriction_with_re_export() -> Result<()> {
        let violations = run_check(
            "export { deprecated } from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert_eq!(violations.len(), 1);
        Ok(())
    }

    #[test]
    fn named_restriction_does_not_match_export_all() -> Result<()> {
        let violations = run_check(
            "export * from 'my-lib';\n",
            &config(vec![full("my-lib", Some(vec!["deprecated"]), None)]),
        );
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn custom_message_appears_in_detail() -> Result<()> {
        let violations = run_check(
            "import { merge } from 'lodash';\n",
            &config(vec![full("lodash", None, Some("Use lodash-es instead."))]),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].detail.as_deref(),
            Some("\"lodash\" matches a restricted pattern: Use lodash-es instead.")
        );
        Ok(())
    }

    #[test]
    fn default_detail_when_no_custom_message() -> Result<()> {
        let violations = run_check(
            "import { merge } from 'lodash';\n",
            &config(vec![simple("lodash")]),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].detail.as_deref(),
            Some("\"lodash\" matches a restricted pattern")
        );
        Ok(())
    }

    #[test]
    fn toml_string_is_parsed_as_simple() {
        let toml_str = r#"
severity = "warn"
restricted = ["lodash", "moment"]
"#;
        let config: NoRestrictedImportsRuleConfig =
            toml::from_str(toml_str).unwrap();
        assert_eq!(config.restricted.len(), 2);
        match &config.restricted[0] {
            RestrictedImportPattern::Simple(s) => assert_eq!(s, "lodash"),
            _ => panic!("expected Simple"),
        }
    }

    #[test]
    fn toml_table_is_parsed_as_full() {
        let toml_str = r#"
severity = "warn"
[[restricted]]
pattern = "my-lib"
named = ["deprecated", "oldFn"]
message = "Use new API instead."
"#;
        let config: NoRestrictedImportsRuleConfig =
            toml::from_str(toml_str).unwrap();
        assert_eq!(config.restricted.len(), 1);
        match &config.restricted[0] {
            RestrictedImportPattern::Full {
                pattern,
                named,
                message,
            } => {
                assert_eq!(pattern, "my-lib");
                assert_eq!(
                    named.as_deref(),
                    Some(&["deprecated".to_string(), "oldFn".to_string()][..])
                );
                assert_eq!(message.as_deref(), Some("Use new API instead."));
            }
            _ => panic!("expected Full"),
        }
    }

    #[test]
    fn toml_optional_fields_omitted() {
        let toml_str = r#"
severity = "warn"
[[restricted]]
pattern = "my-lib"
"#;
        let config: NoRestrictedImportsRuleConfig =
            toml::from_str(toml_str).unwrap();
        assert_eq!(config.restricted.len(), 1);
        assert_eq!(config.restricted[0].named(), None);
        assert_eq!(config.restricted[0].message(), None);
    }

    #[test]
    fn serialized_roundtrip() -> Result<()> {
        let config = NoRestrictedImportsRuleConfig {
            severity: Severity::Warn,
            restricted: vec![
                RestrictedImportPattern::Simple("lodash".to_string()),
                RestrictedImportPattern::Full {
                    pattern: "my-lib".to_string(),
                    named: Some(vec!["deprecated".to_string()]),
                    message: Some("Use new API".to_string()),
                },
            ],
        };
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: NoRestrictedImportsRuleConfig =
            toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.restricted.len(), 2);
        Ok(())
    }
}
