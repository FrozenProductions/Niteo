use std::path::Path;

use oxc_ast::ast::{
    Declaration, ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
    TSModuleDeclarationName,
};
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::jsx::is_component_file;
use crate::rules::{COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Component files must only export components. Move utilities, types, and hooks to separate files.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    components: &DomainConfig,
) -> Vec<Violation> {
    if !is_component_file(file, components) {
        return Vec::new();
    }

    if file.extension().and_then(|ext| ext.to_str()) != Some("tsx") {
        return Vec::new();
    }

    let mut visitor = ComponentFileVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ComponentFileVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

fn is_pascal_case(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_uppercase())
}

impl<'a, 'f> Visit<'a> for ComponentFileVisitor<'a, 'f> {
    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        let mut is_namespace = false;

        if let Some(declaration) = &decl.declaration {
            is_namespace = matches!(declaration, Declaration::TSModuleDeclaration(_));
            for (span, subject) in non_component_subjects(declaration) {
                let pos = self.line_index.position_for(span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: None,
                    subject,
                });
            }
        }

        for specifier in &decl.specifiers {
            let name = specifier.local.name().as_str();
            if !is_pascal_case(name) {
                let pos = self.line_index.position_for(specifier.span);
                self.violations.push(Violation {
                    file: self.file.to_path_buf(),
                    line: Some(pos.line),
                    column: Some(pos.column),
                    rule: COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID,
                    message: MESSAGE,
                    severity: self.severity,
                    detail: None,
                    subject: Some(name.to_string()),
                });
            }
        }

        if !is_namespace {
            walk::walk_export_named_declaration(self, decl);
        }
    }

    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        if !is_default_export_component(decl) {
            let pos = self.line_index.position_for(decl.span);
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: COMPONENT_FILE_ONLY_COMPONENTS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail: None,
                subject: Some("default".to_string()),
            });
        }
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

fn non_component_subjects(decl: &Declaration) -> Vec<(oxc_span::Span, Option<String>)> {
    match decl {
        Declaration::FunctionDeclaration(func) => {
            if let Some(id) = &func.id
                && !is_pascal_case(id.name.as_str())
            {
                return vec![(id.span, Some(id.name.to_string()))];
            }
            vec![]
        }
        Declaration::ClassDeclaration(class) => {
            if let Some(id) = &class.id
                && !is_pascal_case(id.name.as_str())
            {
                return vec![(id.span, Some(id.name.to_string()))];
            }
            vec![]
        }
        Declaration::VariableDeclaration(var_decl) => var_decl
            .declarations
            .iter()
            .filter_map(|declarator| {
                let name = match &declarator.id {
                    oxc_ast::ast::BindingPattern::BindingIdentifier(id) => id,
                    _ => return None,
                };
                if !is_pascal_case(name.name.as_str()) {
                    Some((name.span, Some(name.name.to_string())))
                } else {
                    None
                }
            })
            .collect(),
        Declaration::TSTypeAliasDeclaration(type_alias) => {
            vec![(type_alias.span, Some(type_alias.id.name.to_string()))]
        }
        Declaration::TSInterfaceDeclaration(interface) => {
            vec![(interface.span, Some(interface.id.name.to_string()))]
        }
        Declaration::TSEnumDeclaration(enum_decl) => {
            vec![(enum_decl.span, Some(enum_decl.id.name.to_string()))]
        }
        Declaration::TSModuleDeclaration(module) => {
            let name = match &module.id {
                TSModuleDeclarationName::Identifier(id) => id.name.to_string(),
                TSModuleDeclarationName::StringLiteral(lit) => lit.value.to_string(),
            };
            vec![(module.span, Some(name))]
        }
        Declaration::TSImportEqualsDeclaration(_) => vec![],
        Declaration::TSGlobalDeclaration(_) => vec![],
    }
}

fn is_default_export_component(decl: &ExportDefaultDeclaration) -> bool {
    match &decl.declaration {
        ExportDefaultDeclarationKind::FunctionDeclaration(func) => func
            .id
            .as_ref()
            .is_some_and(|id| is_pascal_case(id.name.as_str())),
        ExportDefaultDeclarationKind::ClassDeclaration(class) => class
            .id
            .as_ref()
            .is_some_and(|id| is_pascal_case(id.name.as_str())),
        ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::structure::ProjectStructureConfig;
    use crate::config::{RuleConfig, Severity};
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
        let structure = ProjectStructureConfig::default();
        check_file(
            Path::new(file_path),
            &program,
            &line_index,
            &test_config(),
            &structure.components,
        )
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn allows_pascal_case_function_in_components_folder() -> Result<()> {
        let source = "export function Button() { return null; }\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_pascal_case_arrow_in_components_folder() -> Result<()> {
        let source = "export const Modal = () => { return null; };\n";
        let violations = run_check(source, "src/components/Modal.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_pascal_case_class_in_components_folder() -> Result<()> {
        let source = "export class Card extends Component {}\n";
        let violations = run_check(source, "src/components/Card.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_camel_case_function_in_components_folder() -> Result<()> {
        let source = "export function formatDate() { return ''; }\n";
        let violations = run_check(source, "src/components/formatDate.tsx");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("formatDate"));
    
        Ok(())}

    #[test]
    fn reports_camel_case_arrow_in_components_folder() -> Result<()> {
        let source = "export const getData = () => { return {}; };\n";
        let violations = run_check(source, "src/components/getData.tsx");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("getData"));
    
        Ok(())}

    #[test]
    fn reports_type_alias_in_components_folder() -> Result<()> {
        let source = "export type User = { id: string; };\n";
        let violations = run_check(source, "src/components/User.tsx");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("User"));
    
        Ok(())}

    #[test]
    fn reports_interface_in_components_folder() -> Result<()> {
        let source = "export interface Props { name: string; }\n";
        let violations = run_check(source, "src/components/Props.tsx");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("Props"));
    
        Ok(())}

    #[test]
    fn reports_enum_in_components_folder() -> Result<()> {
        let source = "export enum Size { Small, Large }\n";
        let violations = run_check(source, "src/components/Size.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_namespace_in_components_folder() -> Result<()> {
        let source = "export namespace Utils { export function go() {} }\n";
        let violations = run_check(source, "src/components/Utils.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_default_export_interface_in_components_folder() -> Result<()> {
        let source = "export default interface Config { url: string; }\n";
        let violations = run_check(source, "src/components/Config.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_default_export_pascal_case_function() -> Result<()> {
        let source = "export default function App() { return null; }\n";
        let violations = run_check(source, "src/components/App.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_default_export_expression() -> Result<()> {
        let source = "export default memo(() => { return null; });\n";
        let violations = run_check(source, "src/components/App.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_export_specifier_with_camel_case_name() -> Result<()> {
        let source = "const helper = () => {};\nexport { helper };\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_export_specifier_with_pascal_case_name() -> Result<()> {
        let source = "const Icon = () => null;\nexport { Icon };\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_non_component_files() -> Result<()> {
        let source = "export function formatDate() { return ''; }\n";
        let violations = run_check(source, "src/utils/format.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_in_dot_component_file() -> Result<()> {
        let source = "export function Button() { return null; }\n";
        let violations = run_check(source, "Button.component.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_in_dot_component_file() -> Result<()> {
        let source = "export function helper() { return null; }\n";
        let violations = run_check(source, "Button.component.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_destructured_exports() -> Result<()> {
        let source = "export const { format } = someModule;\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_screaming_snake_case_constants() -> Result<()> {
        let source = "export const API_URL = 'https://example.com';\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_camel_case_non_function_variable() -> Result<()> {
        let source = "export const pageSize = 10;\n";
        let violations = run_check(source, "src/components/Button.tsx");
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_type_file_in_components_folder() -> Result<()> {
        let source = "export type Account = { id: string; };\n";
        let violations = run_check(source, "src/components/account.type.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_types_file_in_components_folder() -> Result<()> {
        let source =
            "export type Account = { id: string; };\nexport type User = { name: string; };\n";
        let violations = run_check(source, "src/components/account.types.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_constant_file_in_components_folder() -> Result<()> {
        let source = "export const SIZES = { small: 10, large: 20 };\n";
        let violations = run_check(source, "src/components/sizes.constant.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_constants_file_in_components_folder() -> Result<()> {
        let source = "export const SIZES = { small: 10, large: 20 };\n";
        let violations = run_check(source, "src/components/sizes.constants.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_hook_file_in_components_folder() -> Result<()> {
        let source = "export function useButton() { return { ref: null }; }\n";
        let violations = run_check(source, "src/components/useButton.hook.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_ts_files_in_components_folder() -> Result<()> {
        let source = "export const helper = () => {};\nexport type Props = { id: string };\n";
        let violations = run_check(source, "src/components/utils.ts");
        assert!(violations.is_empty());
    
        Ok(())}
}
