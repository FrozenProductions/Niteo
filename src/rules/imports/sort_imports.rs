use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use oxc_ast::ast::{ImportDeclaration, Program};
use oxc_span::GetSpan;

use crate::config::{ImportGroup, NewlinesBetween, Severity, SortImportsRuleConfig};
use crate::rules::{Fix, SORT_IMPORTS_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import declarations should be sorted by module specifier.";

const BUILTINS: &[&str] = &[
    "assert",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "https",
    "module",
    "net",
    "os",
    "path",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "timers",
    "tls",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "zlib",
];

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

fn classify_import(specifier: &str, internal_glob: Option<&GlobSet>) -> ImportGroup {
    if let Some(stripped) = specifier.strip_prefix("node:")
        && BUILTINS.contains(&stripped)
    {
        return ImportGroup::Builtin;
    }
    if BUILTINS.contains(&specifier) {
        return ImportGroup::Builtin;
    }

    if let Some(glob) = internal_glob
        && glob.is_match(specifier)
    {
        return ImportGroup::Internal;
    }

    if specifier.starts_with("../") {
        ImportGroup::Parent
    } else if specifier.starts_with("./") {
        if is_index_like(specifier) {
            ImportGroup::Index
        } else {
            ImportGroup::Sibling
        }
    } else {
        ImportGroup::External
    }
}

fn is_index_like(specifier: &str) -> bool {
    if specifier.ends_with("/index") {
        return true;
    }
    let segments: Vec<&str> = specifier
        .strip_prefix("./")
        .unwrap_or(specifier)
        .split('/')
        .collect();
    if segments.len() >= 2 {
        let last = segments[segments.len() - 1];
        let parent = segments[segments.len() - 2];
        if last == parent {
            return true;
        }
    }
    false
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

fn build_group_order<'a>(
    group_config: &[Vec<ImportGroup>],
    imports: &[&'a ImportDeclaration<'a>],
    internal_glob: Option<&GlobSet>,
) -> Vec<Vec<(usize, &'a ImportDeclaration<'a>)>> {
    let classified: Vec<(ImportGroup, usize, &ImportDeclaration)> = imports
        .iter()
        .enumerate()
        .map(|(original_index, decl)| {
            let group = classify_import(&decl.source.value, internal_glob);
            (group, original_index, *decl)
        })
        .collect();

    let mut seen_groups: Vec<Vec<ImportGroup>> = group_config.to_vec();
    for (group, _, _) in &classified {
        if !seen_groups.iter().any(|inner| inner.contains(group)) {
            seen_groups.push(vec![*group]);
        }
    }

    let mut ordered: Vec<Vec<(usize, &ImportDeclaration)>> =
        seen_groups.iter().map(|_| Vec::new()).collect();

    for (group, original_index, decl) in &classified {
        if let Some(position) = seen_groups
            .iter()
            .position(|inner| inner.contains(group))
        {
            ordered[position].push((*original_index, decl));
        }
    }

    for bucket in ordered.iter_mut() {
        bucket.sort_by(|a, b| {
            let key_a = import_sort_key(a.1);
            let key_b = import_sort_key(b.1);
            key_a.cmp(&key_b)
        });
    }

    ordered
}

fn count_newlines_between(source: &str, start_offset: usize, end_offset: usize) -> usize {
    source[start_offset..end_offset].matches('\n').count()
}

struct GroupCheckContext<'a, 'b> {
    file: &'a Path,
    line_index: &'a LineIndex,
    severity: Severity,
    group: &'a [&'a ImportDeclaration<'a>],
    group_config: &'a [Vec<ImportGroup>],
    newlines_between: NewlinesBetween,
    internal_glob: Option<&'b GlobSet>,
    source: &'a str,
    program: &'a Program<'a>,
}

fn check_group_grouped(ctx: &GroupCheckContext) -> Vec<Violation> {
    let all_imports: Vec<(usize, &ImportDeclaration)> = ctx
        .program
        .body
        .iter()
        .enumerate()
        .filter_map(|(idx, stmt)| is_import_stmt(stmt).map(|decl| (idx, decl)))
        .collect();

    let global_start_idx = all_imports
        .iter()
        .position(|(_, decl)| decl.span.start == ctx.group[0].span.start)
        .unwrap_or(0);
    let global_end_idx = all_imports
        .iter()
        .rposition(|(_, decl)| decl.span.end == ctx.group[ctx.group.len() - 1].span.end)
        .unwrap_or(all_imports.len().saturating_sub(1));

    let imports_in_range: Vec<(usize, &ImportDeclaration)> = all_imports
        .iter()
        .skip(global_start_idx)
        .take(global_end_idx - global_start_idx + 1)
        .map(|(idx, decl)| (*idx, *decl))
        .collect();

    let ordered = build_group_order(
        ctx.group_config,
        &imports_in_range
            .iter()
            .map(|(_, d)| *d)
            .collect::<Vec<_>>(),
        ctx.internal_glob,
    );

    let flat_expected: Vec<&ImportDeclaration> = ordered
        .iter()
        .flat_map(|bucket| bucket.iter().map(|(_, decl)| *decl))
        .collect();

    let mut violations = Vec::new();

    for (expected_position, expected_decl) in flat_expected.iter().enumerate() {
        let actual_span = expected_decl.span;
        let actual_position = imports_in_range
            .iter()
            .position(|(_, decl)| decl.span.start == actual_span.start)
            .unwrap_or(0);

        if actual_position != expected_position {
            let pos = ctx.line_index.position_for(actual_span);
            violations.push(Violation {
                file: ctx.file.to_path_buf(),
                span: Some(actual_span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_IMPORTS_RULE_ID,
                message: MESSAGE,
                severity: ctx.severity,
                detail: Some(format!(
                    "\"{}\" should appear in a different position",
                    imports_in_range[actual_position].1.source.value
                )),
                subject: Some(imports_in_range[actual_position].1.source.value.to_string()),
            });
        }
    }

    if ctx.newlines_between != NewlinesBetween::Ignore {
        let expected_blank_line_starts: Vec<usize> = ordered
            .windows(2)
            .filter_map(|pair| {
                if pair[0].is_empty() || pair[1].is_empty() {
                    return None;
                }
                let last_of_prev = pair[0].last().unwrap().1.span.end as usize;
                let first_of_next = pair[1].first().unwrap().1.span.start as usize;
                if ctx.newlines_between == NewlinesBetween::Always {
                    if count_newlines_between(ctx.source, last_of_prev, first_of_next) < 2 {
                        Some(last_of_prev)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for start_offset in expected_blank_line_starts {
            let pos = ctx.line_index.position_for(oxc_span::Span::new(
                start_offset as u32,
                start_offset as u32,
            ));
            violations.push(Violation {
                file: ctx.file.to_path_buf(),
                span: Some(oxc_span::Span::new(
                    start_offset as u32,
                    start_offset as u32,
                )),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_IMPORTS_RULE_ID,
                message: "Import groups should be separated by a blank line.",
                severity: ctx.severity,
                detail: Some(
                    "Expected a blank line between import groups with newlines-between: \"always\""
                        .to_string(),
                ),
                subject: None,
            });
        }
    }

    violations
}

fn check_group(
    file: &Path,
    line_index: &LineIndex,
    severity: crate::config::Severity,
    group: &[&ImportDeclaration],
) -> Vec<Violation> {
    let mut violations = Vec::new();

    let keys: Vec<String> = group.iter().map(|decl| import_sort_key(decl)).collect();
    for i in 0..keys.len().saturating_sub(1) {
        if keys[i] > keys[i + 1] {
            let curr = group[i + 1];
            let prev = group[i];
            let pos = line_index.position_for(curr.span);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(curr.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_IMPORTS_RULE_ID,
                message: MESSAGE,
                severity,
                detail: Some(format!(
                    "\"{}\" should appear before \"{}\"",
                    curr.source.value, prev.source.value
                )),
                subject: Some(curr.source.value.to_string()),
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
    config: &SortImportsRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    if !config.groups.is_empty() {
        let internal_glob = build_internal_glob(&config.internal_patterns);
        let groups = find_import_groups(program, source);
        let mut violations = Vec::new();
        for group in &groups {
            let ctx = GroupCheckContext {
                file,
                line_index,
                severity: config.severity,
                group,
                group_config: &config.groups,
                newlines_between: config.newlines_between,
                internal_glob: internal_glob.as_ref(),
                source,
                program,
            };
            violations.extend(check_group_grouped(&ctx));
        }
        return violations;
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
    config: &SortImportsRuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let groups = find_import_groups(program, source);

    if !config.groups.is_empty() {
        return fix_file_grouped(file, program, source, config, &groups);
    }

    fix_file_ungrouped(file, source, config, &groups)
}

fn fix_file_ungrouped(
    file: &Path,
    source: &str,
    _config: &SortImportsRuleConfig,
    groups: &[Vec<&ImportDeclaration>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();

    for group in groups {
        let mut keyed: Vec<(String, &ImportDeclaration)> =
            group.iter().map(|decl| (import_sort_key(decl), *decl)).collect();
        let mut is_sorted = true;
        for window in keyed.windows(2) {
            if window[0].0 > window[1].0 {
                is_sorted = false;
                break;
            }
        }
        if is_sorted {
            continue;
        }

        keyed.sort_by(|a, b| a.0.cmp(&b.0));

        let group_start = group[0].span.start as usize;
        let group_end = group[group.len() - 1].span.end as usize;

        let separator = if group.len() >= 2 {
            &source[group[0].span.end as usize..group[1].span.start as usize]
        } else {
            "\n"
        };

        let snippets: Vec<&str> = keyed
            .iter()
            .map(|(_, decl)| &source[decl.span.start as usize..decl.span.end as usize])
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

fn fix_file_grouped(
    file: &Path,
    program: &Program,
    source: &str,
    config: &SortImportsRuleConfig,
    groups: &[Vec<&ImportDeclaration>],
) -> Vec<Fix> {
    let internal_glob = build_internal_glob(&config.internal_patterns);

    let all_imports: Vec<(usize, &ImportDeclaration)> = program
        .body
        .iter()
        .enumerate()
        .filter_map(|(idx, stmt)| is_import_stmt(stmt).map(|decl| (idx, decl)))
        .collect();

    let mut fixes = Vec::new();

    for group in groups {
        let global_start_idx = all_imports
            .iter()
            .position(|(_, decl)| decl.span.start == group[0].span.start)
            .unwrap_or(0);
        let global_end_idx = all_imports
            .iter()
            .rposition(|(_, decl)| decl.span.end == group[group.len() - 1].span.end)
            .unwrap_or(all_imports.len().saturating_sub(1));

        let imports_in_range: Vec<&ImportDeclaration> = all_imports
            .iter()
            .skip(global_start_idx)
            .take(global_end_idx - global_start_idx + 1)
            .map(|(_, decl)| *decl)
            .collect();

        let ordered = build_group_order(
            &config.groups,
            &imports_in_range,
            internal_glob.as_ref(),
        );

        let group_start = group[0].span.start as usize;
        let group_end = group[group.len() - 1].span.end as usize;

        let separator = if group.len() >= 2 {
            &source[group[0].span.end as usize..group[1].span.start as usize]
        } else {
            "\n"
        };

        let mut snippet_groups: Vec<String> = Vec::new();
        for bucket in &ordered {
            if bucket.is_empty() {
                continue;
            }
            let bucket_snippets: Vec<&str> = bucket
                .iter()
                .map(|(_, decl)| &source[decl.span.start as usize..decl.span.end as usize])
                .collect();
            snippet_groups.push(bucket_snippets.join(separator));
        }

        let replacement = if config.newlines_between == NewlinesBetween::Always {
            snippet_groups.join("\n\n")
        } else {
            snippet_groups.join("\n")
        };

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

fn build_internal_glob(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .ok()?;
        builder.add(glob);
    }
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use super::*;
    use crate::config::{ImportGroup, NewlinesBetween, Severity, SortImportsRuleConfig};
    use crate::fix;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn run_check(source: &str) -> Vec<Violation> {
        run_check_with_config(source, test_config())
    }

    fn run_check_with_config(source: &str, config: SortImportsRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &line_index,
            &config,
        )
    }

    fn run_fix(source: &str) -> Vec<Fix> {
        run_fix_with_config(source, test_config())
    }

    fn run_fix_with_config(source: &str, config: SortImportsRuleConfig) -> Vec<Fix> {
        let allocator = Allocator::default();
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        fix_file(
            Path::new("Component.tsx"),
            &program,
            source,
            &config,
        )
    }

    fn apply_fix(source: &str, fixes: &[Fix]) -> String {
        let edits: Vec<TextEdit> = fixes.iter().flat_map(|f| f.edits.clone()).collect();
        fix::apply_edits(source, &edits)
    }

    fn test_config() -> SortImportsRuleConfig {
        SortImportsRuleConfig {
            severity: Severity::Warn,
            groups: Vec::new(),
            newlines_between: NewlinesBetween::Ignore,
            internal_patterns: Vec::new(),
        }
    }

    fn grouped_config(groups: Vec<Vec<ImportGroup>>) -> SortImportsRuleConfig {
        SortImportsRuleConfig {
            severity: Severity::Warn,
            groups,
            newlines_between: NewlinesBetween::Ignore,
            internal_patterns: Vec::new(),
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

    #[test]
    fn classifies_external_imports() {
        assert_eq!(
            classify_import("react", None),
            ImportGroup::External
        );
        assert_eq!(
            classify_import("lodash", None),
            ImportGroup::External
        );
    }

    #[test]
    fn classifies_builtin_imports() {
        assert_eq!(classify_import("fs", None), ImportGroup::Builtin);
        assert_eq!(
            classify_import("path", None),
            ImportGroup::Builtin
        );
        assert_eq!(
            classify_import("node:fs", None),
            ImportGroup::Builtin
        );
    }

    #[test]
    fn classifies_parent_imports() {
        assert_eq!(
            classify_import("../foo", None),
            ImportGroup::Parent
        );
        assert_eq!(
            classify_import("../../bar", None),
            ImportGroup::Parent
        );
    }

    #[test]
    fn classifies_sibling_imports() {
        assert_eq!(
            classify_import("./foo", None),
            ImportGroup::Sibling
        );
        assert_eq!(
            classify_import("./bar/baz", None),
            ImportGroup::Sibling
        );
    }

    #[test]
    fn classifies_index_imports() {
        assert_eq!(
            classify_import("./foo/index", None),
            ImportGroup::Index
        );
        assert_eq!(classify_import("./foo/foo", None), ImportGroup::Index);
    }

    #[test]
    fn classifies_internal_imports_with_patterns() {
        let glob = build_internal_glob(&["@scope/**".to_string()]).unwrap();
        assert_eq!(
            classify_import("@scope/package", Some(&glob)),
            ImportGroup::Internal
        );
        assert_eq!(
            classify_import("@scope/utils/helper", Some(&glob)),
            ImportGroup::Internal
        );
    }

    #[test]
    fn group_ordering_moves_imports() -> Result<()> {
        let config = grouped_config(vec![
            vec![ImportGroup::External],
            vec![ImportGroup::Sibling],
        ]);
        let source =
            "import \"./local\";\nimport React from \"react\";\nimport \"./other\";\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import React from \"react\";\nimport \"./local\";\nimport \"./other\";\n"
        );
        Ok(())
    }

    #[test]
    fn nested_groups_merge_builtin_external() -> Result<()> {
        let config = grouped_config(vec![
            vec![ImportGroup::Builtin, ImportGroup::External],
            vec![ImportGroup::Sibling],
        ]);
        let source =
            "import \"./local\";\nimport React from \"react\";\nimport fs from \"fs\";\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import fs from \"fs\";\nimport React from \"react\";\nimport \"./local\";\n"
        );
        Ok(())
    }

    #[test]
    fn newlines_between_always_adds_blank_lines() -> Result<()> {
        let mut config = grouped_config(vec![
            vec![ImportGroup::External],
            vec![ImportGroup::Sibling],
        ]);
        config.newlines_between = NewlinesBetween::Always;
        let source =
            "import \"./local\";\nimport React from \"react\";\nimport \"./other\";\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import React from \"react\";\n\nimport \"./local\";\nimport \"./other\";\n"
        );
        Ok(())
    }

    #[test]
    fn backward_compat_blind_line_groups_work_with_empty_groups_config() -> Result<()> {
        let config = test_config();
        let source =
            "import z from \"z\";\nimport y from \"y\";\n\nimport b from \"b\";\nimport a from \"a\";\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "import y from \"y\";\nimport z from \"z\";\n\nimport a from \"a\";\nimport b from \"b\";\n"
        );
        Ok(())
    }
}
