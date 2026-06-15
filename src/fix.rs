use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::rules::{FileContext, FileRule, Fix, TextEdit};
use crate::syntax::source_type_from_path;

pub struct ApplyFixOptions {
    pub dry_run: bool,
    pub validate_parse: bool,
}

pub struct FixOutcome {
    pub fixed_files: Vec<PathBuf>,
    pub rejected_overlapping: usize,
    pub rejected_stale: usize,
    pub rejected_invalid: usize,
    pub rejected_parse: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditValidationResult {
    Valid,
    Overlapping,
    Invalid { reason: String },
}

pub fn apply_fixes(fixes: Vec<Fix>, options: ApplyFixOptions) -> Result<FixOutcome> {
    let mut by_file: HashMap<PathBuf, Vec<TextEdit>> = HashMap::new();
    for fix in fixes {
        by_file.entry(fix.file).or_default().extend(fix.edits);
    }

    let mut fixed_files = Vec::new();
    let mut rejected_overlapping = 0;
    let mut rejected_stale = 0;
    let mut rejected_invalid = 0;
    let mut rejected_parse = 0;

    for (file_path, mut edits) in by_file {
        edits.sort_by_key(|edit| edit.start);

        let source = std::fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        match validate_edits(&source, &edits) {
            EditValidationResult::Valid => {}
            EditValidationResult::Overlapping => {
                rejected_overlapping += edits.len();
                continue;
            }
            EditValidationResult::Invalid { reason } => {
                eprintln!(
                    "warning: rejected edits in {}: {}",
                    file_path.display(),
                    reason
                );
                rejected_invalid += edits.len();
                continue;
            }
        }

        let modified = apply_edits(&source, &edits);

        if options.validate_parse
            && let Some(source_type) = source_type_from_path(&file_path)
        {
            let allocator = oxc_allocator::Allocator::default();
            let parser_return = oxc_parser::Parser::new(&allocator, &modified, source_type).parse();
            if parser_return.panicked {
                eprintln!(
                    "warning: rejected edits in {}: fixed source is not parseable",
                    file_path.display()
                );
                rejected_parse += edits.len();
                continue;
            }
        }

        if !options.dry_run {
            let current_source = std::fs::read_to_string(&file_path)
                .with_context(|| format!("failed to read {}", file_path.display()))?;

            if current_source != source {
                rejected_stale += edits.len();
                continue;
            }

            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            std::fs::write(&file_path, &modified)
                .with_context(|| format!("failed to write {}", file_path.display()))?;
        }

        fixed_files.push(file_path);
    }

    Ok(FixOutcome {
        fixed_files,
        rejected_overlapping,
        rejected_stale,
        rejected_invalid,
        rejected_parse,
    })
}

pub fn validate_edits(source: &str, edits: &[TextEdit]) -> EditValidationResult {
    let source_len = source.len();
    for edit in edits {
        if edit.start > edit.end {
            return EditValidationResult::Invalid {
                reason: format!("start {} is greater than end {}", edit.start, edit.end),
            };
        }
        if edit.end > source_len {
            return EditValidationResult::Invalid {
                reason: format!("end {} exceeds source length {}", edit.end, source_len),
            };
        }
        if !source.is_char_boundary(edit.start) || !source.is_char_boundary(edit.end) {
            return EditValidationResult::Invalid {
                reason: format!(
                    "edit range {}-{} is not on a UTF-8 character boundary",
                    edit.start, edit.end
                ),
            };
        }
    }

    if has_overlap(edits) {
        return EditValidationResult::Overlapping;
    }

    EditValidationResult::Valid
}

fn has_overlap(edits: &[TextEdit]) -> bool {
    for window in edits.windows(2) {
        if window[0].end > window[1].start {
            return true;
        }
    }
    false
}

pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut sorted_edits: Vec<&TextEdit> = edits.iter().collect();
    sorted_edits.sort_by_key(|edit| edit.start);
    sorted_edits.reverse();

    let mut result = String::with_capacity(source.len());
    let mut cursor = source.len();

    for edit in &sorted_edits {
        let start = edit.start;
        let end = edit.end;

        result.insert_str(0, &source[end..cursor]);
        result.insert_str(0, &edit.replacement);
        cursor = start;
    }

    result.insert_str(0, &source[..cursor]);
    result
}

pub fn collect_fixes(ctx: &FileContext<'_>, rules: &[Box<dyn FileRule>]) -> Vec<Fix> {
    let mut all_fixes = Vec::new();
    for rule in rules {
        if rule.severity().is_enabled() && rule.supports_fix() {
            all_fixes.extend(rule.fix(ctx));
        }
    }
    all_fixes
}

pub fn report_dry_run(fixes: &[Fix]) {
    const PREVIEW_MAX: usize = 40;
    for fix in fixes {
        println!("{}: {}", fix.file.display(), fix.rule);
        for edit in &fix.edits {
            let preview = preview_replacement(&edit.replacement, PREVIEW_MAX);
            println!(
                "  would replace bytes {}-{} with {}",
                edit.start, edit.end, preview
            );
        }
    }
}

fn preview_replacement(replacement: &str, max_len: usize) -> String {
    if replacement.len() <= max_len {
        format!("{:?}", replacement)
    } else {
        let truncated: String = replacement.chars().take(max_len).collect();
        format!("{:?}... ({} bytes)", truncated, replacement.len())
    }
}

pub fn span_edit(start: usize, end: usize, replacement: impl Into<String>) -> TextEdit {
    TextEdit {
        start,
        end,
        replacement: replacement.into(),
    }
}

pub fn remove_span(start: usize, end: usize) -> TextEdit {
    span_edit(start, end, String::new())
}

pub fn extend_end_through_optional_semicolon(source: &str, end: usize) -> usize {
    let after = &source[end..];
    let after_trimmed = after.trim_start();
    if after_trimmed.starts_with(';') {
        let semicolon_offset = after.len() - after_trimmed.len();
        end + semicolon_offset + 1
    } else {
        end
    }
}

pub fn extend_end_through_line_trivia(source: &str, end: usize) -> usize {
    let after = &source[end..];
    let mut new_end = end;
    for (index, byte) in after.bytes().enumerate() {
        match byte {
            b' ' | b'\t' => new_end = end + index + 1,
            b'\n' => {
                new_end = end + index + 1;
                break;
            }
            _ => break,
        }
    }
    new_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_overlapping_edits_apply() {
        let source = "aaabbbccc";
        let edits = vec![
            TextEdit {
                start: 3,
                end: 6,
                replacement: String::new(),
            },
            TextEdit {
                start: 0,
                end: 3,
                replacement: String::new(),
            },
        ];
        let result = apply_edits(source, &edits);
        assert_eq!(result, "ccc");
    }

    #[test]
    fn overlapping_edits_rejected() {
        let edits = vec![
            TextEdit {
                start: 0,
                end: 5,
                replacement: String::new(),
            },
            TextEdit {
                start: 3,
                end: 8,
                replacement: String::new(),
            },
        ];
        assert!(has_overlap(&edits));
    }

    #[test]
    fn reverse_order_preserves_offsets() {
        let source = "abcdef";
        let edits = vec![
            TextEdit {
                start: 0,
                end: 3,
                replacement: "ABC".to_string(),
            },
            TextEdit {
                start: 3,
                end: 6,
                replacement: "DEF".to_string(),
            },
        ];
        let result = apply_edits(source, &edits);
        assert_eq!(result, "ABCDEF");
    }

    #[test]
    fn replacement_at_same_span() {
        let source = "debugger;\n";
        let edits = vec![TextEdit {
            start: 0,
            end: 10,
            replacement: String::new(),
        }];
        let result = apply_edits(source, &edits);
        assert_eq!(result, "");
    }

    #[test]
    fn validate_rejects_start_greater_than_end() {
        let source = "abcdef";
        let edits = vec![TextEdit {
            start: 5,
            end: 3,
            replacement: String::new(),
        }];
        assert!(matches!(
            validate_edits(source, &edits),
            EditValidationResult::Invalid { .. }
        ));
    }

    #[test]
    fn validate_rejects_end_beyond_source() {
        let source = "abcdef";
        let edits = vec![TextEdit {
            start: 0,
            end: 10,
            replacement: String::new(),
        }];
        assert!(matches!(
            validate_edits(source, &edits),
            EditValidationResult::Invalid { .. }
        ));
    }

    #[test]
    fn validate_rejects_non_char_boundary() {
        let source = "\u{4f60}\u{597d}";
        let edits = vec![TextEdit {
            start: 1,
            end: 2,
            replacement: String::new(),
        }];
        assert!(matches!(
            validate_edits(source, &edits),
            EditValidationResult::Invalid { .. }
        ));
    }

    #[test]
    fn validate_detects_overlap() {
        let source = "abcdef";
        let edits = vec![
            TextEdit {
                start: 0,
                end: 3,
                replacement: String::new(),
            },
            TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            },
        ];
        assert_eq!(
            validate_edits(source, &edits),
            EditValidationResult::Overlapping
        );
    }

    #[test]
    fn span_edit_builds_text_edit() {
        let edit = span_edit(1, 3, "x");
        assert_eq!(edit.start, 1);
        assert_eq!(edit.end, 3);
        assert_eq!(edit.replacement, "x");
    }

    #[test]
    fn remove_span_builds_empty_replacement() {
        let edit = remove_span(1, 3);
        assert_eq!(edit.replacement, "");
    }

    #[test]
    fn extend_end_through_optional_semicolon_finds_semicolon() {
        let source = "debugger; more";
        let end = 8;
        assert_eq!(extend_end_through_optional_semicolon(source, end), 9);
    }

    #[test]
    fn extend_end_through_optional_semicolon_ignores_no_semicolon() {
        let source = "debugger more";
        let end = 8;
        assert_eq!(extend_end_through_optional_semicolon(source, end), 8);
    }

    #[test]
    fn extend_end_through_line_trivia_stops_after_newline() {
        let source = "abc   \nmore";
        let end = 3;
        assert_eq!(extend_end_through_line_trivia(source, end), 7);
    }

    #[test]
    fn extend_end_through_line_trivia_stops_at_non_whitespace() {
        let source = "abc   more";
        let end = 3;
        assert_eq!(extend_end_through_line_trivia(source, end), 6);
    }
}
