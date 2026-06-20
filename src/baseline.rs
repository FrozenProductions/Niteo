use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::rules::Violation;

const BASELINE_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    version: u8,
    violations: Vec<BaselineViolation>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
struct BaselineViolation {
    file: String,
    line: Option<usize>,
    column: Option<usize>,
    rule: String,
    message: String,
    subject: Option<String>,
}

pub struct PruneResult {
    pub baseline: Baseline,
    pub removed_count: usize,
}

impl Baseline {
    pub fn from_violations(root: &Path, violations: &[Violation]) -> Self {
        let mut baseline = Self {
            version: BASELINE_VERSION,
            violations: violations
                .iter()
                .map(|violation| BaselineViolation::from_violation(root, violation))
                .collect(),
        };

        baseline.sort_and_dedup();
        baseline
    }

    fn sort_and_dedup(&mut self) {
        self.violations.sort();
        self.violations.dedup();
    }

    // Copy edits and severity tuning should not invalidate an existing baseline.
    pub fn filter_new_violations(&self, root: &Path, violations: Vec<Violation>) -> Vec<Violation> {
        violations
            .into_iter()
            .filter(|violation| {
                let candidate = BaselineViolation::from_violation(root, violation);
                self.violations.binary_search(&candidate).is_err()
            })
            .collect()
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn prune(&self, root: &Path, current_violations: &[Violation]) -> PruneResult {
        let current_set: HashSet<BaselineViolation> = current_violations
            .iter()
            .map(|violation| BaselineViolation::from_violation(root, violation))
            .collect();

        let remaining: Vec<BaselineViolation> = self
            .violations
            .iter()
            .filter(|violation| current_set.contains(violation))
            .cloned()
            .collect();

        let removed_count = self.violations.len() - remaining.len();

        PruneResult {
            baseline: Baseline {
                version: self.version,
                violations: remaining,
            },
            removed_count,
        }
    }
}

pub fn read_baseline(path: &Path) -> Result<Option<Baseline>> {
    if !path.exists() {
        return Ok(None);
    }

    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut baseline: Baseline = serde_json::from_str(&source)
        .with_context(|| format!("failed to parse baseline from {}", path.display()))?;
    baseline.sort_and_dedup();

    Ok(Some(baseline))
}

pub fn write_baseline(path: &Path, baseline: &Baseline) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let output = serde_json::to_string_pretty(baseline)?;
    fs::write(path, format!("{output}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

impl BaselineViolation {
    fn from_violation(root: &Path, violation: &Violation) -> Self {
        Self {
            file: normalize_path(root, &violation.file),
            line: violation.line,
            column: violation.column,
            rule: violation.rule.to_string(),
            message: violation.message.to_string(),
            subject: violation.subject.clone(),
        }
    }
}

impl Ord for BaselineViolation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file
            .cmp(&other.file)
            .then(self.line.cmp(&other.line))
            .then(self.column.cmp(&other.column))
            .then(self.rule.cmp(&other.rule))
            .then(self.message.cmp(&other.message))
            .then(self.subject.cmp(&other.subject))
    }
}

impl PartialOrd for BaselineViolation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn normalize_path(root: &Path, path: &Path) -> String {
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    pathbuf_to_unix_string(relative_path)
}

fn pathbuf_to_unix_string(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use std::path::{Path, PathBuf};

    use crate::config::Severity;

    use super::*;

    #[test]
    fn filters_existing_violations() -> Result<()> {
        let root = Path::new("/repo/src");
        let existing = violation("/repo/src/app.ts", Some(1), "no-console");
        let new = violation("/repo/src/new.ts", Some(1), "no-console");
        let baseline = Baseline::from_violations(root, std::slice::from_ref(&existing));

        let violations = baseline.filter_new_violations(root, vec![existing, new.clone()]);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].file, new.file);

        Ok(())
    }

    #[test]
    fn treats_changed_location_as_new() -> Result<()> {
        let root = Path::new("/repo/src");
        let baseline = Baseline::from_violations(
            root,
            &[violation("/repo/src/app.ts", Some(1), "no-console")],
        );

        let violations = baseline.filter_new_violations(
            root,
            vec![violation("/repo/src/app.ts", Some(2), "no-console")],
        );

        assert_eq!(violations.len(), 1);

        Ok(())
    }

    #[test]
    fn ignores_changed_details_for_same_violation_identity() -> Result<()> {
        let root = Path::new("/repo/src");
        let mut existing = violation("/repo/src", None, "min-items-per-directory");
        existing.detail = Some("Contains 1 TypeScript files (minimum: 3).".to_string());
        let baseline = Baseline::from_violations(root, &[existing]);
        let mut changed = violation("/repo/src", None, "min-items-per-directory");
        changed.detail = Some("Contains 2 TypeScript files (minimum: 3).".to_string());

        let violations = baseline.filter_new_violations(root, vec![changed]);

        assert!(violations.is_empty());

        Ok(())
    }

    fn violation(file: &str, line: Option<usize>, rule: &'static str) -> Violation {
        Violation {
            file: PathBuf::from(file),
            span: None,
            line,
            column: Some(1),
            rule,
            message: "message",
            severity: Severity::Warn,
            detail: None,
            subject: None,
        }
    }

    #[test]
    fn prune_removes_stale_entries() -> Result<()> {
        let root = Path::new("/repo");
        let baseline = Baseline::from_violations(
            root,
            &[
                violation("/repo/src/app.ts", Some(1), "no-console"),
                violation("/repo/src/old.ts", Some(5), "no-debugger"),
                violation("/repo/src/keep.ts", Some(10), "no-eval"),
            ],
        );

        let current = vec![
            violation("/repo/src/app.ts", Some(1), "no-console"),
            violation("/repo/src/keep.ts", Some(10), "no-eval"),
        ];

        let result = baseline.prune(root, &current);

        assert_eq!(result.removed_count, 1);
        assert_eq!(result.baseline.violation_count(), 2);

        Ok(())
    }

    #[test]
    fn prune_keeps_all_when_current() -> Result<()> {
        let root = Path::new("/repo");
        let violations = vec![violation("/repo/src/app.ts", Some(1), "no-console")];
        let baseline = Baseline::from_violations(root, &violations);

        let result = baseline.prune(root, &violations);

        assert_eq!(result.removed_count, 0);
        assert_eq!(result.baseline.violation_count(), 1);

        Ok(())
    }

    #[test]
    fn prune_removes_all_when_no_current_violations() -> Result<()> {
        let root = Path::new("/repo");
        let baseline = Baseline::from_violations(
            root,
            &[violation("/repo/src/app.ts", Some(1), "no-console")],
        );

        let result = baseline.prune(root, &[]);

        assert_eq!(result.removed_count, 1);
        assert_eq!(result.baseline.violation_count(), 0);

        Ok(())
    }
}
