use anyhow::{Context, Result};
use std::collections::{BTreeSet, HashMap, HashSet};
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
    let mut by_file: HashMap<PathBuf, Vec<Fix>> = HashMap::new();
    for fix in fixes {
        by_file.entry(fix.file.clone()).or_default().push(fix);
    }

    let mut fixed_files = Vec::new();
    let mut rejected_overlapping = 0;
    let mut rejected_stale = 0;
    let mut rejected_invalid = 0;
    let mut rejected_parse = 0;

    let mut file_entries: Vec<(PathBuf, Vec<Fix>)> = by_file.into_iter().collect();
    file_entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (file_path, fixes) in file_entries {
        let source = std::fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        let rejected_indices = overlapping_fix_indices(&fixes);

        if !rejected_indices.is_empty() {
            let conflicting_rules: BTreeSet<_> = rejected_indices
                .iter()
                .map(|idx| fixes[*idx].rule)
                .collect();
            eprintln!(
                "warning: rejected overlapping edits in {} from {}",
                file_path.display(),
                conflicting_rules.into_iter().collect::<Vec<_>>().join(", ")
            );
        }

        let mut kept_edits = Vec::new();
        for (idx, fix) in fixes.iter().enumerate() {
            if rejected_indices.contains(&idx) {
                rejected_overlapping += fix.edits.len();
                continue;
            }
            kept_edits.extend(fix.edits.iter().cloned());
        }

        if kept_edits.is_empty() {
            continue;
        }

        kept_edits.sort_by_key(|edit| edit.start);

        match validate_edits(&source, &kept_edits) {
            EditValidationResult::Valid => {}
            EditValidationResult::Overlapping => {
                eprintln!(
                    "warning: rejected edits in {}: overlapping edits detected after conflict filtering",
                    file_path.display()
                );
                rejected_overlapping += kept_edits.len();
                continue;
            }
            EditValidationResult::Invalid { reason } => {
                eprintln!(
                    "warning: rejected edits in {}: {}",
                    file_path.display(),
                    reason
                );
                rejected_invalid += kept_edits.len();
                continue;
            }
        }

        let modified = apply_edits(&source, &kept_edits);

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
                rejected_parse += kept_edits.len();
                continue;
            }
        }

        if !options.dry_run {
            let current_source = std::fs::read_to_string(&file_path)
                .with_context(|| format!("failed to read {}", file_path.display()))?;

            if current_source != source {
                rejected_stale += kept_edits.len();
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

fn overlapping_fix_indices(fixes: &[Fix]) -> HashSet<usize> {
    struct AnnotatedEdit<'a> {
        fix_index: usize,
        edit_index: usize,
        edit: &'a TextEdit,
    }

    let mut annotated = Vec::new();
    for (fix_index, fix) in fixes.iter().enumerate() {
        for (edit_index, edit) in fix.edits.iter().enumerate() {
            annotated.push(AnnotatedEdit {
                fix_index,
                edit_index,
                edit,
            });
        }
    }

    annotated.sort_by(|a, b| {
        a.edit
            .start
            .cmp(&b.edit.start)
            .then_with(|| a.edit.end.cmp(&b.edit.end))
            .then_with(|| a.fix_index.cmp(&b.fix_index))
            .then_with(|| a.edit_index.cmp(&b.edit_index))
    });

    let mut rejected = HashSet::new();
    for window in annotated.windows(2) {
        let [first, second] = window else { continue };
        if first.edit.end > second.edit.start {
            rejected.insert(first.fix_index);
            rejected.insert(second.fix_index);
        }
    }
    rejected
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
        let [first, second] = window else { continue };
        if first.end > second.start {
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

        result.insert_str(0, source.get(end..cursor).unwrap_or(""));
        result.insert_str(0, &edit.replacement);
        cursor = start;
    }

    result.insert_str(0, source.get(..cursor).unwrap_or(""));
    result
}

pub fn collect_fixes(ctx: &FileContext<'_>, rules: &[Box<dyn FileRule + Send + Sync>]) -> Vec<Fix> {
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
    let after = source.get(end..).unwrap_or("");
    let after_trimmed = after.trim_start();
    if after_trimmed.starts_with(';') {
        let semicolon_offset = after.len() - after_trimmed.len();
        end + semicolon_offset + 1
    } else {
        end
    }
}

pub fn extend_end_through_line_trivia(source: &str, end: usize) -> usize {
    let after = source.get(end..).unwrap_or("");
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
    fn non_overlapping_edits_apply() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn overlapping_edits_rejected() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn reverse_order_preserves_offsets() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn replacement_at_same_span() -> Result<()> {
        let source = "debugger;\n";
        let edits = vec![TextEdit {
            start: 0,
            end: 10,
            replacement: String::new(),
        }];
        let result = apply_edits(source, &edits);
        assert_eq!(result, "");
        Ok(())
    }

    #[test]
    fn validate_rejects_start_greater_than_end() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn validate_rejects_end_beyond_source() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn validate_rejects_non_char_boundary() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn validate_detects_overlap() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn span_edit_builds_text_edit() -> Result<()> {
        let edit = span_edit(1, 3, "x");
        assert_eq!(edit.start, 1);
        assert_eq!(edit.end, 3);
        assert_eq!(edit.replacement, "x");
        Ok(())
    }

    #[test]
    fn remove_span_builds_empty_replacement() -> Result<()> {
        let edit = remove_span(1, 3);
        assert_eq!(edit.replacement, "");
        Ok(())
    }

    #[test]
    fn extend_end_through_optional_semicolon_finds_semicolon() -> Result<()> {
        let source = "debugger; more";
        let end = 8;
        assert_eq!(extend_end_through_optional_semicolon(source, end), 9);
        Ok(())
    }

    #[test]
    fn extend_end_through_optional_semicolon_ignores_no_semicolon() -> Result<()> {
        let source = "debugger more";
        let end = 8;
        assert_eq!(extend_end_through_optional_semicolon(source, end), 8);
        Ok(())
    }

    #[test]
    fn extend_end_through_line_trivia_stops_after_newline() -> Result<()> {
        let source = "abc   \nmore";
        let end = 3;
        assert_eq!(extend_end_through_line_trivia(source, end), 7);
        Ok(())
    }

    #[test]
    fn extend_end_through_line_trivia_stops_at_non_whitespace() -> Result<()> {
        let source = "abc   more";
        let end = 3;
        assert_eq!(extend_end_through_line_trivia(source, end), 6);
        Ok(())
    }

    fn test_file_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("niteo_fix_test_{}_{}", std::process::id(), name));
        path
    }

    fn build_fix(rule: &'static str, file: PathBuf, edits: Vec<TextEdit>) -> Fix {
        Fix { file, rule, edits }
    }

    #[test]
    fn non_overlapping_fixes_from_different_rules_apply() -> Result<()> {
        let path = test_file_path("non_overlap");
        std::fs::write(&path, "aaabbbccc")?;

        let fixes = vec![
            build_fix(
                "rule-a",
                path.clone(),
                vec![TextEdit {
                    start: 0,
                    end: 3,
                    replacement: "A".to_string(),
                }],
            ),
            build_fix(
                "rule-b",
                path.clone(),
                vec![TextEdit {
                    start: 3,
                    end: 6,
                    replacement: "B".to_string(),
                }],
            ),
        ];

        let outcome = apply_fixes(
            fixes,
            ApplyFixOptions {
                dry_run: false,
                validate_parse: false,
            },
        )?;

        let result = std::fs::read_to_string(&path)?;
        std::fs::remove_file(&path)?;

        assert_eq!(result, "ABccc");
        assert_eq!(outcome.rejected_overlapping, 0);
        assert!(outcome.fixed_files.contains(&path));
        Ok(())
    }

    #[test]
    fn overlapping_fixes_reject_only_conflicting_fixes() -> Result<()> {
        let path = test_file_path("partial_overlap");
        std::fs::write(&path, "aaabbbccc")?;

        let fixes = vec![
            build_fix(
                "rule-a",
                path.clone(),
                vec![TextEdit {
                    start: 0,
                    end: 5,
                    replacement: "A".to_string(),
                }],
            ),
            build_fix(
                "rule-b",
                path.clone(),
                vec![TextEdit {
                    start: 3,
                    end: 6,
                    replacement: "B".to_string(),
                }],
            ),
            build_fix(
                "rule-c",
                path.clone(),
                vec![TextEdit {
                    start: 6,
                    end: 9,
                    replacement: "C".to_string(),
                }],
            ),
        ];

        let outcome = apply_fixes(
            fixes,
            ApplyFixOptions {
                dry_run: false,
                validate_parse: false,
            },
        )?;

        let result = std::fs::read_to_string(&path)?;
        std::fs::remove_file(&path)?;

        assert_eq!(result, "aaabbbC");
        assert_eq!(outcome.rejected_overlapping, 2);
        assert!(outcome.fixed_files.contains(&path));
        Ok(())
    }

    #[test]
    fn internally_overlapping_fix_is_rejected() -> Result<()> {
        let path = test_file_path("internal_overlap");
        std::fs::write(&path, "aaabbbcccddd")?;

        let fixes = vec![
            build_fix(
                "rule-a",
                path.clone(),
                vec![
                    TextEdit {
                        start: 0,
                        end: 5,
                        replacement: "A".to_string(),
                    },
                    TextEdit {
                        start: 3,
                        end: 8,
                        replacement: "B".to_string(),
                    },
                ],
            ),
            build_fix(
                "rule-b",
                path.clone(),
                vec![TextEdit {
                    start: 9,
                    end: 12,
                    replacement: "C".to_string(),
                }],
            ),
        ];

        let outcome = apply_fixes(
            fixes,
            ApplyFixOptions {
                dry_run: false,
                validate_parse: false,
            },
        )?;

        let result = std::fs::read_to_string(&path)?;
        std::fs::remove_file(&path)?;

        assert_eq!(result, "aaabbbcccC");
        assert_eq!(outcome.rejected_overlapping, 2);
        assert!(outcome.fixed_files.contains(&path));
        Ok(())
    }

    #[test]
    fn all_overlapping_fixes_reject_file_write() -> Result<()> {
        let path = test_file_path("all_overlap");
        std::fs::write(&path, "aaabbbccc")?;

        let fixes = vec![
            build_fix(
                "rule-a",
                path.clone(),
                vec![TextEdit {
                    start: 0,
                    end: 5,
                    replacement: "A".to_string(),
                }],
            ),
            build_fix(
                "rule-b",
                path.clone(),
                vec![TextEdit {
                    start: 3,
                    end: 8,
                    replacement: "B".to_string(),
                }],
            ),
        ];

        let outcome = apply_fixes(
            fixes,
            ApplyFixOptions {
                dry_run: false,
                validate_parse: false,
            },
        )?;

        let result = std::fs::read_to_string(&path)?;
        std::fs::remove_file(&path)?;

        assert_eq!(result, "aaabbbccc");
        assert_eq!(outcome.rejected_overlapping, 2);
        assert!(!outcome.fixed_files.contains(&path));
        Ok(())
    }
}
