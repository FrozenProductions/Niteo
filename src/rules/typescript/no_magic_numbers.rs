use std::path::Path;

use oxc_ast::ast::{
    CallExpression, ComputedMemberExpression, Expression, NewExpression, NumericLiteral,
    ObjectProperty, PropertyKey, StringLiteral, UnaryExpression, UnaryOperator,
};
use oxc_ast_visit::Visit;

use crate::config::NoMagicNumbersRuleConfig;
use crate::rules::{NO_MAGIC_NUMBERS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "No magic numbers. Extract numeric literals to named constants.";
const STRING_MESSAGE: &str =
    "No magic strings. Extract string literals to named constants.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoMagicNumbersRuleConfig,
) -> Vec<Violation> {
    let allowed_values: Vec<f64> = config
        .allowed_numbers
        .iter()
        .filter_map(|allowed| allowed.parse::<f64>().ok())
        .collect();

    let mut visitor = MagicNumberVisitor {
        violations: Vec::new(),
        file,
        line_index,
        config,
        allowed_values,
        current_var_decl_is_const: false,
        in_computed_member: false,
        in_property_key: false,
        in_unary_negation: false,
        in_array_constructor: false,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct MagicNumberVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    config: &'a NoMagicNumbersRuleConfig,
    allowed_values: Vec<f64>,
    current_var_decl_is_const: bool,
    in_computed_member: bool,
    in_property_key: bool,
    in_unary_negation: bool,
    in_array_constructor: bool,
}

impl<'a, 'f> MagicNumberVisitor<'a, 'f> {
    fn is_allowed_number(&self, literal: &NumericLiteral) -> bool {
        let value = if self.in_unary_negation {
            -literal.value
        } else {
            literal.value
        };
        self.allowed_values
            .iter()
            .any(|allowed| (value - allowed).abs() < f64::EPSILON)
    }

    fn is_allowed_string(&self, lit: &StringLiteral) -> bool {
        self.config
            .allowed_strings
            .iter()
            .any(|allowed| lit.value.as_str() == allowed.as_str())
    }
}

impl<'a, 'f> Visit<'a> for MagicNumberVisitor<'a, 'f> {
    fn visit_numeric_literal(&mut self, literal: &NumericLiteral<'a>) {
        if self.in_array_constructor || self.is_allowed_number(literal) {
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

    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        if !self.config.enforce_strings
            || self.in_computed_member
            || self.in_property_key
            || self.current_var_decl_is_const
            || self.is_allowed_string(lit)
        {
            return;
        }

        let pos = self.line_index.position_for(lit.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(lit.span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_MAGIC_NUMBERS_RULE_ID,
            message: STRING_MESSAGE,
            severity: self.config.severity,
            detail: None,
            subject: None,
        });
    }

    fn visit_unary_expression(&mut self, expr: &UnaryExpression<'a>) {
        let saved = self.in_unary_negation;
        self.in_unary_negation = matches!(expr.operator, UnaryOperator::UnaryNegation);
        oxc_ast_visit::walk::walk_unary_expression(self, expr);
        self.in_unary_negation = saved;
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let saved = self.in_array_constructor;
        if let Expression::Identifier(id) = &call.callee
            && id.name == "Array"
        {
            self.in_array_constructor = true;
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
        self.in_array_constructor = saved;
    }

    fn visit_new_expression(&mut self, expr: &NewExpression<'a>) {
        let saved = self.in_array_constructor;
        if let Expression::Identifier(id) = &expr.callee
            && id.name == "Array"
        {
            self.in_array_constructor = true;
        }
        oxc_ast_visit::walk::walk_new_expression(self, expr);
        self.in_array_constructor = saved;
    }

    fn visit_computed_member_expression(
        &mut self,
        expr: &ComputedMemberExpression<'a>,
    ) {
        let saved = self.in_computed_member;

        self.visit_expression(&expr.object);

        self.in_computed_member =
            matches!(&expr.expression, Expression::StringLiteral(_));
        self.visit_expression(&expr.expression);

        self.in_computed_member = saved;
    }

    fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
        let saved = self.in_property_key;

        if !prop.computed {
            self.in_property_key =
                matches!(&prop.key, PropertyKey::StringLiteral(_));
        }

        self.visit_property_key(&prop.key);

        self.in_property_key = false;

        self.visit_expression(&prop.value);

        self.in_property_key = saved;
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
            match init {
                Expression::NumericLiteral(_) => return,
                Expression::UnaryExpression(unary) => {
                    if matches!(unary.operator, UnaryOperator::UnaryNegation)
                        && matches!(&unary.argument, Expression::NumericLiteral(_))
                    {
                        return;
                    }
                }
                Expression::StringLiteral(_) => return,
                _ => {}
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
        run_check_with_config(source, vec![], vec![])
    }

    fn run_check_with_config(
        source: &str,
        allowed_numbers: Vec<String>,
        allowed_strings: Vec<String>,
    ) -> Vec<Violation> {
        run_check_with_full_config(source, allowed_numbers, allowed_strings, false)
    }

    fn run_check_with_full_config(
        source: &str,
        allowed_numbers: Vec<String>,
        allowed_strings: Vec<String>,
        enforce_strings: bool,
    ) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let config = NoMagicNumbersRuleConfig {
            severity: Severity::Warn,
            allowed_numbers,
            allowed_strings,
            enforce_strings,
        };
        check_file(Path::new("test.ts"), &program, &line_index, &config)
    }

    #[test]
    fn reports_magic_number_in_function_call() -> Result<()> {
        let violations = run_check("setTimeout(callback, 3000);\n");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));

        Ok(())
    }

    #[test]
    fn reports_magic_number_in_expression() -> Result<()> {
        let violations = run_check("const result = price * 1.15;\n");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_magic_number_in_array() -> Result<()> {
        let violations = run_check("const array = [1, 2, 3, 42, 5];\n");
        assert_eq!(violations.len(), 5);

        Ok(())
    }

    #[test]
    fn allows_numbers_in_const_declarations() -> Result<()> {
        let violations = run_check("const MAX_SIZE = 100;\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_numbers_in_enum_members() -> Result<()> {
        let violations = run_check("enum Status { Active = 1, Inactive = 0 }\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_numbers_in_type_annotations() -> Result<()> {
        let violations = run_check("type Size = 10 | 20 | 30;\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_numbers_in_jsx_attributes() -> Result<()> {
        let violations = run_check("const element = <div width={100} height={200} />;\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_magic_number_in_let_declaration() -> Result<()> {
        let violations = run_check("let count = 42;\n");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_magic_number_in_var_declaration() -> Result<()> {
        let violations = run_check("var value = 3.14;\n");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_configured_numbers() -> Result<()> {
        let violations = run_check_with_config(
            "const x = 1; const y = 0; const z = -1;",
            vec!["0".to_string(), "1".to_string(), "-1".to_string()],
            vec![],
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_negative_numbers_in_function_args_with_config() -> Result<()> {
        let violations = run_check_with_config(
            "foo(-1);\n",
            vec!["-1".to_string()],
            vec![],
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_unconfigured_negative_numbers() -> Result<()> {
        let violations = run_check("foo(-1);\n");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignores_numeric_args_to_array_constructor() -> Result<()> {
        let violations = run_check("Array(5);\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_numeric_args_to_new_array() -> Result<()> {
        let violations = run_check("new Array(3);\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_multiple_numeric_args_to_array() -> Result<()> {
        let violations = run_check("Array(1, 2, 3);\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn flags_non_array_constructor_numeric_args() -> Result<()> {
        let violations = run_check("Buffer.alloc(5);\n");
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_configured_strings() -> Result<()> {
        let source = r#"dispatch({ type: "FETCH_USERS" });"#;
        let violations = run_check_with_full_config(
            source,
            vec![],
            vec!["FETCH_USERS".to_string()],
            true,
        );
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn flags_strings_not_in_allowed_list() -> Result<()> {
        let source = r#"dispatch({ type: "FETCH_USERS", payload: "data" });"#;
        let violations = run_check_with_full_config(
            source,
            vec![],
            vec!["FETCH_USERS".to_string()],
            true,
        );
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_multiple_const_declarations() -> Result<()> {
        let violations = run_check("const a = 10, b = 20, c = 30;\n");
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_multiple_magic_numbers() -> Result<()> {
        let violations = run_check("calculate(10, 20, 30);\n");
        assert_eq!(violations.len(), 3);

        Ok(())
    }

    #[test]
    fn ignores_numbers_in_comments() -> Result<()> {
        let source = "// const value = 42;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_numbers_in_strings() -> Result<()> {
        let source = r#"const text = "42";"#;
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn does_not_flag_strings_when_enforce_strings_is_false() -> Result<()> {
        let source = r#"fetch("/api/users");"#;
        let violations = run_check(source);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn flags_inline_string_literals() -> Result<()> {
        let source = r#"fetch("/api/users");"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("magic strings"));

        Ok(())
    }

    #[test]
    fn allows_strings_in_const_declarations() -> Result<()> {
        let source = r#"const API_URL = "/api/users";"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_strings_in_enum_members() -> Result<()> {
        let source = "enum Action { FETCH = \"fetch\", UPDATE = \"update\" }\n";
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_string_literal_property_keys() -> Result<()> {
        let source = r#"const obj = { "key": "value" };"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_string_literal_member_expression_keys() -> Result<()> {
        let source = r#"const val = obj["key"];"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn flags_string_literals_in_function_args() -> Result<()> {
        let source = r#"localStorage.getItem("auth_token");"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn flags_multiple_string_literals() -> Result<()> {
        let source = r#"dispatch({ type: "FETCH_USERS", payload: "data" });"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert_eq!(violations.len(), 2);

        Ok(())
    }

    #[test]
    fn allows_strings_in_type_annotations() -> Result<()> {
        let source = "type Status = \"active\" | \"inactive\";\n";
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn allows_strings_in_jsx_attributes() -> Result<()> {
        let source = r#"const el = <div className="container" />;"#;
        let violations =
            run_check_with_full_config(source, vec![], vec![], true);
        assert!(violations.is_empty());

        Ok(())
    }
}
