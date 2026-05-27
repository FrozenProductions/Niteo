use std::path::Path;

use oxc_ast::ast::{ExportDefaultDeclarationKind, Statement, VariableDeclarator};
use oxc_ast_visit::Visit;

use crate::config::EntryFileNoLogicRuleConfig;
use crate::rules::{ENTRY_FILE_NO_LOGIC_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Entry files should delegate logic. Move implementation to dedicated modules.";

const DEFAULT_ENTRY_FILES: &[&str] = &["main", "app", "layout", "page"];

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &EntryFileNoLogicRuleConfig,
) -> Vec<Violation> {
    if !is_entry_file(file, config) {
        return Vec::new();
    }

    let mut visitor = EntryFileVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

fn is_entry_file(file: &Path, config: &EntryFileNoLogicRuleConfig) -> bool {
    let Some(stem) = file.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };

    if DEFAULT_ENTRY_FILES.contains(&stem) {
        return true;
    }

    config
        .entry_files
        .iter()
        .any(|pattern| stem == pattern || stem_matches_suffix(stem, pattern))
}

fn stem_matches_suffix(stem: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('.') {
        stem.ends_with(suffix)
    } else {
        false
    }
}

struct EntryFileVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for EntryFileVisitor<'a, 'f> {
    fn visit_program(&mut self, program: &oxc_ast::ast::Program<'a>) {
        for stmt in &program.body {
            self.check_top_level_statement(stmt);
        }
    }
}

impl<'a, 'f> EntryFileVisitor<'a, 'f> {
    fn check_top_level_statement(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::ImportDeclaration(_) => {}
            Statement::ExportNamedDeclaration(_) => {}
            Statement::ExportAllDeclaration(_) => {}
            Statement::ExportDefaultDeclaration(export_default) => {
                self.check_export_default(export_default);
            }
            Statement::ExpressionStatement(_) => {}
            Statement::TSTypeAliasDeclaration(_) => {}
            Statement::TSInterfaceDeclaration(_) => {}
            Statement::VariableDeclaration(var_decl) => {
                for declarator in &var_decl.declarations {
                    self.check_variable_declarator(declarator);
                }
            }
            Statement::FunctionDeclaration(func) => {
                let pos = self.line_index.position_for(func.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: Some("Function declarations contain logic that should be moved to a dedicated module.".to_string()),
                    subject: func.id.as_ref().map(|id| id.name.to_string()),
                });
            }
            Statement::ClassDeclaration(class) => {
                let pos = self.line_index.position_for(class.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: Some("Class declarations contain logic that should be moved to a dedicated module.".to_string()),
                    subject: class.id.as_ref().map(|id| id.name.to_string()),
                });
            }
            Statement::IfStatement(s) => self.report_control_flow("If statement", s.span),
            Statement::ForStatement(s) => self.report_control_flow("For loop", s.span),
            Statement::ForInStatement(s) => self.report_control_flow("For-in loop", s.span),
            Statement::ForOfStatement(s) => self.report_control_flow("For-of loop", s.span),
            Statement::WhileStatement(s) => self.report_control_flow("While loop", s.span),
            Statement::DoWhileStatement(s) => self.report_control_flow("Do-while loop", s.span),
            Statement::SwitchStatement(s) => self.report_control_flow("Switch statement", s.span),
            Statement::TryStatement(s) => self.report_control_flow("Try-catch block", s.span),
            _ => {}
        }
    }

    fn check_export_default(
        &mut self,
        export_default: &oxc_ast::ast::ExportDefaultDeclaration<'a>,
    ) {
        match &export_default.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(_) => {}
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                let pos = self.line_index.position_for(class.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: Some(
                        "Default-exported class contains logic that should be delegated."
                            .to_string(),
                    ),
                    subject: class.id.as_ref().map(|id| id.name.to_string()),
                });
            }
            _ => {}
        }
    }

    fn check_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let Some(init) = &declarator.init else {
            return;
        };
        match init {
            oxc_ast::ast::Expression::ArrowFunctionExpression(arrow) => {
                let pos = self.line_index.position_for(arrow.span);
                let name = declarator.id.get_identifier_name().map(|n| n.to_string());
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: Some(
                        "Arrow function contains logic that should be moved to a dedicated module."
                            .to_string(),
                    ),
                    subject: name,
                });
            }
            oxc_ast::ast::Expression::FunctionExpression(func) => {
                let pos = self.line_index.position_for(func.span);
                let name = declarator.id.get_identifier_name().map(|n| n.to_string());
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: Some(
                        "Function expression contains logic that should be moved to a dedicated module."
                            .to_string(),
                    ),
                    subject: name,
                });
            }
            _ => {}
        }
    }

    fn report_control_flow(&mut self, kind: &str, span: oxc_span::Span) {
        let pos = self.line_index.position_for(span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: ENTRY_FILE_NO_LOGIC_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: Some(format!(
                "{kind} at top level contains logic that should be delegated."
            )),
            subject: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EntryFileNoLogicRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(file_name: &str, source: &str) -> Vec<Violation> {
        run_check_with_config(file_name, source, &test_config())
    }

    fn run_check_with_config(
        file_name: &str,
        source: &str,
        config: &EntryFileNoLogicRuleConfig,
    ) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let source_type = if file_name.ends_with(".tsx") {
            SourceType::tsx()
        } else {
            SourceType::ts()
        };
        let parser_return = Parser::new(&allocator, source, source_type).parse();
        let program = parser_return.program;
        check_file(Path::new(file_name), &program, &line_index, config)
    }

    fn test_config() -> EntryFileNoLogicRuleConfig {
        EntryFileNoLogicRuleConfig {
            severity: Severity::Warn,
            entry_files: vec![],
        }
    }

    #[test]
    fn skips_non_entry_files() {
        let source = "function helper() { return 42; }\n";
        let violations = run_check("utils.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_imports_in_entry_file() {
        let source = "import { App } from './App';\nimport React from 'react';\n";
        let violations = run_check("main.tsx", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_expression_statements_in_entry_file() {
        let source = "import { createRoot } from 'react-dom/client';\ncreateRoot(document.getElementById('root')!).render(null);\n";
        let violations = run_check("main.tsx", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_named_exports_in_entry_file() {
        let source = "export { App } from './App';\nexport type { Props } from './types';\n";
        let violations = run_check("main.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_export_all_in_entry_file() {
        let source = "export * from './module';\n";
        let violations = run_check("main.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_default_export_function_in_entry_file() {
        let source = "export default function App() { return null; }\n";
        let violations = run_check("app.tsx", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_declarations_in_entry_file() {
        let source =
            "type Config = { port: number };\ninterface Props { children: React.ReactNode }\n";
        let violations = run_check("main.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_simple_const_in_entry_file() {
        let source = "const port = 3000;\nconst name = 'app';\n";
        let violations = run_check("main.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_function_declaration_in_main() {
        let source = "function bootstrap() { console.log('starting'); }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("bootstrap".to_string()));
    }

    #[test]
    fn reports_function_declaration_in_app() {
        let source = "function setup() { return {}; }\n";
        let violations = run_check("app.tsx", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_function_declaration_in_layout() {
        let source = "function computeLayout() { return {}; }\n";
        let violations = run_check("layout.tsx", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_function_declaration_in_page() {
        let source = "function fetchData() { return []; }\n";
        let violations = run_check("page.tsx", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_class_declaration() {
        let source = "class AppService { run() {} }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("AppService".to_string()));
    }

    #[test]
    fn reports_arrow_function_variable() {
        let source = "const bootstrap = () => { console.log('starting'); };\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("bootstrap".to_string()));
    }

    #[test]
    fn reports_function_expression_variable() {
        let source = "const init = function() { return true; };\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, Some("init".to_string()));
    }

    #[test]
    fn reports_if_statement() {
        let source = "if (true) { console.log('yes'); }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_for_loop() {
        let source = "for (let i = 0; i < 10; i++) {}\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_while_loop() {
        let source = "while (true) { break; }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_switch_statement() {
        let source = "switch (x) { case 1: break; }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_try_catch() {
        let source = "try { doSomething(); } catch (e) { console.error(e); }\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_violations() {
        let source = "function a() {}\nfunction b() {}\nclass C {}\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 3);
    }

    #[test]
    fn custom_entry_files_config() {
        let config = EntryFileNoLogicRuleConfig {
            severity: Severity::Warn,
            entry_files: vec!["bootstrap".to_string()],
        };
        let source = "function init() { return true; }\n";
        let violations = run_check_with_config("bootstrap.ts", source, &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn custom_entry_files_non_match() {
        let config = EntryFileNoLogicRuleConfig {
            severity: Severity::Warn,
            entry_files: vec!["bootstrap".to_string()],
        };
        let source = "function init() { return true; }\n";
        let violations = run_check_with_config("server.ts", source, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn allows_const_with_function_call() {
        let source = "const app = createApp();\nconst root = document.getElementById('root');\n";
        let violations = run_check("main.ts", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn typical_main_ts_is_clean() {
        let source = r#"import { createRoot } from 'react-dom/client';
import { App } from './App';

const root = createRoot(document.getElementById('root')!);
root.render(<App />);
"#;
        let violations = run_check("main.tsx", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn typical_app_tsx_is_clean() {
        let source = r#"import { Layout } from './Layout';
import { RouterProvider } from 'react-router-dom';

export default function App() {
    return <Layout><RouterProvider /></Layout>;
}
"#;
        let violations = run_check("app.tsx", source);
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_default_export_class() {
        let source = "export default class App { run() {} }\n";
        let violations = run_check("app.tsx", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_for_in_loop() {
        let source = "for (const key in obj) {}\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_for_of_loop() {
        let source = "for (const item of items) {}\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_do_while_loop() {
        let source = "do { } while (false);\n";
        let violations = run_check("main.ts", source);
        assert_eq!(violations.len(), 1);
    }
}
