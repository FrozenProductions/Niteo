use std::path::Path;

use oxc_ast::ast::Statement;
use oxc_span::GetSpan;

use crate::config::RuleConfig;
use crate::rules::{NO_LOGIC_IN_BARREL_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Keep barrel files limited to re-exports.";
const BARREL_FILE_NAME: &str = "index.ts";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    if file.file_name().and_then(|name| name.to_str()) != Some(BARREL_FILE_NAME) {
        return Vec::new();
    }

    for stmt in &program.body {
        if !is_re_export(stmt) {
            let pos = line_index.position_for(stmt.span());
            return vec![Violation {
                file: file.to_path_buf(),
                span: Some(stmt.span()),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_LOGIC_IN_BARREL_RULE_ID,
                message: MESSAGE,
                severity: config.severity,
                detail: None,
                subject: None,
            }];
        }
    }

    Vec::new()
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
    use crate::config::{RuleConfig, Severity};
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
    fn allows_named_re_exports() -> Result<()> {
        let source = r#"export { Button } from "./Button";
export type { ButtonProps } from "./Button.type";
"#;
        let violations = run_check("index.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_namespace_re_exports() -> Result<()> {
        let source = r#"export * from "./Button";
export * as ButtonParts from "./Button.parts";
"#;
        let violations = run_check("index.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_multiline_re_exports() -> Result<()> {
        let source = r#"export {
  Button,
  type ButtonProps,
} from "./Button";
"#;
        let violations = run_check("index.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_non_barrel_files() -> Result<()> {
        let source = r#"const value = 1;"#;
        let violations = run_check("Button.ts", source);

        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_imports_in_barrels() -> Result<()> {
        let source = r#"import { Button } from "./Button";
export { Button };
"#;
        let violations = run_check("index.ts", source);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_local_exports_in_barrels() -> Result<()> {
        let source = r#"export { Button };
"#;
        let violations = run_check("index.ts", source);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    #[test]
    fn reports_logic_after_re_exports() -> Result<()> {
        let source = r#"export { Button } from "./Button";

const value = 1;
"#;
        let violations = run_check("index.ts", source);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    
        Ok(())}

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
