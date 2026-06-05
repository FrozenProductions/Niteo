use crate::harness;
use predicates::prelude::*;

#[test]
fn clean_project_exits_zero() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();
}

#[test]
fn project_with_violations_exits_nonzero() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .failure();
}

#[test]
fn explicit_lint_subcommand_works() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();
}

#[test]
fn format_text_produces_output() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "text"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Niteo Structure Health"));
}

#[test]
fn format_json_produces_valid_json() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(assert.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("files").is_some());
    assert!(parsed.get("violations").is_some());
}

#[test]
fn format_sarif_produces_valid_sarif() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(assert.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(
        parsed["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "Niteo");
}

#[test]
fn output_flag_writes_to_file() {
    let project = harness::copy_fixture("reports/basic").unwrap();
    let output_path = project.path().join("report.json");

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "json",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .failure();

    assert!(output_path.exists());
    let contents = std::fs::read_to_string(&output_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert!(parsed.get("summary").is_some());
}

#[test]
fn fail_on_error_exits_zero_for_warnings_only() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "error"])
        .assert()
        .success();
}

#[test]
fn fail_on_any_exits_nonzero_for_info_violations() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "any"])
        .assert()
        .failure();
}

#[test]
fn verbose_flag_shows_timing() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--verbose"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Done in"));
}

#[test]
fn expected_rule_ids_appear_in_json() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(assert.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    let rule_ids: Vec<&str> = violations
        .iter()
        .map(|v| v["rule"].as_str().unwrap())
        .collect();

    assert!(rule_ids.contains(&"no-console"));
    assert!(rule_ids.contains(&"no-debugger"));
    assert!(rule_ids.contains(&"no-any"));
}

#[test]
fn expected_file_paths_appear_in_json() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(assert.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    let file_paths: Vec<String> = violations
        .iter()
        .map(|v| v["file"].as_str().unwrap().to_string())
        .collect();

    let has_console = file_paths.iter().any(|p| p.contains("console.ts"));
    let has_any = file_paths.iter().any(|p| p.contains("any.ts"));
    assert!(has_console);
    assert!(has_any);
}

#[test]
fn violation_counts_match() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(assert.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let summary = &parsed["summary"];
    let violation_count = summary["violations"].as_u64().unwrap();
    let violations_array = parsed["violations"].as_array().unwrap();

    assert_eq!(violation_count as usize, violations_array.len());
    assert!(violation_count > 0);
}
