use std::path::Path;

use oxc_ast::ast::{
    ExportAllDeclaration, ExportDefaultDeclaration, ExportNamedDeclaration, Program,
    TSModuleDeclarationName,
};
use oxc_span::GetSpan;

use crate::config::RuleConfig;
use crate::rules::{Fix, SORT_EXPORTS_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Export declarations should be sorted by exported name.";

#[derive(Debug, Clone)]
enum ExportDecl<'a> {
    Named(&'a ExportNamedDeclaration<'a>),
    Default(&'a ExportDefaultDeclaration<'a>),
    All(&'a ExportAllDeclaration<'a>),
}

impl<'a> ExportDecl<'a> {
    fn span(&self) -> oxc_span::Span {
        match self {
            ExportDecl::Named(d) => d.span,
            ExportDecl::Default(d) => d.span,
            ExportDecl::All(d) => d.span,
        }
    }

    fn sort_key(&self) -> String {
        match self {
            ExportDecl::Default(_) => String::new(),
            ExportDecl::All(decl) => {
                if let Some(exported) = &decl.exported {
                    exported.name().to_lowercase()
                } else {
                    decl.source.value.to_lowercase()
                }
            }
            ExportDecl::Named(decl) => {
                if let Some(ref declaration) = decl.declaration {
                    first_binding_name(declaration).to_lowercase()
                } else {
                    decl.specifiers
                        .first()
                        .map(|spec| spec.local.name().to_lowercase())
                        .unwrap_or_default()
                }
            }
        }
    }
}

fn first_binding_name(declaration: &oxc_ast::ast::Declaration) -> String {
    match declaration {
        oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => var_decl
            .declarations
            .first()
            .and_then(|d| d.id.get_binding_identifier())
            .map(|ident| ident.name.to_string())
            .unwrap_or_default(),
        oxc_ast::ast::Declaration::FunctionDeclaration(func_decl) => func_decl
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_default(),
        oxc_ast::ast::Declaration::ClassDeclaration(class_decl) => class_decl
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_default(),
        oxc_ast::ast::Declaration::TSTypeAliasDeclaration(type_decl) => {
            type_decl.id.name.to_string()
        }
        oxc_ast::ast::Declaration::TSInterfaceDeclaration(iface_decl) => {
            iface_decl.id.name.to_string()
        }
        oxc_ast::ast::Declaration::TSEnumDeclaration(enum_decl) => {
            enum_decl.id.name.to_string()
        }
        oxc_ast::ast::Declaration::TSModuleDeclaration(module_decl) => {
            match &module_decl.id {
                TSModuleDeclarationName::Identifier(id) => id.name.to_string(),
                TSModuleDeclarationName::StringLiteral(lit) => lit.value.to_string(),
            }
        }
        _ => String::new(),
    }
}

fn is_export_stmt<'a>(stmt: &'a oxc_ast::ast::Statement<'a>) -> Option<ExportDecl<'a>> {
    match stmt {
        oxc_ast::ast::Statement::ExportNamedDeclaration(decl) => Some(ExportDecl::Named(decl)),
        oxc_ast::ast::Statement::ExportDefaultDeclaration(decl) => Some(ExportDecl::Default(decl)),
        oxc_ast::ast::Statement::ExportAllDeclaration(decl) => Some(ExportDecl::All(decl)),
        _ => None,
    }
}

fn find_export_groups<'a>(program: &'a Program<'a>, source: &str) -> Vec<Vec<ExportDecl<'a>>> {
    let exports: Vec<(usize, ExportDecl)> = program
        .body
        .iter()
        .enumerate()
        .filter_map(|(idx, stmt)| is_export_stmt(stmt).map(|decl| (idx, decl)))
        .collect();

    if exports.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<Vec<ExportDecl>> = Vec::new();
    let mut current_group: Vec<ExportDecl> = vec![exports[0].1.clone()];

    for window in exports.windows(2) {
        let (prev_idx, _) = window[0];
        let (curr_idx, ref curr_decl) = window[1];

        let has_blank_line = if let (Some(prev_stmt), Some(curr_stmt)) =
            (program.body.get(prev_idx), program.body.get(curr_idx))
        {
            let prev_end = prev_stmt.span().end as usize;
            let curr_start = curr_stmt.span().start as usize;
            let between = &source[prev_end..curr_start];
            between.matches('\n').count() >= 2
        } else {
            false
        };

        if has_blank_line {
            groups.push(current_group);
            current_group = vec![curr_decl.clone()];
        } else {
            current_group.push(curr_decl.clone());
        }
    }
    groups.push(current_group);
    groups
}

fn check_group(
    file: &Path,
    line_index: &LineIndex,
    severity: crate::config::Severity,
    group: &[ExportDecl],
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for window in group.windows(2) {
        let prev_key = window[0].sort_key();
        let curr_key = window[1].sort_key();
        if prev_key > curr_key {
            let pos = line_index.position_for(window[1].span());
            let prev_name = format_export_name(&window[0]);
            let curr_name = format_export_name(&window[1]);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(window[1].span()),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_EXPORTS_RULE_ID,
                message: MESSAGE,
                severity,
                detail: Some(format!(
                    "Export \"{curr_name}\" should appear before \"{prev_name}\""
                )),
                subject: Some(curr_name),
            });
        }
    }

    violations
}

fn format_export_name(decl: &ExportDecl) -> String {
    match decl {
        ExportDecl::Named(d) => {
            if let Some(ref declaration) = d.declaration {
                first_binding_name(declaration)
            } else {
                d.specifiers
                    .first()
                    .map(|spec| spec.local.name().to_string())
                    .unwrap_or_default()
            }
        }
        ExportDecl::Default(_) => "default".to_string(),
        ExportDecl::All(decl) => {
            if let Some(exported) = &decl.exported {
                exported.name().to_string()
            } else {
                format!("* from \"{}\"", decl.source.value)
            }
        }
    }
}

pub fn check_file(
    file: &Path,
    program: &Program,
    source: &str,
    line_index: &LineIndex,
    config: &RuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let groups = find_export_groups(program, source);
    let mut violations = Vec::new();
    for group in &groups {
        violations.extend(check_group(file, line_index, config.severity, group));
    }
    violations
}

pub fn fix_file(file: &Path, program: &Program, source: &str, config: &RuleConfig) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let groups = find_export_groups(program, source);
    let mut fixes = Vec::new();

    for group in &groups {
        let mut sorted: Vec<&ExportDecl> = group.iter().collect();
        let mut is_sorted = true;
        for window in sorted.windows(2) {
            if window[0].sort_key() > window[1].sort_key() {
                is_sorted = false;
                break;
            }
        }
        if is_sorted {
            continue;
        }

        sorted.sort_by_key(|decl| decl.sort_key());

        let group_start = group[0].span().start as usize;
        let group_end = group[group.len() - 1].span().end as usize;

        let separator = if group.len() >= 2 {
            &source[group[0].span().end as usize..group[1].span().start as usize]
        } else {
            "\n"
        };

        let snippets: Vec<&str> = sorted
            .iter()
            .map(|decl| &source[decl.span().start as usize..decl.span().end as usize])
            .collect();

        let replacement = snippets.join(separator);

        fixes.push(Fix {
            file: file.to_path_buf(),
            rule: SORT_EXPORTS_RULE_ID,
            edits: vec![TextEdit {
                start: group_start,
                end: group_end,
                replacement,
            }],
        });
    }

    fixes
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use super::*;
    use crate::config::{RuleConfig, Severity};
    use crate::fix;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn run_check(source: &str) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &line_index,
            &test_config(),
        )
    }

    fn run_fix(source: &str) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        fix_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &test_config(),
        )
    }

    fn apply_fix(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|f| f.edits.clone()).collect();
        fix::apply_edits(source, &edits)
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }

    #[test]
    fn allows_sorted_exports() -> Result<()> {
        let source = "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_unsorted_exports() -> Result<()> {
        let source = "export const c = 3;\nexport const a = 1;\nexport const b = 2;\n";
        let violations = run_check(source);
        assert!(!violations.is_empty());
        Ok(())
    }

    #[test]
    fn fix_sorts_exports() -> Result<()> {
        let source = "export const c = 3;\nexport const a = 1;\nexport const b = 2;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n"
        );
        Ok(())
    }

    #[test]
    fn sorts_case_insensitive() -> Result<()> {
        let source = "export const Zebra = 1;\nexport const apple = 2;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "export const apple = 2;\nexport const Zebra = 1;\n");
        Ok(())
    }

    #[test]
    fn default_export_sorts_first() -> Result<()> {
        let source = "export const b = 2;\nexport default function foo() {}\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export default function foo() {}\nexport const b = 2;\n"
        );
        Ok(())
    }

    #[test]
    fn handles_export_specifiers() -> Result<()> {
        let source = "export { c };\nexport { a };\nexport { b };\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "export { a };\nexport { b };\nexport { c };\n");
        Ok(())
    }

    #[test]
    fn handles_export_all() -> Result<()> {
        let source = "export * from \"./z\";\nexport * from \"./a\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "export * from \"./a\";\nexport * from \"./z\";\n");
        Ok(())
    }

    #[test]
    fn handles_export_namespace() -> Result<()> {
        let source = "export * as Z from \"./z\";\nexport * as A from \"./a\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export * as A from \"./a\";\nexport * as Z from \"./z\";\n"
        );
        Ok(())
    }

    #[test]
    fn handles_mixed_export_kinds() -> Result<()> {
        let source =
            "export const zebra = 1;\nexport default 42;\nexport const apple = 2;\nexport function banana() {}\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export default 42;\nexport const apple = 2;\nexport function banana() {}\nexport const zebra = 1;\n"
        );
        Ok(())
    }

    #[test]
    fn preserves_blank_line_groups() -> Result<()> {
        let source =
            "export const z = 1;\nexport const y = 2;\n\nexport const b = 3;\nexport const a = 4;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert!(fixed.contains("\n\n"));
        assert_eq!(
            fixed,
            "export const y = 2;\nexport const z = 1;\n\nexport const a = 4;\nexport const b = 3;\n"
        );
        Ok(())
    }

    #[test]
    fn handles_type_exports() -> Result<()> {
        let source = "export type { Zebra };\nexport type { Alpha };\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "export type { Alpha };\nexport type { Zebra };\n");
        Ok(())
    }

    #[test]
    fn no_fix_when_sorted() -> Result<()> {
        let source = "export default 1;\nexport const a = 2;\nexport const b = 3;\n";
        let fixes = run_fix(source);
        assert!(fixes.is_empty());
        Ok(())
    }

    #[test]
    fn single_export_no_violation() -> Result<()> {
        let source = "export const x = 1;\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn handles_interface_exports() -> Result<()> {
        let source = "export interface Zebra {}\nexport interface Alpha {}\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export interface Alpha {}\nexport interface Zebra {}\n"
        );
        Ok(())
    }

    #[test]
    fn preserves_crlf_in_groups() -> Result<()> {
        let source =
            "export const c = 3;\r\nexport const a = 1;\r\nexport const b = 2;\r\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export const a = 1;\r\nexport const b = 2;\r\nexport const c = 3;\r\n"
        );
        Ok(())
    }

    #[test]
    fn respects_crlf_blank_line_groups() -> Result<()> {
        let source =
            "export const z = 1;\r\nexport const y = 2;\r\n\r\nexport const b = 3;\r\nexport const a = 4;\r\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export const y = 2;\r\nexport const z = 1;\r\n\r\nexport const a = 4;\r\nexport const b = 3;\r\n"
        );
        Ok(())
    }
}
