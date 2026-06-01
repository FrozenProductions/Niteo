use std::path::Path;

use oxc_ast::ast::{
    Declaration, ExportDefaultDeclaration, ExportNamedDeclaration, Expression,
    VariableDeclarator,
};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::rules::{EXPLICIT_RETURN_TYPE_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Exported functions must have an explicit return type.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    let mut visitor = ExplicitReturnTypeVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ExplicitReturnTypeVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> ExplicitReturnTypeVisitor<'a, 'f> {
    fn report(&mut self, span: oxc_span::Span, name: Option<&str>) {
        let pos = self.line_index.position_for(span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: EXPLICIT_RETURN_TYPE_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: name.map(|n| n.to_string()),
        });
    }
}

impl<'a, 'f> Visit<'a> for ExplicitReturnTypeVisitor<'a, 'f> {
    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        match &decl.declaration {
            Some(Declaration::FunctionDeclaration(func))
                if func.return_type.is_none() =>
            {
                let name = func.id.as_ref().map(|id| id.name.as_str());
                self.report(func.span, name);
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
            oxc_ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(func)
                if func.return_type.is_none() =>
            {
                let name = func.id.as_ref().map(|id| id.name.as_str());
                self.report(func.span, name);
            }
            oxc_ast::ast::ExportDefaultDeclarationKind::ArrowFunctionExpression(arrow)
                if arrow.return_type.is_none() =>
            {
                self.report(arrow.span, None);
            }
            _ => {}
        }
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

fn check_exported_declarator<'a>(
    visitor: &mut ExplicitReturnTypeVisitor<'a, '_>,
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
        Expression::ArrowFunctionExpression(arrow) if arrow.return_type.is_none() => {
            visitor.report(arrow.span, name);
        }
        Expression::FunctionExpression(func) if func.return_type.is_none() => {
            visitor.report(func.span, name);
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
            &test_config(),
        )
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn reports_exported_function_without_return_type() {
        let violations = run_check("export function add(a: number, b: number) { return a + b; }");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));
    }

    #[test]
    fn allows_exported_function_with_return_type() {
        let violations =
            run_check("export function add(a: number, b: number): number { return a + b; }");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_exported_arrow_without_return_type() {
        let violations = run_check("export const add = (a: number, b: number) => a + b;");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));
    }

    #[test]
    fn allows_exported_arrow_with_return_type() {
        let violations =
            run_check("export const add = (a: number, b: number): number => a + b;");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_exported_function_expression_without_return_type() {
        let violations =
            run_check("export const add = function(a: number, b: number) { return a + b; };");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));
    }

    #[test]
    fn allows_exported_function_expression_with_return_type() {
        let violations = run_check(
            "export const add = function(a: number, b: number): number { return a + b; };",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_default_exported_function_without_return_type() {
        let violations = run_check("export default function greet() { return 'hello'; }");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("greet"));
    }

    #[test]
    fn allows_default_exported_function_with_return_type() {
        let violations =
            run_check("export default function greet(): string { return 'hello'; }");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_default_exported_anonymous_function() {
        let violations = run_check("export default function() { return 'hello'; }");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].subject.is_none());
    }

    #[test]
    fn reports_default_exported_arrow_without_return_type() {
        let violations = run_check("export default () => { return 'hello'; };");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_default_exported_arrow_with_return_type() {
        let violations = run_check("export default (): string => { return 'hello'; };");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_exported_functions() {
        let violations = run_check("function add(a: number, b: number) { return a + b; }");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_function_exports() {
        let violations = run_check("export const value = 42;");
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_exported_class() {
        let violations = run_check("export class Foo { bar() { return 1; } }");
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiple_violations() {
        let source = "export function a() { return 1; }\nexport const b = () => 2;";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn reports_correct_line() {
        let source = "const x = 1;\nexport function foo() { return x; }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn ignores_in_comments() {
        let source = "// export function foo() { return 1; }";
        let violations = run_check(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_exported_void_function_with_return_type() {
        let violations = run_check("export function log(msg: string): void { console.log(msg); }");
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_re_export_specifiers() {
        let violations = run_check("export { foo } from './foo';");
        assert!(violations.is_empty());
    }
}
