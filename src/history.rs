use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::report::model::Report;
use crate::report::summary::score;

/// Directory under the project root where history and other Niteo metadata lives.
const NITEO_DIR: &str = ".niteo";
/// JSON Lines file with one `HistoryEntry` per line.
const HISTORY_FILE: &str = "history.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    /// ISO 8601 timestamp of the run.
    pub timestamp: String,
    /// Total files scanned.
    pub files: usize,
    /// Total violations across all severities.
    pub violations: usize,
    /// Number of error‑level violations.
    pub errors: usize,
    /// Number of warning‑level violations.
    pub warnings: usize,
    /// Number of info‑level violations.
    pub infos: usize,
    /// Normalised health score (0–100). Higher is better.
    pub health_score: u8,
}

/// Resolve the full path to `.niteo/history.jsonl` inside `workspace`.
pub fn history_path(workspace: &Path) -> PathBuf {
    workspace.join(NITEO_DIR).join(HISTORY_FILE)
}

/// Ensure the `.niteo` directory exists and return the path to `history.jsonl`.
fn ensure_history_file(workspace: &Path) -> io::Result<PathBuf> {
    let niteo_dir = workspace.join(NITEO_DIR);
    if !niteo_dir.exists() {
        fs::create_dir_all(&niteo_dir)?;
    }
    Ok(niteo_dir.join(HISTORY_FILE))
}

fn compute_health_score(report: &Report) -> u8 {
    score(
        report.error_count(),
        report.warning_count(),
        report.files.len(),
    ) as u8
}

fn format_iso8601_now() -> String {
    let now = SystemTime::now();
    let since_epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = since_epoch.as_secs();
    let nanos = since_epoch.subsec_nanos();

    let days_since_epoch = secs / 86_400;
    let seconds_in_day = (secs % 86_400) as u32;

    let mut year = 1970;
    let mut remaining_days = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        year += 1;
        remaining_days -= days_in_year;
    }

    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    for (idx, &days) in month_days.iter().enumerate() {
        let dim = if idx == 1 && is_leap_year(year) {
            29
        } else {
            days
        };
        if remaining_days < dim {
            month = idx as u32 + 1;
            break;
        }
        remaining_days -= dim;
        month = idx as u32 + 2;
    }
    let day = remaining_days + 1;

    let hour = seconds_in_day / 3600;
    let minute = (seconds_in_day % 3600) / 60;
    let second = seconds_in_day % 60;
    let millis = nanos / 1_000_000;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, minute, second, millis
    )
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn append_to_workspace(workspace: &Path, report: &Report) -> io::Result<()> {
    let path = ensure_history_file(workspace)?;
    let entry = HistoryEntry {
        timestamp: format_iso8601_now(),
        files: report.files.len(),
        violations: report.violations.len(),
        errors: report.error_count(),
        warnings: report.warning_count(),
        infos: report.info_count(),
        health_score: compute_health_score(report),
    };

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let line =
        serde_json::to_string(&entry).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Read all history entries from the project's `.niteo/history.jsonl`.
/// Lines that cannot be parsed are silently skipped.
///
/// # Errors
///
/// Returns an I/O error if the file cannot be read.
pub fn read_entries(workspace: &Path) -> io::Result<Vec<HistoryEntry>> {
    let path = history_path(workspace);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::config::Severity;
    use crate::report::model::Report;
    use crate::rules::Violation;
    use anyhow::Context;

    use super::{compute_health_score, history_path, read_entries};

    fn make_violation(severity: Severity) -> Violation {
        Violation {
            file: PathBuf::from("test.ts"),
            span: None,
            line: Some(1),
            column: Some(1),
            rule: "test-rule",
            message: "test message",
            severity,
            detail: None,
            subject: None,
        }
    }

    #[test]
    fn test_compute_health_score_perfect() {
        let report = Report::new(vec![PathBuf::from("a.ts")], vec![]);
        assert_eq!(compute_health_score(&report), 100);
    }

    #[test]
    fn test_compute_health_score_with_warnings() {
        let report = Report::new(
            vec![PathBuf::from("a.ts"), PathBuf::from("b.ts")],
            vec![make_violation(Severity::Warn)],
        );
        assert_eq!(compute_health_score(&report), 50);
    }

    #[test]
    fn test_compute_health_score_with_errors() {
        let report = Report::new(
            vec![PathBuf::from("a.ts")],
            vec![make_violation(Severity::Error)],
        );
        assert_eq!(compute_health_score(&report), 0);
    }

    #[test]
    fn test_history_path() {
        let ws = PathBuf::from("/tmp/foo");
        assert_eq!(
            history_path(&ws),
            PathBuf::from("/tmp/foo/.niteo/history.jsonl")
        );
    }

    #[test]
    fn test_read_entries_skips_malformed_lines() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join(".niteo").join("history.jsonl");
        fs::create_dir_all(path.parent().context("expected parent")?)?;
        fs::write(
            &path,
            r#"{"timestamp":"2024-01-01T00:00:00Z","files":1,"violations":0,"errors":0,"warnings":0,"infos":0,"health_score":100}
malformed_line
{"timestamp":"2024-01-02T00:00:00Z","files":2,"violations":1,"errors":1,"warnings":0,"infos":0,"health_score":0}"#,
        )?;

        let entries = read_entries(tmp.path())?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].health_score, 100);
        assert_eq!(entries[1].health_score, 0);
        Ok(())
    }
}
