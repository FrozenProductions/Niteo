use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use niteo::fix::{ApplyFixOptions, apply_fixes};
use niteo::rules::{Fix, TextEdit};

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
            sources: HashMap::new(),
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
            sources: HashMap::new(),
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
            sources: HashMap::new(),
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
            sources: HashMap::new(),
        },
    )?;

    let result = std::fs::read_to_string(&path)?;
    std::fs::remove_file(&path)?;

    assert_eq!(result, "aaabbbccc");
    assert_eq!(outcome.rejected_overlapping, 2);
    assert!(!outcome.fixed_files.contains(&path));
    Ok(())
}
