use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use oxc_ast::ast::{ImportDeclaration, Program};
use oxc_span::GetSpan;

use crate::config::{ImportGroup, NewlinesBetween, Severity, SortImportsRuleConfig};
use crate::rules::{Fix, SORT_IMPORTS_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Import declarations should be sorted by module specifier.";
const NEWLINE_MESSAGE: &str = "Import declarations should not be separated by a blank line.";
const ALWAYS_NEWLINE_MESSAGE: &str = "Import groups should be separated by a blank line.";

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

#[derive(Debug)]
struct SortableBlock<T> {
    statements: Vec<T>,
    start: usize,
    end: usize,
}

fn leading_trivia_start(program: &Program, statement_start: u32) -> usize {
    program
        .comments
        .iter()
        .filter(|comment| comment.is_leading() && comment.attached_to == statement_start)
        .map(|comment| comment.span.start as usize)
        .min()
        .unwrap_or(statement_start as usize)
}

fn trailing_trivia_end(program: &Program, statement_end: u32, upper_bound: usize) -> usize {
    program
        .comments
        .iter()
        .filter(|comment| {
            comment.is_trailing()
                && comment.span.start >= statement_end
                && comment.span.end as usize <= upper_bound
        })
        .map(|comment| comment.span.end as usize)
        .max()
        .unwrap_or(statement_end as usize)
}

fn has_blank_line_between(source: &str, start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }

    let bytes = source.as_bytes().get(start..end).unwrap_or(&[]);
    let mut saw_first_line_break = false;
    let mut line_has_content = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        let next_byte = bytes.get(index + 1).copied();

        if in_line_comment {
            if byte == b'\n' {
                if saw_first_line_break && !line_has_content {
                    return true;
                }
                saw_first_line_break = true;
                line_has_content = false;
                in_line_comment = false;
            } else {
                line_has_content = true;
            }
            index += 1;
            continue;
        }

        if in_block_comment {
            if byte == b'\n' {
                saw_first_line_break = true;
                line_has_content = false;
            } else {
                line_has_content = true;
            }
            if byte == b'*' && next_byte == Some(b'/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if byte == b'/' && next_byte == Some(b'/') {
            line_has_content = true;
            in_line_comment = true;
            index += 2;
            continue;
        }
        if byte == b'/' && next_byte == Some(b'*') {
            line_has_content = true;
            in_block_comment = true;
            index += 2;
            continue;
        }

        if byte == b'\n' {
            if saw_first_line_break && !line_has_content {
                return true;
            }
            saw_first_line_break = true;
            line_has_content = false;
        } else if !byte.is_ascii_whitespace() {
            line_has_content = true;
        }
        index += 1;
    }
    false
}

fn find_contiguous_import_blocks<'a>(
    program: &'a Program<'a>,
    source: &str,
) -> Vec<SortableBlock<&'a ImportDeclaration<'a>>> {
    let mut blocks = Vec::new();
    let mut statements = Vec::new();

    for statement in &program.body {
        if let Some(declaration) = is_import_stmt(statement) {
            statements.push(declaration);
            continue;
        }

        if let Some(last) = statements.last().copied() {
            let start = leading_trivia_start(program, statements[0].span.start);
            let end = trailing_trivia_end(
                program,
                last.span.end,
                statement.span().start as usize,
            );
            blocks.push(SortableBlock {
                statements: std::mem::take(&mut statements),
                start,
                end,
            });
        }
    }

    if let Some(last) = statements.last().copied() {
        let start = leading_trivia_start(program, statements[0].span.start);
        let end = trailing_trivia_end(program, last.span.end, source.len());
        blocks.push(SortableBlock {
            statements,
            start,
            end,
        });
    }

    blocks
}

fn find_import_groups<'a>(
    program: &'a Program<'a>,
    source: &str,
) -> Vec<SortableBlock<&'a ImportDeclaration<'a>>> {
    let mut groups = Vec::new();

    for block in find_contiguous_import_blocks(program, source) {
        let mut current_group: Vec<&'a ImportDeclaration<'a>> = Vec::new();
        let mut current_start = block.start;

        for declaration in block.statements {
            let declaration_start = leading_trivia_start(program, declaration.span.start);
            if let Some(previous) = current_group.last().copied()
                && has_blank_line_between(
                    source,
                    previous.span.end as usize,
                    declaration.span.start as usize,
                )
            {
                let end = trailing_trivia_end(program, previous.span.end, declaration_start);
                groups.push(SortableBlock {
                    statements: std::mem::take(&mut current_group),
                    start: current_start,
                    end,
                });
                current_start = declaration_start;
            }
            current_group.push(declaration);
        }

        if let Some(last) = current_group.last().copied() {
            let end = trailing_trivia_end(program, last.span.end, block.end);
            groups.push(SortableBlock {
                statements: current_group,
                start: current_start,
                end,
            });
        }
    }

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

struct GroupCheckContext<'a, 'b> {
    file: &'a Path,
    line_index: &'a LineIndex,
    severity: Severity,
    group: &'a SortableBlock<&'a ImportDeclaration<'a>>,
    group_config: &'a [Vec<ImportGroup>],
    newlines_between: NewlinesBetween,
    internal_glob: Option<&'b GlobSet>,
    source: &'a str,
}

fn check_group_grouped(ctx: &GroupCheckContext) -> Vec<Violation> {
    let imports_in_range = &ctx.group.statements;
    let ordered = build_group_order(ctx.group_config, imports_in_range, ctx.internal_glob);

    let flat_expected: Vec<&ImportDeclaration> = ordered
        .iter()
        .flat_map(|bucket| bucket.iter().map(|(_, decl)| *decl))
        .collect();

    let mut violations = Vec::new();

    for (expected_position, expected_decl) in flat_expected.iter().enumerate() {
        let actual_span = expected_decl.span;
        let actual_position = imports_in_range
            .iter()
            .position(|decl| decl.span.start == actual_span.start)
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
                    imports_in_range[actual_position].source.value
                )),
                subject: Some(imports_in_range[actual_position].source.value.to_string()),
            });
        }
    }

    if ctx.newlines_between == NewlinesBetween::Always {
        let bucket_indices: Vec<usize> = imports_in_range
            .iter()
            .map(|declaration| {
                ordered
                    .iter()
                    .position(|bucket| {
                        bucket.iter().any(|(_, candidate)| {
                            candidate.span.start == declaration.span.start
                        })
                    })
                    .unwrap_or(0)
            })
            .collect();

        for (index, pair) in imports_in_range.windows(2).enumerate() {
            let Some(current_bucket) = bucket_indices.get(index) else {
                continue;
            };
            let Some(next_bucket) = bucket_indices.get(index + 1) else {
                continue;
            };
            if current_bucket == next_bucket
                || has_blank_line_between(
                    ctx.source,
                    pair[0].span.end as usize,
                    pair[1].span.start as usize,
                )
            {
                continue;
            }

            let start_offset = pair[0].span.end as usize;
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
                message: ALWAYS_NEWLINE_MESSAGE,
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

fn newline_violation(
    file: &Path,
    line_index: &LineIndex,
    severity: Severity,
    start_offset: usize,
) -> Violation {
    let span = oxc_span::Span::new(start_offset as u32, start_offset as u32);
    let pos = line_index.position_for(span);
    Violation {
        file: file.to_path_buf(),
        span: Some(span),
        line: Some(pos.line),
        column: Some(pos.column),
        rule: SORT_IMPORTS_RULE_ID,
        message: NEWLINE_MESSAGE,
        severity,
        detail: Some(
            "Expected no blank line between import declarations with newlines-between: \"never\""
                .to_string(),
        ),
        subject: None,
    }
}

fn check_never_newlines(
    file: &Path,
    line_index: &LineIndex,
    severity: Severity,
    source: &str,
    blocks: &[SortableBlock<&ImportDeclaration>],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for block in blocks {
        for pair in block.statements.windows(2) {
            if has_blank_line_between(
                source,
                pair[0].span.end as usize,
                pair[1].span.start as usize,
            ) {
                violations.push(newline_violation(
                    file,
                    line_index,
                    severity,
                    pair[0].span.end as usize,
                ));
            }
        }
    }
    violations
}

fn check_group(
    file: &Path,
    line_index: &LineIndex,
    severity: crate::config::Severity,
    group: &SortableBlock<&ImportDeclaration>,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    let keys: Vec<String> = group
        .statements
        .iter()
        .map(|decl| import_sort_key(decl))
        .collect();
    for i in 0..keys.len().saturating_sub(1) {
        if keys[i] > keys[i + 1] {
            let curr = group.statements[i + 1];
            let prev = group.statements[i];
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

    let blocks = find_contiguous_import_blocks(program, source);
    let groups = find_import_groups(program, source);
    let mut violations = Vec::new();

    if !config.groups.is_empty() {
        let internal_glob = build_internal_glob(&config.internal_patterns);
        for group in &blocks {
            let ctx = GroupCheckContext {
                file,
                line_index,
                severity: config.severity,
                group,
                group_config: &config.groups,
                newlines_between: config.newlines_between,
                internal_glob: internal_glob.as_ref(),
                source,
            };
            violations.extend(check_group_grouped(&ctx));
        }
    } else {
        for group in &groups {
            violations.extend(check_group(file, line_index, config.severity, group));
        }
    }

    if config.newlines_between == NewlinesBetween::Never {
        violations.extend(check_never_newlines(
            file,
            line_index,
            config.severity,
            source,
            &blocks,
        ));
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

    let blocks = find_contiguous_import_blocks(program, source);
    let groups = find_import_groups(program, source);
    if !config.groups.is_empty() {
        return fix_file_grouped(file, program, source, config, &blocks);
    }

    let mut fixes = fix_file_ungrouped(file, program, source, &groups);
    if config.newlines_between == NewlinesBetween::Never {
        fixes.extend(fix_never_newlines(file, program, source, &blocks));
    }
    fixes
}

fn fix_file_ungrouped(
    file: &Path,
    program: &Program,
    source: &str,
    groups: &[SortableBlock<&ImportDeclaration>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();

    for group in groups {
        let mut keyed: Vec<(String, &ImportDeclaration)> =
            group
                .statements
                .iter()
                .map(|decl| (import_sort_key(decl), *decl))
                .collect();
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

        let ordered: Vec<&ImportDeclaration> = keyed.iter().map(|(_, decl)| *decl).collect();
        let replacement = render_imports(source, program, group, &ordered, None);
        let original = source.get(group.start..group.end).unwrap_or("");
        if replacement == original {
            continue;
        }

        fixes.push(Fix {
            file: file.to_path_buf(),
            rule: SORT_IMPORTS_RULE_ID,
            edits: vec![TextEdit {
                start: group.start,
                end: group.end,
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
    groups: &[SortableBlock<&ImportDeclaration>],
) -> Vec<Fix> {
    let internal_glob = build_internal_glob(&config.internal_patterns);
    let mut fixes = Vec::new();

    for group in groups {
        let ordered_buckets = build_group_order(
            &config.groups,
            &group.statements,
            internal_glob.as_ref(),
        );
        let mut ordered = Vec::new();
        let mut bucket_indices = Vec::new();
        for (bucket_index, bucket) in ordered_buckets.iter().enumerate() {
            for (_, declaration) in bucket {
                ordered.push(*declaration);
                bucket_indices.push(bucket_index);
            }
        }
        let replacement = render_imports(
            source,
            program,
            group,
            &ordered,
            Some((&bucket_indices, config.newlines_between)),
        );
        let original = source.get(group.start..group.end).unwrap_or("");
        if replacement == original {
            continue;
        }

        fixes.push(Fix {
            file: file.to_path_buf(),
            rule: SORT_IMPORTS_RULE_ID,
            edits: vec![TextEdit {
                start: group.start,
                end: group.end,
                replacement,
            }],
        });
    }

    fixes
}

fn import_item_ranges(
    program: &Program,
    group: &SortableBlock<&ImportDeclaration>,
) -> Vec<(usize, usize)> {
    group
        .statements
        .iter()
        .enumerate()
        .map(|(index, declaration)| {
            let start = leading_trivia_start(program, declaration.span.start);
            let upper_bound = group
                .statements
                .get(index + 1)
                .map(|next| leading_trivia_start(program, next.span.start))
                .unwrap_or(group.end);
            let end = trailing_trivia_end(program, declaration.span.end, upper_bound);
            (start, end)
        })
        .collect()
}

fn import_snippet<'a>(
    source: &'a str,
    group: &SortableBlock<&ImportDeclaration>,
    ranges: &[(usize, usize)],
    declaration: &ImportDeclaration,
) -> &'a str {
    let index = group
        .statements
        .iter()
        .position(|candidate| candidate.span.start == declaration.span.start)
        .unwrap_or(0);
    let (start, end) = ranges
        .get(index)
        .copied()
        .unwrap_or((declaration.span.start as usize, declaration.span.end as usize));
    source.get(start..end).unwrap_or("")
}

fn line_ending(source: &str, start: usize, end: usize) -> &'static str {
    if source
        .get(start..end)
        .unwrap_or("")
        .contains("\r\n")
    {
        "\r\n"
    } else {
        "\n"
    }
}

fn render_imports(
    source: &str,
    program: &Program,
    group: &SortableBlock<&ImportDeclaration>,
    ordered: &[&ImportDeclaration],
    grouped: Option<(&[usize], NewlinesBetween)>,
) -> String {
    let ranges = import_item_ranges(program, group);
    let separators: Vec<&str> = ranges
        .windows(2)
        .map(|pair| source.get(pair[0].1..pair[1].0).unwrap_or(""))
        .collect();
    let newline = line_ending(source, group.start, group.end);
    let mut replacement = String::new();

    for (index, declaration) in ordered.iter().enumerate() {
        replacement.push_str(import_snippet(source, group, &ranges, declaration));
        if index + 1 >= ordered.len() {
            continue;
        }

        let original_separator = separators.get(index).copied().unwrap_or(newline);
        let separator = if let Some((bucket_indices, newlines_between)) = grouped {
            let has_blank_line = ranges
                .get(index)
                .zip(ranges.get(index + 1))
                .is_some_and(|(previous, next)| {
                    has_blank_line_between(source, previous.1, next.0)
                });
            match newlines_between {
                NewlinesBetween::Always
                    if bucket_indices.get(index) != bucket_indices.get(index + 1)
                        && !has_blank_line =>
                {
                    let mut separator = String::from(newline);
                    separator.push_str(newline);
                    separator
                }
                NewlinesBetween::Never => ranges
                    .get(index)
                    .zip(ranges.get(index + 1))
                    .map(|(previous, next)| {
                        remove_blank_lines(source, program, previous.1, next.0)
                    })
                    .unwrap_or_else(|| original_separator.to_string()),
                _ => original_separator.to_string(),
            }
        } else {
            original_separator.to_string()
        };
        replacement.push_str(&separator);
    }

    replacement
}

fn remove_blank_lines(source: &str, program: &Program, start: usize, end: usize) -> String {
    let gap = source.get(start..end).unwrap_or("");
    let mut replacement = String::with_capacity(gap.len());
    let mut offset = 0;

    for (index, chunk) in gap.split_inclusive('\n').enumerate() {
        let chunk_start = start + offset;
        let chunk_end = chunk_start + chunk.len();
        offset += chunk.len();
        let is_blank = chunk
            .trim_matches(|character: char| character.is_ascii_whitespace())
            .is_empty();
        let contains_comment = program.comments.iter().any(|comment| {
            (comment.span.start as usize) < chunk_end
                && (comment.span.end as usize) > chunk_start
        });
        if index > 0 && chunk.ends_with('\n') && is_blank && !contains_comment {
            continue;
        }
        replacement.push_str(chunk);
    }

    replacement
}

fn fix_never_newlines(
    file: &Path,
    program: &Program,
    source: &str,
    blocks: &[SortableBlock<&ImportDeclaration>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();
    for block in blocks {
        for pair in block.statements.windows(2) {
            if !has_blank_line_between(
                source,
                pair[0].span.end as usize,
                pair[1].span.start as usize,
            ) {
                continue;
            }
            let next_start = leading_trivia_start(program, pair[1].span.start);
            let previous_end = trailing_trivia_end(program, pair[0].span.end, next_start);
            let replacement = remove_blank_lines(source, program, previous_end, next_start);
            let original = source.get(previous_end..next_start).unwrap_or("");
            if replacement == original {
                continue;
            }
            fixes.push(Fix {
                file: file.to_path_buf(),
                rule: SORT_IMPORTS_RULE_ID,
                edits: vec![TextEdit {
                    start: previous_end,
                    end: next_start,
                    replacement,
                }],
            });
        }
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

    #[test]
    fn never_sorts_across_an_executable_statement() -> Result<()> {
        let source =
            "import z from \"z\";\nconst keep = true;\nimport a from \"a\";\n";
        let fixes = run_fix(source);
        assert!(fixes.is_empty());
        assert_eq!(apply_fix(source, &fixes), source);

        let config = grouped_config(vec![
            vec![ImportGroup::External],
            vec![ImportGroup::Sibling],
        ]);
        let grouped_fixes = run_fix_with_config(source, config);
        assert!(grouped_fixes.is_empty());
        assert_eq!(apply_fix(source, &grouped_fixes), source);
        Ok(())
    }

    #[test]
    fn moves_leading_comments_with_imports() -> Result<()> {
        let source =
            "// Z comment\nimport z from \"z\";\n// A comment\nimport a from \"a\";\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "// A comment\nimport a from \"a\";\n// Z comment\nimport z from \"z\";\n"
        );
        Ok(())
    }

    #[test]
    fn newline_modes_have_matching_import_diagnostics_and_fixes() -> Result<()> {
        let groups = vec![
            vec![ImportGroup::External],
            vec![ImportGroup::Sibling],
        ];
        let source = "import React from \"react\";\nimport \"./local\";\n";

        let mut always = grouped_config(groups.clone());
        always.newlines_between = NewlinesBetween::Always;
        let always_violations = run_check_with_config(source, always.clone());
        assert!(!always_violations.is_empty());
        let always_fixed = apply_fix(source, &run_fix_with_config(source, always.clone()));
        assert_eq!(
            always_fixed,
            "import React from \"react\";\n\nimport \"./local\";\n"
        );
        assert!(run_check_with_config(&always_fixed, always).is_empty());

        let never_source = "import React from \"react\";\n\nimport \"./local\";\n";
        let mut never = grouped_config(groups.clone());
        never.newlines_between = NewlinesBetween::Never;
        assert!(!run_check_with_config(never_source, never.clone()).is_empty());
        let never_fixed = apply_fix(never_source, &run_fix_with_config(never_source, never.clone()));
        assert_eq!(never_fixed, source);
        assert!(run_check_with_config(&never_fixed, never).is_empty());

        let mut ignore = grouped_config(groups);
        ignore.newlines_between = NewlinesBetween::Ignore;
        assert!(run_check_with_config(never_source, ignore.clone()).is_empty());
        assert!(run_fix_with_config(never_source, ignore).is_empty());
        Ok(())
    }

    #[test]
    fn preserves_crlf_without_adding_an_eof_newline() -> Result<()> {
        let source = "import c from \"c\";\r\nimport a from \"a\";";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "import a from \"a\";\r\nimport c from \"c\";");
        Ok(())
    }

    #[test]
    fn applying_import_sort_fix_twice_is_idempotent() -> Result<()> {
        let source = "import c from \"c\";\nimport a from \"a\";\n";
        let once = apply_fix(source, &run_fix(source));
        let second_fixes = run_fix(&once);
        assert!(second_fixes.is_empty());
        assert_eq!(apply_fix(&once, &second_fixes), once);
        Ok(())
    }

    #[test]
    fn never_fixes_order_and_blank_lines_in_one_pass() -> Result<()> {
        let mut config = grouped_config(vec![
            vec![ImportGroup::External],
            vec![ImportGroup::Sibling],
        ]);
        config.newlines_between = NewlinesBetween::Never;
        let source = "import \"./local\";\n\nimport React from \"react\";\n";
        let fixed = apply_fix(source, &run_fix_with_config(source, config.clone()));
        assert_eq!(
            fixed,
            "import React from \"react\";\nimport \"./local\";\n"
        );
        assert!(run_check_with_config(&fixed, config.clone()).is_empty());
        assert!(run_fix_with_config(&fixed, config).is_empty());
        Ok(())
    }
}
