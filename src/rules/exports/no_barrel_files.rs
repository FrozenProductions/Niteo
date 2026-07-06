use std::path::Path;

use oxc_ast::ast::Statement;

use crate::config::BarrelRuleConfig;
use crate::rules::{NO_BARREL_FILES_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Avoid barrel files; import directly from the source module.";

fn is_barrel_file(file: &Path, barrel_names: &[String]) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| barrel_names.iter().any(|n| n == name))
}

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    _line_index: &LineIndex,
    config: &BarrelRuleConfig,
) -> Vec<Violation> {
    if !is_barrel_file(file, &config.barrel_names) {
        return Vec::new();
    }

    let has_re_export = program.body.iter().any(is_re_export);

    if has_re_export {
        vec![Violation {
            file: file.to_path_buf(),
            span: None,
            line: None,
            column: None,
            rule: NO_BARREL_FILES_RULE_ID,
            message: MESSAGE,
            severity: config.severity,
            detail: None,
            subject: None,
        }]
    } else {
        Vec::new()
    }
}

fn is_re_export(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::ExportAllDeclaration(_) => true,
        Statement::ExportNamedDeclaration(decl) => decl.source.is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::check_file;
    use crate::config::{BarrelRuleConfig, Severity};
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(file_name: &str, source: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(Path::new(file_name), &program, &line_index, &test_config())
    }

    #[test]
    fn reports_barrel_file_with_re_exports() -> Result<()> {
        let source = r#"export { Button } from "./Button";
export type { ButtonProps } from "./Button.type";
"#;
        let violations = run_check("index.ts", source);

        assert_eq!(violations.len(), 1);
        assert!(violations[0].line.is_none());
        assert!(violations[0].column.is_none());
    
        Ok(())}

    #[test]
    fn reports_barrel_file_with_namespace_re_exports() -> Result<()> {
        let source = r#"export * from "./Button";
export * as ButtonParts from "./Button.parts";
"#;
        let violations = run_check("index.ts", source);

        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn ignores_non_barrel_files() -> Result<()> {
        let source = r#"export { Button } from "./Button";
"#;
        let violations = run_check("Button.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_index_file_without_re_exports() -> Result<()> {
        let source = r#"const value = 1;
"#;
        let violations = run_check("index.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    fn test_config() -> BarrelRuleConfig {
        BarrelRuleConfig {
            severity: Severity::Warn,
            barrel_names: vec!["index.ts".to_string(), "index.tsx".to_string()],
        }
    }

    #[test]
    fn reports_index_tsx() {
        let source = r#"export { Button } from "./Button";"#;
        let violations = run_check("index.tsx", source);
        assert_eq!(violations.len(), 1);
    }
}
