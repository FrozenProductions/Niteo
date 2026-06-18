use std::path::Path;

use oxc_ast::ast::NumericLiteral;
use oxc_ast_visit::Visit;

use crate::config::NoMagicNumbersRuleConfig;
use crate::rules::{NO_MAGIC_NUMBERS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "No magic numbers. Extract numeric literals to named constants.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoMagicNumbersRuleConfig,
) -> Vec<Violation> {
    let mut visitor = MagicNumberVisitor {
        violations: Vec::new(),
        file,
        line_index,
        config,
        current_var_decl_is_const: false,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct MagicNumberVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    config: &'a NoMagicNumbersRuleConfig,
    current_var_decl_is_const: bool,
}

impl<'a, 'f> MagicNumberVisitor<'a, 'f> {
    fn is_allowed_number(&self, literal: &NumericLiteral) -> bool {
        self.config.allowed_numbers.iter().any(|allowed| {
            if let Ok(allowed_value) = allowed.parse::<f64>() {
                (literal.value - allowed_value).abs() < f64::EPSILON
            } else {
                false
            }
        })
    }
}

impl<'a, 'f> Visit<'a> for MagicNumberVisitor<'a, 'f> {
    fn visit_numeric_literal(&mut self, literal: &NumericLiteral<'a>) {
        if self.is_allowed_number(literal) {
            return;
        }

        let pos = self.line_index.position_for(literal.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(literal.span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_MAGIC_NUMBERS_RULE_ID,
            message: MESSAGE,
            severity: self.config.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_numeric_literal(self, literal);
    }

    fn visit_variable_declaration(&mut self, decl: &oxc_ast::ast::VariableDeclaration<'a>) {
        use oxc_ast::ast::VariableDeclarationKind;
        let was_const = self.current_var_decl_is_const;
        self.current_var_decl_is_const = matches!(decl.kind, VariableDeclarationKind::Const);
        oxc_ast_visit::walk::walk_variable_declaration(self, decl);
        self.current_var_decl_is_const = was_const;
    }

    fn visit_variable_declarator(&mut self, declarator: &oxc_ast::ast::VariableDeclarator<'a>) {
        if self.current_var_decl_is_const
            && let Some(init) = &declarator.init
        {
            if let oxc_ast::ast::Expression::NumericLiteral(_) = init {
                return;
            }
            if let oxc_ast::ast::Expression::UnaryExpression(unary) = init {
                use oxc_ast::ast::UnaryOperator;
                if matches!(unary.operator, UnaryOperator::UnaryNegation)
                    && let oxc_ast::ast::Expression::NumericLiteral(_) = &unary.argument
                {
                    return;
                }
            }
        }
        oxc_ast_visit::walk::walk_variable_declarator(self, declarator);
    }

    fn visit_ts_enum_member(&mut self, _member: &oxc_ast::ast::TSEnumMember<'a>) {}

    fn visit_ts_type(&mut self, _ty: &oxc_ast::ast::TSType<'a>) {}

    fn visit_jsx_attribute(&mut self, _attr: &oxc_ast::ast::JSXAttribute<'a>) {}
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::Severity;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str) -> Vec<Violation> {
        run_check_with_config(source, vec![])
    }

    fn run_check_with_config(source: &str, allowed_numbers: Vec<String>) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let config = NoMagicNumbersRuleConfig {
            severity: Severity::Warn,
            allowed_numbers,
        };
        check_file(Path::new("test.ts"), &program, &line_index, &config)
    }

    #[test]
    fn reports_magic_number_in_function_call() -> Result<()> {
        let violations = run_check("setTimeout(callback, 3000);\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    
        Ok(())}

    #[test]
    fn reports_magic_number_in_expression() -> Result<()> {
        let violations = run_check("const result = price * 1.15;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_magic_number_in_array() -> Result<()> {
        let violations = run_check("const array = [1, 2, 3, 42, 5];\n");
        assert_eq!(violations.len(), 5);
    
        Ok(())}

    #[test]
    fn allows_numbers_in_const_declarations() -> Result<()> {
        let violations = run_check("const MAX_SIZE = 100;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_numbers_in_enum_members() -> Result<()> {
        let violations = run_check("enum Status { Active = 1, Inactive = 0 }\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_numbers_in_type_annotations() -> Result<()> {
        let violations = run_check("type Size = 10 | 20 | 30;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_numbers_in_jsx_attributes() -> Result<()> {
        let violations = run_check("const element = <div width={100} height={200} />;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_magic_number_in_let_declaration() -> Result<()> {
        let violations = run_check("let count = 42;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_magic_number_in_var_declaration() -> Result<()> {
        let violations = run_check("var value = 3.14;\n");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_configured_numbers() -> Result<()> {
        let violations = run_check_with_config(
            "const x = 1; const y = 0; const z = -1;",
            vec!["0".to_string(), "1".to_string(), "-1".to_string()],
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_multiple_const_declarations() -> Result<()> {
        let violations = run_check("const a = 10, b = 20, c = 30;\n");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_multiple_magic_numbers() -> Result<()> {
        let violations = run_check("calculate(10, 20, 30);\n");
        assert_eq!(violations.len(), 3);
    
        Ok(())}

    #[test]
    fn ignores_numbers_in_comments() -> Result<()> {
        let source = "// const value = 42;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_numbers_in_strings() -> Result<()> {
        let source = r#"const text = "42";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());
    
        Ok(())}
}
