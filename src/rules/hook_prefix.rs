use std::path::Path;

use oxc_ast::ast::{BindingPattern, Expression, VariableDeclarationKind};
use oxc_ast_visit::Visit;

use crate::config::HookPrefixRuleConfig;
use crate::jsx::is_hook_file;
use crate::rules::{HOOK_PREFIX_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Hook function name must start with one of the configured prefixes.";

const DEFAULT_PREFIXES: &[&str] = &["use"];

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &HookPrefixRuleConfig,
) -> Vec<Violation> {
    if !is_hook_file(file) {
        return Vec::new();
    }

    let prefixes: Vec<&str> = if config.prefixes.is_empty() {
        DEFAULT_PREFIXES.to_vec()
    } else {
        config.prefixes.iter().map(|p| p.as_str()).collect()
    };

    let mut visitor = HookPrefixVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        prefixes,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct HookPrefixVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    prefixes: Vec<&'f str>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for HookPrefixVisitor<'a, 'f> {
    fn visit_function(
        &mut self,
        func: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        if let Some(id) = &func.id {
            let name = id.name.as_str();
            if !has_valid_prefix(name, &self.prefixes) {
                let pos = self.line_index.position_for(id.span);
                let detail = Some(format!(
                    "'{}' does not start with any prefix: {}",
                    name,
                    self.prefixes.join(", ")
                ));
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: HOOK_PREFIX_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail,
                    subject: Some(name.to_string()),
                });
            }
        }

        oxc_ast_visit::walk::walk_function(self, func, flags);
    }

    fn visit_variable_declaration(&mut self, decl: &oxc_ast::ast::VariableDeclaration<'a>) {
        if matches!(decl.kind, VariableDeclarationKind::Var) {
            oxc_ast_visit::walk::walk_variable_declaration(self, decl);
            return;
        }

        for declarator in &decl.declarations {
            let BindingPattern::BindingIdentifier(binding_id) = &declarator.id else {
                continue;
            };

            let is_function = declarator
                .init
                .as_ref()
                .is_some_and(|init| is_function_expression(init));

            if !is_function {
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
                rule: HOOK_PREFIX_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some(name.to_string()),
            });
        }

        oxc_ast_visit::walk::walk_variable_declaration(self, decl);
    }
}

fn is_function_expression(expr: &Expression<'_>) -> bool {
    matches!(
        expr,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    )
}

fn has_valid_prefix(name: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        name.len() > prefix.len()
            && name.starts_with(prefix)
            && name
                .as_bytes()
                .get(prefix.len())
                .is_some_and(|c| c.is_ascii_uppercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HookPrefixRuleConfig, Severity};
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
        check_file(Path::new(file_path), &program, &line_index, &test_config())
    }

    fn test_config() -> HookPrefixRuleConfig {
        HookPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec![],
        }
    }

    #[test]
    fn ignores_non_hook_files() {
        let source = "export function authenticate() { return true; }\n";
        let violations = run_check(source, "src/components/Auth.tsx");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_use_prefixed_function_in_hooks_folder() {
        let source = "export function useAuth() { return { user: null }; }\n";
        let violations = run_check(source, "src/hooks/useAuth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_use_prefixed_arrow_in_hooks_folder() {
        let source = "export const useCounter = () => { return { count: 0 }; };\n";
        let violations = run_check(source, "src/hooks/useCounter.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_unprefixed_function_in_hooks_folder() {
        let source = "export function authenticate() { return true; }\n";
        let violations = run_check(source, "src/hooks/auth.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_unprefixed_arrow_in_hooks_folder() {
        let source = "export const getData = () => { return {}; };\n";
        let violations = run_check(source, "src/hooks/getData.ts");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("getData"));
    }

    #[test]
    fn ignores_non_function_const_in_hooks_folder() {
        let source = "const value = 42;\nconst name = 'Niteo';\n";
        let violations = run_check(source, "src/hooks/constants.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_var_declarations() {
        let source = "var authenticate = function() { return true; };\n";
        let violations = run_check(source, "src/hooks/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_prefixed_let_in_hooks_folder() {
        let source = "let useCounter = () => { return { count: 0 }; };\n";
        let violations = run_check(source, "src/hooks/useCounter.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_unprefixed_let_function_in_hooks_folder() {
        let source = "let counter = () => { return { count: 0 }; };\n";
        let violations = run_check(source, "src/hooks/counter.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_prefixed_in_dot_hook_file() {
        let source = "export function useToggle() { return [true, () => {}] as const; }\n";
        let violations = run_check(source, "useToggle.hook.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_unprefixed_in_dot_hook_file() {
        let source = "export function toggle() { return [true, () => {}] as const; }\n";
        let violations = run_check(source, "toggle.hook.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_prefixed_in_dot_hooks_file() {
        let source = "export function useToggle() { return [true, () => {}] as const; }\n";
        let violations = run_check(source, "useToggle.hooks.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn respects_custom_prefixes() {
        let config = HookPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec!["should".to_string(), "with".to_string()],
        };
        let allocator = Allocator::default();
        let source = "export function shouldRender() { return true; }\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("src/hooks/render.ts"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn flags_unprefixed_with_custom_prefixes() {
        let config = HookPrefixRuleConfig {
            severity: Severity::Warn,
            prefixes: vec!["should".to_string()],
        };
        let allocator = Allocator::default();
        let source = "export function useRender() { return true; }\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let violations = check_file(
            Path::new("src/hooks/render.ts"),
            &parser_return.program,
            &line_index,
            &config,
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn requires_camel_case_after_prefix() {
        let source = "export function usecounter() { return { count: 0 }; }\n";
        let violations = run_check(source, "src/hooks/useCounter.ts");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_destructured_variables() {
        let source = "const { useAuth } = someImportedModule;\n";
        let violations = run_check(source, "src/hooks/auth.ts");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_arrow_non_function_const() {
        let source = "const useAuth = someExistingHook;\n";
        let violations = run_check(source, "src/hooks/auth.ts");
        assert!(violations.is_empty());
    }
}
