use std::path::Path;

use oxc_ast::ast::ExportDefaultDeclaration;
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::jsx::is_component_file;
use crate::rules::{NO_COMPONENT_DEFAULT_EXPORT_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str =
    "Components must use named exports so imports stay explicit and refactorable.";

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

    let mut visitor = ComponentDefaultExportVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ComponentDefaultExportVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ComponentDefaultExportVisitor<'a, 'f> {
    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        let pos = self.line_index.position_for(decl.span);
        self.violations.push(Violation {
            file: self.file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_COMPONENT_DEFAULT_EXPORT_RULE_ID,
            message: MESSAGE,
            severity: self.severity,
            detail: None,
            subject: None,
        });
        oxc_ast_visit::walk::walk_export_default_declaration(self, decl);
    }
}

#[cfg(test)]
mod tests {
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
    fn reports_default_function_export_in_component_file() {
        let violations = run_check(
            "export default function Button() {}\n",
            "src/components/Button.tsx",
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_default_value_export_in_component_file() {
        let violations = run_check(
            "const Button = () => {};\nexport default Button;\n",
            "src/components/Button.tsx",
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn reports_default_export_in_dot_component_file() {
        let violations = run_check(
            "export default function Button() {}\n",
            "Button.component.tsx",
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_named_exports_in_component_file() {
        let violations = run_check(
            "export function Button() {}\nexport const Modal = () => {};\n",
            "src/components/Button.tsx",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_component_files() {
        let violations = run_check(
            "export default function helper() {}\n",
            "src/utils/helper.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_default_export_in_hook_file() {
        let violations = run_check(
            "export default function useAuth() {}\n",
            "src/hooks/useAuth.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_default_export_in_type_file_outside_components() {
        let violations = run_check(
            "export default interface Config { url: string; }\n",
            "src/types/config.type.ts",
        );
        assert!(violations.is_empty());
    }

    #[test]
    fn reports_multiline_default_export() {
        let violations = run_check("export\n  default Button;\n", "src/components/Button.tsx");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn ignores_export_default_in_comments_and_strings() {
        let source = r#"// export default value;
const text = "export default value";
/* export default value; */
"#;
        let violations = run_check(source, "src/components/Button.tsx");
        assert!(violations.is_empty());
    }
}
