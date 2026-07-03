use std::path::Path;

use oxc_ast::ast::{ImportDeclaration, Program};
use oxc_span::GetSpan;

use crate::config::RuleConfig;
use crate::rules::{Fix, SORT_IMPORTS_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import declarations should be sorted by module specifier.";

fn is_import_stmt<'a>(
    stmt: &'a oxc_ast::ast::Statement<'a>,
) -> Option<&'a ImportDeclaration<'a>> {
    match stmt {
        oxc_ast::ast::Statement::ImportDeclaration(decl) => Some(decl.as_ref()),
        _ => None,
    }
}

fn import_sort_key(decl: &ImportDeclaration) -> String {
    decl.source.value.to_lowercase()
}

fn find_import_groups<'a>(
    program: &'a Program<'a>,
    source: &str,
) -> Vec<Vec<&'a ImportDeclaration<'a>>> {
    let imports: Vec<(usize, &ImportDeclaration)> = program
        .body
        .iter()
        .enumerate()
        .filter_map(|(idx, stmt)| is_import_stmt(stmt).map(|decl| (idx, decl)))
        .collect();

    if imports.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<Vec<&ImportDeclaration>> = Vec::new();
    let mut current_group: Vec<&ImportDeclaration> = vec![imports[0].1];

    for window in imports.windows(2) {
        let (prev_idx, _prev_decl) = window[0];
        let (curr_idx, curr_decl) = window[1];

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
            current_group = vec![curr_decl];
        } else {
            current_group.push(curr_decl);
        }
    }
    groups.push(current_group);
    groups
}

fn check_group(
    file: &Path,
    line_index: &LineIndex,
    severity: crate::config::Severity,
    group: &[&ImportDeclaration],
) -> Vec<Violation> {
    let mut violations = Vec::new();

    for window in group.windows(2) {
        let prev_key = import_sort_key(window[0]);
        let curr_key = import_sort_key(window[1]);
        if prev_key > curr_key {
            let pos = line_index.position_for(window[1].span);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(window[1].span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_IMPORTS_RULE_ID,
                message: MESSAGE,
                severity,
                detail: Some(format!(
                    "\"{}\" should appear before \"{}\"",
                    window[1].source.value,
                    window[0].source.value
                )),
                subject: Some(window[1].source.value.to_string()),
            });
        }
    }

    violations
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

    let groups = find_import_groups(program, source);
    let mut violations = Vec::new();
    for group in &groups {
        violations.extend(check_group(file, line_index, config.severity, group));
    }
    violations
}

pub fn fix_file(
    file: &Path,
    program: &Program,
    source: &str,
    config: &RuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let groups = find_import_groups(program, source);
    let mut fixes = Vec::new();

    for group in &groups {
        let mut sorted: Vec<&&ImportDeclaration> = group.iter().collect();
        let mut is_sorted = true;
        for window in sorted.windows(2) {
            if import_sort_key(window[0]) > import_sort_key(window[1]) {
                is_sorted = false;
                break;
            }
        }
        if is_sorted {
            continue;
        }

        sorted.sort_by_key(|decl| import_sort_key(decl));

        let group_start = group[0].span.start as usize;
        let group_end = group[group.len() - 1].span.end as usize;

        let separator = if group.len() >= 2 {
            &source[group[0].span.end as usize..group[1].span.start as usize]
        } else {
            "\n"
        };

        let snippets: Vec<&str> = sorted
            .iter()
            .map(|decl| &source[decl.span.start as usize..decl.span.end as usize])
            .collect();

        let replacement = snippets.join(separator);

        fixes.push(Fix {
            file: file.to_path_buf(),
            rule: SORT_IMPORTS_RULE_ID,
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
    fn allows_sorted_imports() -> Result<()> {
        let source = "import a from \"a\";\nimport b from \"b\";\nimport c from \"c\";\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn reports_unsorted_imports() -> Result<()> {
        let source = "import c from \"c\";\nimport a from \"a\";\nimport b from \"b\";\n";
        let violations = run_check(source);
        assert!(!violations.is_empty());
        Ok(())
    }

    #[test]
    fn respects_blank_line_groups() -> Result<()> {
        let source =
            "import c from \"c\";\nimport a from \"a\";\n\nimport z from \"z\";\nimport y from \"y\";\n";
        let violations = run_check(source);
        assert!(!violations.is_empty());
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert!(fixed.contains("\n\n"));
        Ok(())
    }

    #[test]
    fn fix_sorts_imports() -> Result<()> {
        let source = "import c from \"c\";\nimport a from \"a\";\nimport b from \"b\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import a from \"a\";\nimport b from \"b\";\nimport c from \"c\";\n"
        );
        Ok(())
    }

    #[test]
    fn fix_preserves_blank_line_groups() -> Result<()> {
        let source =
            "import z from \"z\";\nimport y from \"y\";\n\nimport b from \"b\";\nimport a from \"a\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import y from \"y\";\nimport z from \"z\";\n\nimport a from \"a\";\nimport b from \"b\";\n"
        );
        Ok(())
    }

    #[test]
    fn sorts_case_insensitive() -> Result<()> {
        let source = "import Zebra from \"zebra\";\nimport apple from \"apple\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import apple from \"apple\";\nimport Zebra from \"zebra\";\n"
        );
        Ok(())
    }

    #[test]
    fn handles_side_effect_imports() -> Result<()> {
        let source = "import \"./z.css\";\nimport \"./a.css\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "import \"./a.css\";\nimport \"./z.css\";\n");
        Ok(())
    }

    #[test]
    fn handles_type_imports() -> Result<()> {
        let source =
            "import type { Zebra } from \"./z\";\nimport type { Alpha } from \"./a\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import type { Alpha } from \"./a\";\nimport type { Zebra } from \"./z\";\n"
        );
        Ok(())
    }

    #[test]
    fn handles_mixed_import_kinds() -> Result<()> {
        let source =
            "import c from \"c\";\nimport * as a from \"a\";\nimport { b } from \"b\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import * as a from \"a\";\nimport { b } from \"b\";\nimport c from \"c\";\n"
        );
        Ok(())
    }

    #[test]
    fn no_fix_when_already_sorted() -> Result<()> {
        let source = "import a from \"a\";\nimport b from \"b\";\n";
        let fixes = run_fix(source);
        assert!(fixes.is_empty());
        Ok(())
    }

    #[test]
    fn single_import_no_violation() -> Result<()> {
        let source = "import x from \"x\";\n";
        let violations = run_check(source);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn preserves_crlf_in_groups() -> Result<()> {
        let source = "import c from \"c\";\r\nimport a from \"a\";\r\nimport b from \"b\";\r\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import a from \"a\";\r\nimport b from \"b\";\r\nimport c from \"c\";\r\n"
        );
        Ok(())
    }

    #[test]
    fn respects_crlf_blank_line_groups() -> Result<()> {
        let source =
            "import z from \"z\";\r\nimport y from \"y\";\r\n\r\nimport b from \"b\";\r\nimport a from \"a\";\r\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import y from \"y\";\r\nimport z from \"z\";\r\n\r\nimport a from \"a\";\r\nimport b from \"b\";\r\n"
        );
        Ok(())
    }
}
