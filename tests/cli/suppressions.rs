use crate::harness;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn file_level_suppression_hides_violations() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let file_ignore_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| v["file"].as_str().unwrap().contains("file_ignore.ts"))
        .collect();

    assert!(
        file_ignore_violations.is_empty(),
        "file_ignore.ts should have no violations due to file-level suppression"
    );
}

#[test]
fn next_line_suppression_hides_targeted_violation() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let next_line_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("next_line.ts") && v["line"].as_u64() == Some(2)
        })
        .collect();

    assert!(
        next_line_violations.is_empty(),
        "line 2 of next_line.ts should be suppressed"
    );

    let unsuppressed: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("next_line.ts") && v["line"].as_u64() == Some(3)
        })
        .collect();

    assert!(
        !unsuppressed.is_empty(),
        "line 3 of next_line.ts should NOT be suppressed"
    );
}

#[test]
fn same_line_suppression_hides_targeted_violation() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let same_line_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("same_line.ts") && v["line"].as_u64() == Some(1)
        })
        .collect();

    assert!(
        same_line_violations.is_empty(),
        "line 1 of same_line.ts should be suppressed"
    );
}

#[test]
fn rule_scoped_suppression_only_affects_target_rule() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let next_line_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("next_line.ts")
                && v["line"].as_u64() == Some(2)
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    assert!(
        next_line_console.is_empty(),
        "no-console on line 2 should be suppressed by rule-scoped directive"
    );
}

#[test]
fn stale_suppression_reporting_via_flag() {
    let project = harness::copy_fixture("suppressions").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--report-suppressions"])
        .assert()
        .stdout(predicate::str::contains("stale"));
}

#[test]
fn suppressions_appear_in_json_when_requested() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json", "--report-suppressions"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    assert!(
        parsed.get("suppressions").is_some(),
        "JSON output should contain suppressions key when --report-suppressions is used"
    );

    let suppressions = &parsed["suppressions"];
    assert!(suppressions["totalSuppressed"].is_number());
    assert!(suppressions["totalStale"].is_number());
}

#[test]
fn stale_directives_reported_when_reporting_enabled() {
    let project = harness::copy_fixture("suppressions").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json", "--report-suppressions"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let suppressions = &parsed["suppressions"];
    let stale_count = suppressions["totalStale"].as_u64().unwrap();
    assert!(
        stale_count > 0,
        "stale.ts should produce at least one stale directive"
    );
}
