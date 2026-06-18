use std::path::Path;

use oxc_ast::ast::ExportDefaultDeclaration;
use oxc_ast_visit::Visit;

use crate::config::structure::DomainConfig;
use crate::config::NoDefaultExportRuleConfig;
use crate::jsx::is_component_file;
use crate::rules::{NO_DEFAULT_EXPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const GENERAL_MESSAGE: &str = "Use named exports so imports stay explicit and refactorable.";
const COMPONENT_MESSAGE: &str =
    "Components must use named exports so imports stay explicit and refactorable.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoDefaultExportRuleConfig,
    components: &DomainConfig,
) -> Vec<Violation> {
    if config.components_only && !is_component_file(file, components) {
        return Vec::new();
    }

    let message = if config.components_only {
        COMPONENT_MESSAGE
    } else {
        GENERAL_MESSAGE
    };

    let mut visitor = DefaultExportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        message,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct DefaultExportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    message: &'static str,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for DefaultExportVisitor<'a, 'f> {
    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            span: Some(decl.span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_DEFAULT_EXPORT_RULE_ID,
            message: self.message,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::structure::ProjectStructureConfig;
    use crate::config::{NoDefaultExportRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, file_path: &str, components_only: bool) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        let structure = ProjectStructureConfig::default();
        check_file(
            Path::new(file_path),
            &program,
            &line_index,
            &NoDefaultExportRuleConfig {
                severity: Severity::Warn,
                components_only,
            },
            &structure.components,
        )
    }

    #[test]
    fn reports_default_function_export() -> Result<()> {
        let violations = run_check(
            "export default function Component() {}\n",
            "Component.tsx",
            false,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_default_value_export() -> Result<()> {
        let violations = run_check(
            "const value = 1;\nexport default value;\n",
            "Component.tsx",
            false,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_multiline_default_export() -> Result<()> {
        let violations = run_check(
            "export\n  default value;\n",
            "Component.tsx",
            false,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn allows_named_exports() -> Result<()> {
        let violations = run_check(
            "export function Component() {}\nexport { value } from './value';\n",
            "Component.tsx",
            false,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_export_default_in_comments_and_strings() -> Result<()> {
        let source = r#"// export default value;
const text = "export default value";
/* export default value; */
"#;
        let violations = run_check(source, "Component.tsx", false);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn does_not_match_identifier_fragments() -> Result<()> {
        let source = r#"const exportDefault = true;
const value = "before export default after";
"#;
        let violations = run_check(source, "Component.tsx", false);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn general_mode_also_checks_non_component_files() -> Result<()> {
        let violations = run_check(
            "export default function helper() {}\n",
            "src/utils/helper.ts",
            false,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn components_only_reports_in_component_file() -> Result<()> {
        let violations = run_check(
            "export default function Button() {}\n",
            "src/components/Button.tsx",
            true,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn components_only_reports_default_value_export_in_component_file() -> Result<()> {
        let violations = run_check(
            "const Button = () => {};\nexport default Button;\n",
            "src/components/Button.tsx",
            true,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn components_only_reports_in_dot_component_file() -> Result<()> {
        let violations = run_check(
            "export default function Button() {}\n",
            "Button.component.tsx",
            true,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn components_only_allows_named_exports_in_component_file() -> Result<()> {
        let violations = run_check(
            "export function Button() {}\nexport const Modal = () => {};\n",
            "src/components/Button.tsx",
            true,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn components_only_ignores_non_component_files() -> Result<()> {
        let violations = run_check(
            "export default function helper() {}\n",
            "src/utils/helper.ts",
            true,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn components_only_ignores_default_export_in_hook_file() -> Result<()> {
        let violations = run_check(
            "export default function useAuth() {}\n",
            "src/hooks/useAuth.ts",
            true,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn components_only_reports_multiline_default_export() -> Result<()> {
        let violations = run_check(
            "export\n  default Button;\n",
            "src/components/Button.tsx",
            true,
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}
}
