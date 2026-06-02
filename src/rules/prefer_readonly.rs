use std::path::Path;

use oxc_ast::ast::{
    Declaration, ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
    Expression, TSType, TSTypeName, TSTypeOperatorOperator, VariableDeclarator,
};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{PREFER_READONLY_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Array parameter in exported function should use `readonly` to prevent accidental mutation.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = PreferReadonlyVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct PreferReadonlyVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> PreferReadonlyVisitor<'a, 'f> {
    fn report(&mut self, span: oxc_span::Span, param_name: Option<&str>, func_name: Option<&str>) {
        let pos = self.line_index.position_for(span);
        let subject = match (func_name, param_name) {
            (Some(function), Some(parameter)) => Some(format!("{function}.{parameter}")),
            (Some(function), None) => Some(function.to_string()),
            (None, Some(parameter)) => Some(parameter.to_string()),
            (None, None) => None,
        };
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: PREFER_READONLY_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject,
        });
    }

    fn check_params(
        &mut self,
        params: &oxc_ast::ast::FormalParameters<'a>,
        func_name: Option<&str>,
    ) {
        for param in &params.items {
            if let Some(type_annotation) = &param.type_annotation {
                if is_mutable_array_type(&type_annotation.type_annotation) {
                    let param_name = binding_name(&param.pattern);
                    self.report(param.span, param_name, func_name);
                }
            }
        }
        if let Some(rest) = &params.rest {
            if let Some(type_annotation) = &rest.type_annotation {
                if is_mutable_array_type(&type_annotation.type_annotation) {
                    let param_name = binding_name(&rest.rest.argument);
                    self.report(rest.span, param_name, func_name);
                }
            }
        }
    }
}

fn is_mutable_array_type(ts_type: &TSType) -> bool {
    match ts_type {
        TSType::TSArrayType(_) => true,
        TSType::TSTypeOperatorType(operator) => {
            if operator.operator == TSTypeOperatorOperator::Readonly {
                false
            } else {
                is_mutable_array_type(&operator.type_annotation)
            }
        }
        TSType::TSTypeReference(type_ref) => {
            if let TSTypeName::IdentifierReference(id) = &type_ref.type_name {
                id.name == "Array"
            } else {
                false
            }
        }
        TSType::TSParenthesizedType(paren) => is_mutable_array_type(&paren.type_annotation),
        _ => false,
    }
}

fn binding_name<'a>(pattern: &'a oxc_ast::ast::BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        oxc_ast::ast::BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

impl<'a, 'f> Visit<'a> for PreferReadonlyVisitor<'a, 'f> {
    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        match &decl.declaration {
            Some(Declaration::FunctionDeclaration(func)) => {
                let name = func.id.as_ref().map(|id| id.name.as_str());
                self.check_params(&func.params, name);
            }
            Some(Declaration::VariableDeclaration(var_decl)) => {
                for declarator in &var_decl.declarations {
                    check_exported_declarator(self, declarator);
                }
            }
            _ => {}
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        match &decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                let name = func.id.as_ref().map(|id| id.name.as_str());
                self.check_params(&func.params, name);
            }
            ExportDefaultDeclarationKind::ArrowFunctionExpression(arrow) => {
                self.check_params(&arrow.params, None);
            }
            _ => {}
        }
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

fn check_exported_declarator<'a>(
    visitor: &mut PreferReadonlyVisitor<'a, '_>,
    declarator: &VariableDeclarator<'a>,
) {
    let Some(init) = &declarator.init else {
        return;
    };
    let name = match &declarator.id {
        oxc_ast::ast::BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    };
    match init {
        Expression::ArrowFunctionExpression(arrow) => {
            visitor.check_params(&arrow.params, name);
        }
        Expression::FunctionExpression(func) => {
            visitor.check_params(&func.params, name);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("utils.ts"),
            &program,
            &line_index,
            &RuleConfig {
                severity: Severity::Warn,
            },
        )
    }

    #[test]
    fn reports_array_param_in_exported_function() {
        let violations = run_check("export function process(items: string[]) {}");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("process.items"));
    }

    #[test]
    fn allows_readonly_array_param() {
        let violations = run_check("export function process(items: readonly string[]) {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_array_type_reference_param() {
        let violations = run_check("export function process(items: Array<string>) {}");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_readonly_array_type_reference() {
        let violations = run_check("export function process(items: ReadonlyArray<string>) {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_array_param_in_exported_arrow() {
        let violations = run_check("export const process = (items: number[]) => {};");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("process.items"));
    }

    #[test]
    fn reports_array_param_in_exported_function_expression() {
        let violations = run_check("export const process = function(items: string[]) {};");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_array_param_in_default_exported_function() {
        let violations = run_check("export default function process(items: string[]) {}");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_array_param_in_default_exported_arrow() {
        let violations = run_check("export default (items: string[]) => {};");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_non_exported_function() {
        let violations = run_check("function process(items: string[]) {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_array_params() {
        let violations = run_check("export function process(name: string, count: number) {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiple_array_params() {
        let violations =
            run_check("export function merge(a: string[], b: number[]) {}");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn allows_readonly_mixed_with_non_array() {
        let violations = run_check(
            "export function process(name: string, items: readonly string[]) {}",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_rest_param_with_array_type() {
        let violations = run_check("export function process(...items: string[][]) {}");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_readonly_rest_param() {
        let violations = run_check("export function process(...items: readonly string[][]) {}");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_correct_line() {
        let source = "const x = 1;\nexport function foo(items: string[]) {}\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn allows_tuple_param() {
        let violations = run_check("export function process(items: [string, number]) {}");
        assert!(violations.is_empty());
    }
}
