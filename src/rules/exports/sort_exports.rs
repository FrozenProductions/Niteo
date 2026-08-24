use std::path::Path;

use oxc_ast::ast::{
    ExportAllDeclaration, ExportDefaultDeclaration, ExportNamedDeclaration, Program,
    TSModuleDeclarationName,
};
use oxc_span::GetSpan;

use crate::config::{ExportGroup, NewlinesBetween, Severity, SortExportsRuleConfig};
use crate::rules::{Fix, SORT_EXPORTS_RULE_ID, TextEdit, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Export declarations should be sorted by exported name.";
const NEWLINE_MESSAGE: &str = "Export declarations should not be separated by a blank line.";
const ALWAYS_NEWLINE_MESSAGE: &str = "Export groups should be separated by a blank line.";

#[derive(Debug, Clone, Copy)]
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

    fn classify(&self) -> ExportGroup {
        match self {
            ExportDecl::Default(_) => ExportGroup::Default,
            ExportDecl::All(decl) => classify_specifier(&decl.source.value),
            ExportDecl::Named(decl) => {
                if let Some(source) = &decl.source {
                    classify_specifier(&source.value)
                } else {
                    ExportGroup::Local
                }
            }
        }
    }
}

fn classify_specifier(specifier: &str) -> ExportGroup {
    if specifier.starts_with("../") {
        ExportGroup::Parent
    } else if specifier.starts_with("./") {
        if is_index_like(specifier) {
            ExportGroup::Index
        } else {
            ExportGroup::Sibling
        }
    } else {
        ExportGroup::External
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

fn find_contiguous_export_blocks<'a>(
    program: &'a Program<'a>,
    source: &str,
) -> Vec<SortableBlock<ExportDecl<'a>>> {
    let mut blocks = Vec::new();
    let mut statements: Vec<ExportDecl<'a>> = Vec::new();

    for statement in &program.body {
        if let Some(declaration) = is_export_stmt(statement) {
            statements.push(declaration);
            continue;
        }

        if let Some(last) = statements.last().copied() {
            let start = leading_trivia_start(program, statements[0].span().start);
            let end = trailing_trivia_end(
                program,
                last.span().end,
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
        let start = leading_trivia_start(program, statements[0].span().start);
        let end = trailing_trivia_end(program, last.span().end, source.len());
        blocks.push(SortableBlock {
            statements,
            start,
            end,
        });
    }

    blocks
}

fn find_export_groups<'a>(
    program: &'a Program<'a>,
    source: &str,
) -> Vec<SortableBlock<ExportDecl<'a>>> {
    let mut groups = Vec::new();

    for block in find_contiguous_export_blocks(program, source) {
        let mut current_group: Vec<ExportDecl<'a>> = Vec::new();
        let mut current_start = block.start;

        for declaration in block.statements {
            let declaration_start = leading_trivia_start(program, declaration.span().start);
            if let Some(previous) = current_group.last().copied()
                && has_blank_line_between(
                    source,
                    previous.span().end as usize,
                    declaration.span().start as usize,
                )
            {
                let end = trailing_trivia_end(program, previous.span().end, declaration_start);
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
            let end = trailing_trivia_end(program, last.span().end, block.end);
            groups.push(SortableBlock {
                statements: current_group,
                start: current_start,
                end,
            });
        }
    }

    groups
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

fn build_group_order<'a>(
    group_config: &[Vec<ExportGroup>],
    exports: &[&'a ExportDecl<'a>],
) -> Vec<Vec<(usize, &'a ExportDecl<'a>)>> {
    let classified: Vec<(ExportGroup, usize, &ExportDecl)> = exports
        .iter()
        .enumerate()
        .map(|(original_index, decl)| {
            let group = decl.classify();
            (group, original_index, *decl)
        })
        .collect();

    let mut seen_groups: Vec<Vec<ExportGroup>> = group_config.to_vec();
    for (group, _, _) in &classified {
        if !seen_groups.iter().any(|inner| inner.contains(group)) {
            seen_groups.push(vec![*group]);
        }
    }

    let mut ordered: Vec<Vec<(usize, &ExportDecl)>> =
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
            let key_a = a.1.sort_key();
            let key_b = b.1.sort_key();
            key_a.cmp(&key_b)
        });
    }

    ordered
}

fn check_group(
    file: &Path,
    line_index: &LineIndex,
    severity: Severity,
    group: &SortableBlock<ExportDecl>,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    let keys: Vec<String> = group
        .statements
        .iter()
        .map(ExportDecl::sort_key)
        .collect();
    for i in 0..keys.len().saturating_sub(1) {
        if keys[i] > keys[i + 1] {
            let curr = &group.statements[i + 1];
            let prev = &group.statements[i];
            let pos = line_index.position_for(curr.span());
            let prev_name = format_export_name(prev);
            let curr_name = format_export_name(curr);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(curr.span()),
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

fn check_group_grouped(
    file: &Path,
    line_index: &LineIndex,
    severity: Severity,
    source: &str,
    config: &SortExportsRuleConfig,
    group: &SortableBlock<ExportDecl>,
) -> Vec<Violation> {
    let exports: Vec<&ExportDecl> = group.statements.iter().collect();
    let ordered = build_group_order(&config.groups, &exports);
    let flat_expected: Vec<&ExportDecl> = ordered
        .iter()
        .flat_map(|bucket| bucket.iter().map(|(_, declaration)| *declaration))
        .collect();
    let mut violations = Vec::new();

    for (expected_position, expected_declaration) in flat_expected.iter().enumerate() {
        let actual_position = exports
            .iter()
            .position(|declaration| declaration.span() == expected_declaration.span())
            .unwrap_or(0);
        if actual_position == expected_position {
            continue;
        }

        let span = expected_declaration.span();
        let pos = line_index.position_for(span);
        let actual = exports[actual_position];
        violations.push(Violation {
            file: file.to_path_buf(),
            span: Some(span),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: SORT_EXPORTS_RULE_ID,
            message: MESSAGE,
            severity,
            detail: Some(format!(
                "Export \"{}\" should appear in a different position",
                format_export_name(actual)
            )),
            subject: Some(format_export_name(actual)),
        });
    }

    if config.newlines_between == NewlinesBetween::Always {
        let bucket_indices: Vec<usize> = exports
            .iter()
            .map(|declaration| {
                ordered
                    .iter()
                    .position(|bucket| {
                        bucket.iter().any(|(_, candidate)| {
                            candidate.span() == declaration.span()
                        })
                    })
                    .unwrap_or(0)
            })
            .collect();

        for (index, pair) in exports.windows(2).enumerate() {
            let Some(current_bucket) = bucket_indices.get(index) else {
                continue;
            };
            let Some(next_bucket) = bucket_indices.get(index + 1) else {
                continue;
            };
            if current_bucket == next_bucket
                || has_blank_line_between(
                    source,
                    pair[0].span().end as usize,
                    pair[1].span().start as usize,
                )
            {
                continue;
            }

            let start_offset = pair[0].span().end as usize;
            let span = oxc_span::Span::new(start_offset as u32, start_offset as u32);
            let pos = line_index.position_for(span);
            violations.push(Violation {
                file: file.to_path_buf(),
                span: Some(span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: SORT_EXPORTS_RULE_ID,
                message: ALWAYS_NEWLINE_MESSAGE,
                severity,
                detail: Some(
                    "Expected a blank line between export groups with newlines-between: \"always\""
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
        rule: SORT_EXPORTS_RULE_ID,
        message: NEWLINE_MESSAGE,
        severity,
        detail: Some(
            "Expected no blank line between export declarations with newlines-between: \"never\""
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
    blocks: &[SortableBlock<ExportDecl>],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for block in blocks {
        for pair in block.statements.windows(2) {
            if has_blank_line_between(
                source,
                pair[0].span().end as usize,
                pair[1].span().start as usize,
            ) {
                violations.push(newline_violation(
                    file,
                    line_index,
                    severity,
                    pair[0].span().end as usize,
                ));
            }
        }
    }
    violations
}

pub fn check_file(
    file: &Path,
    program: &Program,
    source: &str,
    line_index: &LineIndex,
    config: &SortExportsRuleConfig,
) -> Vec<Violation> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let blocks = find_contiguous_export_blocks(program, source);
    let groups = find_export_groups(program, source);
    let mut violations = Vec::new();
    if config.groups.is_empty() {
        for group in &groups {
            violations.extend(check_group(file, line_index, config.severity, group));
        }
    } else {
        for group in &blocks {
            violations.extend(check_group_grouped(
                file,
                line_index,
                config.severity,
                source,
                config,
                group,
            ));
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
    config: &SortExportsRuleConfig,
) -> Vec<Fix> {
    if !config.severity.is_enabled() {
        return Vec::new();
    }

    let blocks = find_contiguous_export_blocks(program, source);
    let groups = find_export_groups(program, source);
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
    groups: &[SortableBlock<ExportDecl>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();

    for group in groups {
        let mut keyed: Vec<(String, &ExportDecl)> =
            group
                .statements
                .iter()
                .map(|decl| (decl.sort_key(), decl))
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

        let ordered: Vec<&ExportDecl> = keyed.iter().map(|(_, decl)| *decl).collect();
        let replacement = render_exports(source, program, group, &ordered, None);
        let original = source.get(group.start..group.end).unwrap_or("");
        if replacement == original {
            continue;
        }

        fixes.push(Fix {
            file: file.to_path_buf(),
            rule: SORT_EXPORTS_RULE_ID,
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
    config: &SortExportsRuleConfig,
    groups: &[SortableBlock<ExportDecl>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();

    for group in groups {
        let exports: Vec<&ExportDecl> = group.statements.iter().collect();
        let ordered_buckets = build_group_order(&config.groups, &exports);
        let mut ordered = Vec::new();
        let mut bucket_indices = Vec::new();
        for (bucket_index, bucket) in ordered_buckets.iter().enumerate() {
            for (_, declaration) in bucket {
                ordered.push(*declaration);
                bucket_indices.push(bucket_index);
            }
        }
        let replacement = render_exports(
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
            rule: SORT_EXPORTS_RULE_ID,
            edits: vec![TextEdit {
                start: group.start,
                end: group.end,
                replacement,
            }],
        });
    }

    fixes
}

fn export_item_ranges(
    program: &Program,
    group: &SortableBlock<ExportDecl>,
) -> Vec<(usize, usize)> {
    group
        .statements
        .iter()
        .enumerate()
        .map(|(index, declaration)| {
            let start = leading_trivia_start(program, declaration.span().start);
            let upper_bound = group
                .statements
                .get(index + 1)
                .map(|next| leading_trivia_start(program, next.span().start))
                .unwrap_or(group.end);
            let end = trailing_trivia_end(program, declaration.span().end, upper_bound);
            (start, end)
        })
        .collect()
}

fn export_snippet<'a>(
    source: &'a str,
    group: &SortableBlock<ExportDecl>,
    ranges: &[(usize, usize)],
    declaration: &ExportDecl,
) -> &'a str {
    let index = group
        .statements
        .iter()
        .position(|candidate| candidate.span() == declaration.span())
        .unwrap_or(0);
    let (start, end) = ranges
        .get(index)
        .copied()
        .unwrap_or((declaration.span().start as usize, declaration.span().end as usize));
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

fn render_exports(
    source: &str,
    program: &Program,
    group: &SortableBlock<ExportDecl>,
    ordered: &[&ExportDecl],
    grouped: Option<(&[usize], NewlinesBetween)>,
) -> String {
    let ranges = export_item_ranges(program, group);
    let separators: Vec<&str> = ranges
        .windows(2)
        .map(|pair| source.get(pair[0].1..pair[1].0).unwrap_or(""))
        .collect();
    let newline = line_ending(source, group.start, group.end);
    let mut replacement = String::new();

    for (index, declaration) in ordered.iter().enumerate() {
        replacement.push_str(export_snippet(source, group, &ranges, declaration));
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
    blocks: &[SortableBlock<ExportDecl>],
) -> Vec<Fix> {
    let mut fixes = Vec::new();
    for block in blocks {
        for pair in block.statements.windows(2) {
            if !has_blank_line_between(
                source,
                pair[0].span().end as usize,
                pair[1].span().start as usize,
            ) {
                continue;
            }
            let next_start = leading_trivia_start(program, pair[1].span().start);
            let previous_end = trailing_trivia_end(program, pair[0].span().end, next_start);
            let replacement = remove_blank_lines(source, program, previous_end, next_start);
            let original = source.get(previous_end..next_start).unwrap_or("");
            if replacement == original {
                continue;
            }
            fixes.push(Fix {
                file: file.to_path_buf(),
                rule: SORT_EXPORTS_RULE_ID,
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::path::Path;

    use super::*;
    use crate::config::{ExportGroup, NewlinesBetween, Severity, SortExportsRuleConfig};
    use crate::fix;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn run_check(source: &str) -> Vec<Violation> {
        run_check_with_config(source, test_config())
    }

    fn run_check_with_config(source: &str, config: SortExportsRuleConfig) -> Vec<Violation> {
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

    fn run_fix_with_config(source: &str, config: SortExportsRuleConfig) -> Vec<Fix> {
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

    fn test_config() -> SortExportsRuleConfig {
        SortExportsRuleConfig {
            severity: Severity::Warn,
            groups: Vec::new(),
            newlines_between: NewlinesBetween::Ignore,
        }
    }

    fn grouped_config(groups: Vec<Vec<ExportGroup>>) -> SortExportsRuleConfig {
        SortExportsRuleConfig {
            severity: Severity::Warn,
            groups,
            newlines_between: NewlinesBetween::Ignore,
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

    #[test]
    fn group_ordering_external_before_sibling() -> Result<()> {
        let config = grouped_config(vec![
            vec![ExportGroup::External],
            vec![ExportGroup::Sibling],
        ]);
        let source =
            "export * from \"./local\";\nexport * from \"react\";\nexport * from \"./other\";\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export * from \"react\";\nexport * from \"./local\";\nexport * from \"./other\";\n"
        );
        Ok(())
    }

    #[test]
    fn group_ordering_local_after_reexports() -> Result<()> {
        let config = grouped_config(vec![
            vec![ExportGroup::Default],
            vec![ExportGroup::External, ExportGroup::Sibling],
            vec![ExportGroup::Local],
        ]);
        let source =
            "export const b = 2;\nexport * from \"react\";\nexport default 42;\nexport const a = 1;\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export default 42;\nexport * from \"react\";\nexport const a = 1;\nexport const b = 2;\n"
        );
        Ok(())
    }

    #[test]
    fn newlines_between_always_adds_blank_lines_for_exports() -> Result<()> {
        let mut config = grouped_config(vec![
            vec![ExportGroup::Default],
            vec![ExportGroup::Local],
        ]);
        config.newlines_between = NewlinesBetween::Always;
        let source = "export const b = 2;\nexport default 42;\nexport const a = 1;\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export default 42;\n\nexport const a = 1;\nexport const b = 2;\n"
        );
        Ok(())
    }

    #[test]
    fn backward_compat_empty_groups_still_sorts() -> Result<()> {
        let config = test_config();
        let source = "export const c = 3;\nexport const a = 1;\nexport const b = 2;\n";
        let fixes = run_fix_with_config(source, config);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n"
        );
        Ok(())
    }

    #[test]
    fn never_sorts_across_an_executable_statement() -> Result<()> {
        let source =
            "export const z = 1;\nconst keep = true;\nexport const a = 2;\n";
        let fixes = run_fix(source);
        assert!(fixes.is_empty());
        assert_eq!(apply_fix(source, &fixes), source);

        let config = grouped_config(vec![
            vec![ExportGroup::Local],
            vec![ExportGroup::Sibling],
        ]);
        let grouped_fixes = run_fix_with_config(source, config);
        assert!(grouped_fixes.is_empty());
        assert_eq!(apply_fix(source, &grouped_fixes), source);
        Ok(())
    }

    #[test]
    fn moves_leading_comments_with_exports() -> Result<()> {
        let source =
            "// Z comment\nexport const z = 1;\n// A comment\nexport const a = 2;\n";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(
            fixed,
            "// A comment\nexport const a = 2;\n// Z comment\nexport const z = 1;\n"
        );
        Ok(())
    }

    #[test]
    fn newline_modes_have_matching_export_diagnostics_and_fixes() -> Result<()> {
        let groups = vec![
            vec![ExportGroup::External],
            vec![ExportGroup::Sibling],
        ];
        let source = "export * from \"react\";\nexport * from \"./local\";\n";

        let mut always = grouped_config(groups.clone());
        always.newlines_between = NewlinesBetween::Always;
        let always_violations = run_check_with_config(source, always.clone());
        assert!(!always_violations.is_empty());
        let always_fixed = apply_fix(source, &run_fix_with_config(source, always.clone()));
        assert_eq!(
            always_fixed,
            "export * from \"react\";\n\nexport * from \"./local\";\n"
        );
        assert!(run_check_with_config(&always_fixed, always).is_empty());

        let never_source = "export * from \"react\";\n\nexport * from \"./local\";\n";
        let mut never = grouped_config(groups.clone());
        never.newlines_between = NewlinesBetween::Never;
        assert!(!run_check_with_config(never_source, never.clone()).is_empty());
        let never_fixed =
            apply_fix(never_source, &run_fix_with_config(never_source, never.clone()));
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
        let source = "export const z = 1;\r\nexport const a = 2;";
        let fixes = run_fix(source);
        let fixed = apply_fix(source, &fixes);
        assert_eq!(fixed, "export const a = 2;\r\nexport const z = 1;");
        Ok(())
    }

    #[test]
    fn applying_export_sort_fix_twice_is_idempotent() -> Result<()> {
        let source = "export const z = 1;\nexport const a = 2;\n";
        let once = apply_fix(source, &run_fix(source));
        let second_fixes = run_fix(&once);
        assert!(second_fixes.is_empty());
        assert_eq!(apply_fix(&once, &second_fixes), once);
        Ok(())
    }

    #[test]
    fn never_fixes_order_and_blank_lines_in_one_pass() -> Result<()> {
        let mut config = grouped_config(vec![
            vec![ExportGroup::External],
            vec![ExportGroup::Sibling],
        ]);
        config.newlines_between = NewlinesBetween::Never;
        let source = "export * from \"./local\";\n\nexport * from \"react\";\n";
        let fixed = apply_fix(source, &run_fix_with_config(source, config.clone()));
        assert_eq!(
            fixed,
            "export * from \"react\";\nexport * from \"./local\";\n"
        );
        assert!(run_check_with_config(&fixed, config.clone()).is_empty());
        assert!(run_fix_with_config(&fixed, config).is_empty());
        Ok(())
    }
}
