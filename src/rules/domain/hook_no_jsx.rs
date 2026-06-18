use std::path::Path;

use oxc_ast::ast::{JSXElement, JSXFragment};
use oxc_ast_visit::Visit;

use crate::config::RuleConfig;
use crate::config::structure::DomainConfig;
use crate::rules::{HOOK_NO_JSX_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Hook files should not contain JSX. Extract UI into a separate component.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
    hooks: &DomainConfig,
) -> Vec<Violation> {
    if !hooks.matches_file(file) {
        return Vec::new();
    }

    let mut visitor = HookJsxVisitor { found: None };
    visitor.visit_program(program);

    visitor
        .found
        .map(|span| {
            let pos = line_index.position_for(span);
            Violation {
                file: file.to_path_buf(),
                span: Some(span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: HOOK_NO_JSX_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: None,
                subject: None,
            }
        })
        .into_iter()
        .collect()
}

struct HookJsxVisitor {
    found: Option<oxc_span::Span>,
}

impl<'a> Visit<'a> for HookJsxVisitor {
    fn visit_jsx_element(&mut self, el: &JSXElement<'a>) {
        if self.found.is_none() {
            self.found = Some(el.span);
        }
    }

    fn visit_jsx_fragment(&mut self, frag: &JSXFragment<'a>) {
        if self.found.is_none() {
            self.found = Some(frag.span);
        }
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
        let hooks = ProjectStructureConfig::default().hooks;
        check_file(
            Path::new(file_path),
            &program,
            &line_index,
            &test_config(),
            &hooks,
        )
    }

    #[test]
    fn reports_jsx_in_hook_file() -> Result<()> {
        let violations = run_check(
            "export function useAuth() {\n  return <div>Loading</div>;\n}\n",
            "src/hooks/useAuth.ts",
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    
        Ok(())}

    #[test]
    fn reports_jsx_in_dot_hook_file() -> Result<()> {
        let violations = run_check(
            "export function useAuth() {\n  return <p>Hello</p>;\n}\n",
            "useAuth.hook.ts",
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_jsx_in_dot_hooks_file() -> Result<()> {
        let violations = run_check(
            "export function useAuth() {\n  return <span>Hi</span>;\n}\n",
            "useAuth.hooks.ts",
        );
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_hook_without_jsx() -> Result<()> {
        let violations = run_check(
            "export function useAuth() {\n  return { user: null };\n}\n",
            "src/hooks/useAuth.ts",
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_jsx_in_non_hook_file() -> Result<()> {
        let violations = run_check(
            "export function Auth() {\n  return <div>Login</div>;\n}\n",
            "src/components/Auth.tsx",
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_jsx_in_comments_and_strings() -> Result<()> {
        let source = r#"// return <div>Loading</div>;
const text = "<p>Hello</p>";
"#;
        let violations = run_check(source, "src/hooks/useAuth.ts");
        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
