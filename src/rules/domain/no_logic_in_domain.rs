use std::path::Path;

use oxc_ast::ast::{Declaration, Expression, Statement, VariableDeclarationKind};

use crate::config::structure::DomainConfig;
use crate::config::{RuleConfig, Severity};
use crate::rules::{NO_LOGIC_IN_DOMAIN_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Keep domain files free of logic. Move implementation to feature or service files.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DomainKind {
    Types,
    Constants,
}

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    types: &DomainConfig,
    constants: &DomainConfig,
) -> Vec<Violation> {
    let Some(kind) = classify_domain_file(file, types, constants) else {
        return Vec::new();
    };

    let mut violations = Vec::new();
    for statement in &program.body {
        if let Some(violation) =
            check_statement(file, statement, kind, line_index, config.severity)
        {
            violations.push(violation);
        }
    }

    violations
}

fn classify_domain_file(
    file: &Path,
    types: &DomainConfig,
    constants: &DomainConfig,
) -> Option<DomainKind> {
    if types.matches_file(file) {
        return Some(DomainKind::Types);
    }

    if constants.matches_file(file) {
        return Some(DomainKind::Constants);
    }

    None
}

fn check_statement(
    file: &Path,
    statement: &Statement,
    kind: DomainKind,
    line_index: &LineIndex,
    severity: Severity,
) -> Option<Violation> {
    match statement {
        Statement::FunctionDeclaration(func) => {
            Some(make_violation(file, line_index, func.span, severity))
        }
        Statement::ClassDeclaration(class) => {
            Some(make_violation(file, line_index, class.span, severity))
        }
        Statement::VariableDeclaration(var_decl) => {
            check_variable_declaration(file, var_decl, kind, line_index, severity)
        }
        Statement::ExportNamedDeclaration(export) => export
            .declaration
            .as_ref()
            .and_then(|decl| check_declaration(file, decl, kind, line_index, severity)),
        _ => None,
    }
}

fn check_declaration(
    file: &Path,
    declaration: &Declaration,
    kind: DomainKind,
    line_index: &LineIndex,
    severity: Severity,
) -> Option<Violation> {
    match declaration {
        Declaration::FunctionDeclaration(func) => {
            Some(make_violation(file, line_index, func.span, severity))
        }
        Declaration::ClassDeclaration(class) => {
            Some(make_violation(file, line_index, class.span, severity))
        }
        Declaration::VariableDeclaration(var_decl) => {
            check_variable_declaration(file, var_decl, kind, line_index, severity)
        }
        _ => None,
    }
}

fn check_variable_declaration(
    file: &Path,
    var_decl: &oxc_ast::ast::VariableDeclaration,
    kind: DomainKind,
    line_index: &LineIndex,
    severity: Severity,
) -> Option<Violation> {
    match kind {
        DomainKind::Types => Some(make_violation(file, line_index, var_decl.span, severity)),
        DomainKind::Constants => {
            if var_decl.kind != VariableDeclarationKind::Const {
                return Some(make_violation(file, line_index, var_decl.span, severity));
            }

            let has_function_init = var_decl
                .declarations
                .iter()
                .filter_map(|declarator| declarator.init.as_ref())
                .any(|init| {
                    matches!(
                        init,
                        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                    )
                });

            if has_function_init {
                Some(make_violation(file, line_index, var_decl.span, severity))
            } else {
                None
            }
        }
    }
}

fn make_violation(
    file: &Path,
    line_index: &LineIndex,
    span: oxc_span::Span,
    severity: Severity,
) -> Violation {
    let pos = line_index.position_for(span);
    Violation {
        file: file.to_path_buf(),
        span: Some(span),
        line: Some(pos.line),
        column: Some(pos.column),
        rule: NO_LOGIC_IN_DOMAIN_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    use super::check_file;
    use crate::config::structure::{DomainConfig, ProjectStructureConfig};
    use crate::config::{RuleConfig, Severity};
    use crate::syntax::LineIndex;

    fn default_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    fn default_types() -> DomainConfig {
        ProjectStructureConfig::default().types
    }

    fn default_constants() -> DomainConfig {
        ProjectStructureConfig::default().constants
    }

    fn run_check(file_name: &str, source: &str) -> Vec<crate::rules::Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let source_type = if file_name.ends_with(".tsx") {
            SourceType::tsx()
        } else {
            SourceType::ts()
        };
        let parser_return = Parser::new(&allocator, source, source_type).parse();
        let program = parser_return.program;
        check_file(
            Path::new(file_name),
            &program,
            &line_index,
            &default_config(),
            &default_types(),
            &default_constants(),
        )
    }

    #[test]
    fn reports_function_in_types_folder() -> anyhow::Result<()> {
        let violations = run_check("types/Button.ts", "function handleClick() {}\n");

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));

        Ok(())
    }

    #[test]
    fn allows_const_in_constants_folder() -> anyhow::Result<()> {
        let violations = run_check("constants/routes.ts", "export const ROUTES = { HOME: '/' };\n");

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_function_in_constants_folder() -> anyhow::Result<()> {
        let violations = run_check(
            "constants/routes.ts",
            "function getRoute() { return '/'; }\n",
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_const_arrow_function_in_constants_folder() -> anyhow::Result<()> {
        let violations = run_check("constants/routes.ts", "const getRoute = () => '/';\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_function_in_type_file() -> anyhow::Result<()> {
        let violations = run_check("Button.type.ts", "export function handleClick() {}\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_class_in_constant_file() -> anyhow::Result<()> {
        let violations = run_check("api.constants.ts", "export class ApiClient {}\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn allows_type_declaration_in_type_file() -> anyhow::Result<()> {
        let violations = run_check(
            "Button.type.ts",
            "export type ButtonProps = { label: string };\n",
        );

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_const_in_type_file() -> anyhow::Result<()> {
        let violations = run_check("Button.type.ts", "const VALUE = 1;\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_let_declaration() -> anyhow::Result<()> {
        let violations = run_check("types/state.ts", "let count = 0;\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_async_function() -> anyhow::Result<()> {
        let violations = run_check("types/api.ts", "async function fetchData() {}\n");

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignores_logic_in_comments() -> anyhow::Result<()> {
        let source = "// function handleClick() {}\n/* const value = 1; */\n";
        let violations = run_check("types/test.ts", source);

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_logic_in_strings() -> anyhow::Result<()> {
        let source = r#"const text = "function handleClick() {}";"#;
        let violations = run_check("constants/test.ts", source);

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_keywords_in_regex() -> anyhow::Result<()> {
        let source = r#"export const PATTERN = /\b(function|const|local|export)\b/g;"#;
        let violations = run_check("constants/test.ts", source);

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn ignores_keywords_in_object_literals() -> anyhow::Result<()> {
        let source = r#"export const DOCS = {
    function: "Define a function body.",
    local: "Declare a local variable.",
    typeof: "Capture the inferred type.",
};"#;
        let violations = run_check("constants/test.ts", source);

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn respects_custom_type_folder() -> anyhow::Result<()> {
        let types = DomainConfig {
            folders: vec!["typings".to_string()],
            file_suffixes: vec![".type.ts".to_string(), ".types.ts".to_string()],
        };
        let violations = run_check_with_config(
            "typings/status.ts",
            "function getStatus() {}\n",
            &types,
            &default_constants(),
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn respects_custom_type_suffix() -> anyhow::Result<()> {
        let types = DomainConfig {
            folders: vec!["types".to_string()],
            file_suffixes: vec![".model.ts".to_string()],
        };
        let violations =
            run_check_with_config("User.model.ts", "class User {}\n", &types, &default_constants());

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn respects_custom_constants_folder() -> anyhow::Result<()> {
        let constants = DomainConfig {
            folders: vec!["config".to_string()],
            file_suffixes: vec![".constant.ts".to_string(), ".constants.ts".to_string()],
        };
        let violations = run_check_with_config(
            "config/routes.ts",
            "function getRoute() {}\n",
            &default_types(),
            &constants,
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignores_non_domain_files() -> anyhow::Result<()> {
        let violations = run_check("Button.tsx", "function handleClick() {}\n");

        assert!(violations.is_empty());

        Ok(())
    }

    #[test]
    fn reports_exported_arrow_function_in_constants() -> anyhow::Result<()> {
        let violations = run_check(
            "constants/helpers.ts",
            "export const formatName = (name: string) => name.trim();\n",
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn reports_exported_function_expression_in_constants() -> anyhow::Result<()> {
        let violations = run_check(
            "constants/helpers.ts",
            "export const formatName = function(name: string) { return name.trim(); };\n",
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    fn run_check_with_config(
        file_name: &str,
        source: &str,
        types: &DomainConfig,
        constants: &DomainConfig,
    ) -> Vec<crate::rules::Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let source_type = if file_name.ends_with(".tsx") {
            SourceType::tsx()
        } else {
            SourceType::ts()
        };
        let parser_return = Parser::new(&allocator, source, source_type).parse();
        let program = parser_return.program;
        check_file(
            Path::new(file_name),
            &program,
            &line_index,
            &default_config(),
            types,
            constants,
        )
    }
}
