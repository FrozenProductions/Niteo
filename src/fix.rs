use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::rules::{FileContext, FileRule, Fix, TextEdit};

pub struct FixOutcome {
    pub fixed_files: Vec<PathBuf>,
    pub rejected_overlapping: usize,
    pub rejected_stale: usize,
}

pub fn apply_fixes(fixes: Vec<Fix>, dry_run: bool) -> Result<FixOutcome> {
    let mut by_file: HashMap<PathBuf, Vec<TextEdit>> = HashMap::new();
    for fix in fixes {
        by_file.entry(fix.file).or_default().extend(fix.edits);
    }

    let mut fixed_files = Vec::new();
    let mut rejected_overlapping = 0;
    let mut rejected_stale = 0;

    for (file_path, mut edits) in by_file {
        edits.sort_by_key(|edit| edit.start);

        if has_overlap(&edits) {
            rejected_overlapping += edits.len();
            continue;
        }

        let source = std::fs::read_to_string(&file_path)
            .with_context(|| format!("failed to read {}", file_path.display()))?;

        let modified = apply_edits(&source, &edits);

        if !dry_run {
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
    })
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
    let bytes = source.as_bytes();
    let mut sorted_edits: Vec<&TextEdit> = edits.iter().collect();
    sorted_edits.sort_by_key(|edit| edit.start);
    sorted_edits.reverse();

    let mut result = String::with_capacity(source.len());
    let mut cursor = bytes.len();

    for edit in &sorted_edits {
        let start = edit.start.min(bytes.len());
        let end = edit.end.min(bytes.len());

        result.insert_str(0, std::str::from_utf8(&bytes[end..cursor]).unwrap_or(""));
        result.insert_str(0, &edit.replacement);
        cursor = start;
    }

    result.insert_str(0, std::str::from_utf8(&bytes[..cursor]).unwrap_or(""));
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
    for fix in fixes {
        for edit in &fix.edits {
            println!(
                "{}: would replace bytes {}-{} with {:?}",
                fix.file.display(),
                edit.start,
                edit.end,
                edit.replacement,
            );
        }
    }
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
}
