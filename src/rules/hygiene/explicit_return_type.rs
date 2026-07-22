use std::path::Path;

use oxc_ast::ast::{
    ArrowFunctionExpression, Class, Declaration, ExportDefaultDeclaration, ExportNamedDeclaration,
    Expression, MethodDefinition, VariableDeclarator,
};
use oxc_ast_visit::Visit;

use crate::config::ExplicitReturnTypeRuleConfig;
use crate::rules::{EXPLICIT_RETURN_TYPE_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Exported functions must have an explicit return type.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &ExplicitReturnTypeRuleConfig,
) -> Vec<Violation> {
    let mut visitor = ExplicitReturnTypeVisitor {
        violations: Vec::new(),
        file,
        line_index,
        config,
        inside_method: false,
        inside_exported_class: false,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ExplicitReturnTypeVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    config: &'f ExplicitReturnTypeRuleConfig,
    inside_method: bool,
    inside_exported_class: bool,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> ExplicitReturnTypeVisitor<'a, 'f> {
    fn report(&mut self, span: oxc_span::Span, name: Option<&str>) {
        let pos = self.line_index.position_for(span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: EXPLICIT_RETURN_TYPE_RULE_ID,
            message: MESSAGE,
            severity: self.config.severity,
            detail: None,
            subject: name.map(|n| n.to_string()),
        });
    }

    fn should_check_arrow(&self) -> bool {
        self.config.include_arrow_functions
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
                    self.check_exported_declarator(declarator);
                }
            }
            Some(Declaration::ClassDeclaration(_class)) => {
                let was_exported = self.inside_exported_class;
                self.inside_exported_class = true;
                oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
                self.inside_exported_class = was_exported;
                return;
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
                if self.should_check_arrow() && arrow.return_type.is_none() =>
            {
                self.report(arrow.span, None);
            }
            oxc_ast::ast::ExportDefaultDeclarationKind::ClassDeclaration(_class) => {
                let was_exported = self.inside_exported_class;
                self.inside_exported_class = true;
                oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
                self.inside_exported_class = was_exported;
                return;
            }
            _ => {}
        }
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }

    fn visit_function(&mut self, func: &oxc_ast::ast::Function<'a>, _flags: oxc_syntax::scope::ScopeFlags) {
        if self.config.include_private && !self.inside_method && func.return_type.is_none() {
            let name = func.id.as_ref().map(|id| id.name.as_str());
            self.report(func.span, name);
        }
        let was_inside = self.inside_method;
        self.inside_method = false;
        oxc_ast_visit::walk::walk_function(self, func, _flags);
        self.inside_method = was_inside;
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        let should_check = self.config.include_class_methods
            && (self.inside_exported_class || self.config.include_private);
        if should_check && method.value.return_type.is_none() {
            let name = method.key.static_name();
            self.report(method.span, name.as_deref());
        }
        self.inside_method = true;
        oxc_ast_visit::walk::walk_method_definition(self, method);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        if self.config.include_private && self.should_check_arrow() && arrow.return_type.is_none() {
            self.report(arrow.span, None);
        }
        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_class(&mut self, class: &Class<'a>) {
        oxc_ast_visit::walk::walk_class(self, class);
    }
}

impl<'a, 'f> ExplicitReturnTypeVisitor<'a, 'f> {
    fn check_exported_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let Some(init) = &declarator.init else {
            return;
        };
        if self.config.ignore_when_inferred && declarator.type_annotation.is_some() {
            return;
        }
        let name = match &declarator.id {
            oxc_ast::ast::BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
            _ => None,
        };
        match init {
            Expression::ArrowFunctionExpression(arrow) => {
                if self.should_check_arrow() && arrow.return_type.is_none() {
                    self.report(arrow.span, name);
                }
            }
            Expression::FunctionExpression(func) => {
                if func.return_type.is_none() {
                    self.report(func.span, name);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{ExplicitReturnTypeRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        run_check_with_config(source, test_config())
    }

    fn run_check_with_config(
        source: &str,
        config: ExplicitReturnTypeRuleConfig,
    ) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new("utils.ts"), &program, &line_index, &config)
    }

    fn test_config() -> ExplicitReturnTypeRuleConfig {
        ExplicitReturnTypeRuleConfig {
            severity: Severity::Warn,
            include_arrow_functions: true,
            include_class_methods: false,
            include_private: false,
            ignore_when_inferred: false,
        }
    }

    #[test]
    fn reports_exported_function_without_return_type() -> Result<()> {
        let violations = run_check("export function add(a: number, b: number) { return a + b; }");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));

        Ok(())
    }

    #[test]
    fn allows_exported_function_with_return_type() -> Result<()> {
        let violations =
            run_check("export function add(a: number, b: number): number { return a + b; }");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_exported_arrow_without_return_type() -> Result<()> {
        let violations = run_check("export const add = (a: number, b: number) => a + b;");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));

        Ok(())
    }

    #[test]
    fn allows_exported_arrow_with_return_type() -> Result<()> {
        let violations =
            run_check("export const add = (a: number, b: number): number => a + b;");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_exported_function_expression_without_return_type() -> Result<()> {
        let violations =
            run_check("export const add = function(a: number, b: number) { return a + b; };");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));

        Ok(())
    }

    #[test]
    fn allows_exported_function_expression_with_return_type() -> Result<()> {
        let violations = run_check(
            "export const add = function(a: number, b: number): number { return a + b; };",
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_default_exported_function_without_return_type() -> Result<()> {
        let violations = run_check("export default function greet() { return 'hello'; }");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("greet"));

        Ok(())
    }

    #[test]
    fn allows_default_exported_function_with_return_type() -> Result<()> {
        let violations =
            run_check("export default function greet(): string { return 'hello'; }");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_default_exported_anonymous_function() -> Result<()> {
        let violations = run_check("export default function() { return 'hello'; }");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].subject.is_none());

        Ok(())
    }

    #[test]
    fn reports_default_exported_arrow_without_return_type() -> Result<()> {
        let violations = run_check("export default () => { return 'hello'; };");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_default_exported_arrow_with_return_type() -> Result<()> {
        let violations = run_check("export default (): string => { return 'hello'; };");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_non_exported_functions() -> Result<()> {
        let violations = run_check("function add(a: number, b: number) { return a + b; }");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_non_function_exports() -> Result<()> {
        let violations = run_check("export const value = 42;");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_exported_class() -> Result<()> {
        let violations = run_check("export class Foo { bar() { return 1; } }");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_multiple_violations() -> Result<()> {
        let source = "export function a() { return 1; }\nexport const b = () => 2;";
        let violations = run_check(source);
        assert_eq!(violations.len(), 2);

        Ok(())
    }

    #[test]
    fn reports_correct_line() -> Result<()> {
        let source = "const x = 1;\nexport function foo() { return x; }\n";
        let violations = run_check(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));

        Ok(())
    }

    #[test]
    fn ignores_in_comments() -> Result<()> {
        let source = "// export function foo() { return 1; }";
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_exported_void_function_with_return_type() -> Result<()> {
        let violations = run_check("export function log(msg: string): void { console.log(msg); }");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_re_export_specifiers() -> Result<()> {
        let violations = run_check("export { foo } from './foo';");
        assert!(violations.is_empty());

        Ok(())
    }

    // --- include_arrow_functions option ---

    #[test]
    fn skips_arrow_when_include_arrow_functions_false() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_arrow_functions: false,
            ..test_config()
        };
        let violations =
            run_check_with_config("export const add = (a: number, b: number) => a + b;", config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn skips_default_arrow_when_include_arrow_functions_false() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_arrow_functions: false,
            ..test_config()
        };
        let violations =
            run_check_with_config("export default () => { return 'hello'; };", config);
        assert!(violations.is_empty());

        Ok(())
    }

    // --- include_private option ---

    #[test]
    fn reports_non_exported_function_when_include_private() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_private: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("function add(a: number, b: number) { return a + b; }", config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("add"));

        Ok(())
    }

    #[test]
    fn reports_non_exported_arrow_when_include_private() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_private: true,
            ..test_config()
        };
        let violations = run_check_with_config(
            "const add = (a: number, b: number) => a + b;",
            config,
        );
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    // --- include_class_methods option ---

    #[test]
    fn reports_exported_class_methods_when_enabled() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_class_methods: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("export class Foo { bar() { return 1; } }", config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("bar"));

        Ok(())
    }

    #[test]
    fn allows_exported_class_methods_with_return_type() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_class_methods: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("export class Foo { bar(): number { return 1; } }", config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_non_exported_class_methods_without_include_private() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_class_methods: true,
            include_private: false,
            ..test_config()
        };
        let violations =
            run_check_with_config("class Foo { bar() { return 1; } }", config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_non_exported_class_methods_with_include_private() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            include_class_methods: true,
            include_private: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("class Foo { bar() { return 1; } }", config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("bar"));

        Ok(())
    }

    // --- ignore_when_inferred option ---

    #[test]
    fn skips_typed_variable_when_ignore_when_inferred() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            ignore_when_inferred: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("export const add: Adder = (a: number, b: number) => a + b;", config);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn still_reports_untyped_variable_when_ignore_when_inferred() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            ignore_when_inferred: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("export const add = (a: number, b: number) => a + b;", config);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignore_when_inferred_does_not_affect_function_declarations() -> Result<()> {
        let config = ExplicitReturnTypeRuleConfig {
            ignore_when_inferred: true,
            ..test_config()
        };
        let violations =
            run_check_with_config("export function add(a: number, b: number) { return a + b; }", config);
        assert_eq!(violations.len(), 1);

        Ok(())
    }
}
