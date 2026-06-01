use std::path::Path;

use oxc_ast::ast::{BindingPattern, Expression, TSType, TSTypeName, VariableDeclarationKind};
use oxc_ast_visit::Visit;

use crate::config::BooleanPrefixRuleConfig;
use crate::rules::{BOOLEAN_PREFIX_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Boolean should start with one of the configured prefixes.";

const DEFAULT_PREFIXES: &[&str] = &["is", "has", "can"];

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &BooleanPrefixRuleConfig,
) -> Vec<Violation> {
    let prefixes: Vec<&str> = if config.prefixes.is_empty() {
        DEFAULT_PREFIXES.to_vec()
    } else {
        config.prefixes.iter().map(|prefix| prefix.as_str()).collect()
    };

    let mut visitor = BooleanPrefixVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        prefixes,
        ignore_constants: config.ignore_constants,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct BooleanPrefixVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    prefixes: Vec<&'f str>,
    ignore_constants: bool,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for BooleanPrefixVisitor<'a, 'f> {
    fn visit_variable_declaration(&mut self, decl: &oxc_ast::ast::VariableDeclaration<'a>) {
        if matches!(decl.kind, VariableDeclarationKind::Var) {
            oxc_ast_visit::walk::walk_variable_declaration(self, decl);
            return;
        }

        if self.ignore_constants && matches!(decl.kind, VariableDeclarationKind::Const) {
            oxc_ast_visit::walk::walk_variable_declaration(self, decl);
            return;
        }

        for declarator in &decl.declarations {
            let BindingPattern::BindingIdentifier(binding_id) = &declarator.id else {
                continue;
            };

            let is_boolean = is_boolean_type_annotation(&declarator.type_annotation)
                || has_boolean_initializer(&declarator.init);

            if !is_boolean {
                continue;
            }

            let name = binding_id.name.as_str();
            if has_valid_prefix(name, &self.prefixes) {
                continue;
            }

            let pos = self.line_index.position_for(binding_id.span);
            let detail = Some(format!(
                "'{}' does not start with any prefix: {}",
                name,
                self.prefixes.join(", ")
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: BOOLEAN_PREFIX_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some(name.to_string()),
            });
        }

        oxc_ast_visit::walk::walk_variable_declaration(self, decl);
    }
}

fn is_boolean_type_annotation(
    type_annotation: &Option<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeAnnotation<'_>>>,
) -> bool {
    let Some(annotation) = type_annotation else {
        return false;
    };
    matches!(&annotation.type_annotation, TSType::TSBooleanKeyword(_))
        || is_boolean_type_reference(&annotation.type_annotation)
}

fn is_boolean_type_reference(ts_type: &TSType<'_>) -> bool {
    let TSType::TSTypeReference(type_ref) = ts_type else {
        return false;
    };
    match &type_ref.type_name {
        TSTypeName::IdentifierReference(id) => id.name == "Boolean",
        _ => false,
    }
}

fn has_boolean_initializer(init: &Option<Expression<'_>>) -> bool {
    let Some(expr) = init else {
        return false;
    };
    matches!(expr, Expression::BooleanLiteral(_))
}

fn has_valid_prefix(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        name.len() > prefix.len()
            && name.starts_with(prefix)
            && name
                .as_bytes()
                .get(prefix.len())
                .is_some_and(|byte| byte.is_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BooleanPrefixRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, config: &BooleanPrefixRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(Path::new("Component.tsx"), &program, &line_index, config)
    }

    fn test_config() -> BooleanPrefixRuleConfig {
        BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec![],
            ignore_constants: false,
        }
    }

    #[test]
    fn allows_default_prefixed_boolean_literals() {
        for source in [
            "const isOpen = true;\n",
            "const hasPermission = false;\n",
            "const canEdit = true;\n",
        ] {
            let violations = run_check(source, &test_config());
            assert!(violations.is_empty(), "expected no violations for: {source:?}");
        }
    }

    #[test]
    fn reports_unprefixed_boolean_literal() {
        let source = "const open = true;\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(7));
    }

    #[test]
    fn reports_unprefixed_boolean_type() {
        let source = "const open: boolean = fetchData();\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("open"));
    }

    #[test]
    fn reports_unprefixed_boolean_capital_type() {
        let source = "const open: Boolean = fetchData();\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_prefixed_boolean_type() {
        let source = "const isReady: boolean = check();\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_let_with_prefix() {
        let source = "let hasChanged = false;\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_let_without_prefix() {
        let source = "let changed = false;\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_var_declarations() {
        let source = "var open = true;\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_boolean_const() {
        let source = "const count = 1;\nconst name = 'Niteo';\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_boolean_type() {
        let source = "const count: number = 1;\nconst name: string = 'Niteo';\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_destructured_variables() {
        let source = "const { open } = props;\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn respects_custom_prefixes() {
        let config = BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec!["should".to_string(), "will".to_string()],
            ignore_constants: false,
        };
        let source = "const shouldRender = true;\n";
        let violations = run_check(source, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn flags_unprefixed_with_custom_prefixes() {
        let config = BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec!["should".to_string()],
            ignore_constants: false,
        };
        let source = "const isOpen = true;\n";
        let violations = run_check(source, &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn requires_camel_case_after_prefix() {
        let source = "const isopen = true;\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_boolean_literals_in_comments() {
        let source = "// const open = true;\n/* const open = false; */\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_boolean_literals_in_strings() {
        let source = r#"const text = "const open = true";"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_const_when_ignore_constants_enabled() {
        let config = BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec![],
            ignore_constants: true,
        };
        let source = "const open = true;\n";
        let violations = run_check(source, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn still_reports_let_when_ignore_constants_enabled() {
        let config = BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec![],
            ignore_constants: true,
        };
        let source = "let open = true;\n";
        let violations = run_check(source, &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn resolves_ignore_constants_default() {
        let config = BooleanPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec!["should".to_string()],
            ignore_constants: false,
        };
        let source = "const shouldRender = true;\n";
        let violations = run_check(source, &config);
        assert!(violations.is_empty());
    }
}
