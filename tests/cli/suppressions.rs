use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn file_level_suppression_hides_violations() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let file_ignore_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map_or(false, |s| s.contains("file_ignore.ts"))
        })
        .collect();

    assert!(
        file_ignore_violations.is_empty(),
        "file_ignore.ts should have no violations due to file-level suppression"
    );
    Ok(())
}

#[test]
fn next_line_suppression_hides_targeted_violation() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let next_line_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map_or(false, |s| s.contains("next_line.ts"))
                && v["line"].as_u64() == Some(2)
        })
        .collect();

    assert!(
        next_line_violations.is_empty(),
        "line 2 of next_line.ts should be suppressed"
    );

    let unsuppressed: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map_or(false, |s| s.contains("next_line.ts"))
                && v["line"].as_u64() == Some(3)
        })
        .collect();

    assert!(
        !unsuppressed.is_empty(),
        "line 3 of next_line.ts should NOT be suppressed"
    );
    Ok(())
}

#[test]
fn same_line_suppression_hides_targeted_violation() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let same_line_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map_or(false, |s| s.contains("same_line.ts"))
                && v["line"].as_u64() == Some(1)
        })
        .collect();

    assert!(
        same_line_violations.is_empty(),
        "line 1 of same_line.ts should be suppressed"
    );
    Ok(())
}

#[test]
fn rule_scoped_suppression_only_affects_target_rule() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let next_line_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map_or(false, |s| s.contains("next_line.ts"))
                && v["line"].as_u64() == Some(2)
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    assert!(
        next_line_console.is_empty(),
        "no-console on line 2 should be suppressed by rule-scoped directive"
    );
    Ok(())
}

#[test]
fn stale_suppression_reporting_via_flag() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--report-suppressions"])
        .assert()
        .stdout(predicate::str::contains("stale"));
    Ok(())
}

#[test]
fn suppressions_appear_in_json_when_requested() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json", "--report-suppressions"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    assert!(
        parsed.get("suppressions").is_some(),
        "JSON output should contain suppressions key when --report-suppressions is used"
    );

    let suppressions = &parsed["suppressions"];
    assert!(suppressions["totalSuppressed"].is_number());
    assert!(suppressions["totalStale"].is_number());
    Ok(())
}

#[test]
fn stale_directives_reported_when_reporting_enabled() -> Result<()> {
    let project = harness::copy_fixture("suppressions")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json", "--report-suppressions"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let suppressions = &parsed["suppressions"];
    let stale_count = suppressions["totalStale"]
        .as_u64()
        .context("expected total stale count")?;
    assert!(
        stale_count > 0,
        "stale.ts should produce at least one stale directive"
    );
    Ok(())
}
