use crate::harness;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn root_config_applies_to_root_scan() {
    let project = harness::copy_fixture("monorepo").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let has_console_violations = violations
        .iter()
        .any(|v| v["rule"].as_str() == Some("no-console"));

    assert!(
        has_console_violations,
        "root config should detect no-console violations"
    );
}

#[test]
fn nested_config_changes_severity() {
    let project = harness::copy_fixture("monorepo").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let app_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| v["file"].as_str().unwrap().contains("packages/app"))
        .collect();

    for violation in &app_violations {
        if violation["rule"].as_str() == Some("no-console") {
            assert_eq!(
                violation["severity"].as_str().unwrap(),
                "error",
                "app package should have no-console as error"
            );
        }
    }
}

#[test]
fn scoped_run_limits_to_package() {
    let project = harness::copy_fixture("monorepo").unwrap();
    let scope_path = project.path().join("packages/app");

    let output = harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "json",
            "--scope",
            scope_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let lib_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| v["file"].as_str().unwrap().contains("packages/lib"))
        .collect();

    assert!(
        lib_violations.is_empty(),
        "scoped run should not include violations from packages/lib"
    );
}

#[test]
fn deny_child_configs_fails_with_nested_configs() {
    let project = harness::copy_fixture("monorepo").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--deny-child-configs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--deny-child-configs"));
}

#[test]
fn deny_child_configs_succeeds_with_scope_excluding_children() {
    let project = harness::copy_fixture("monorepo").unwrap();
    let scope_path = project.path().join("packages/lib");

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--deny-child-configs",
            "--fail-on",
            "error",
            "--scope",
            scope_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn config_merging_produces_expected_severities() {
    let project = harness::copy_fixture("monorepo").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let lib_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("packages/lib")
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    for violation in &lib_console {
        assert_eq!(
            violation["severity"].as_str().unwrap(),
            "warning",
            "lib package should inherit root no-console severity (warning)"
        );
    }

    let app_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"].as_str().unwrap().contains("packages/app")
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    for violation in &app_console {
        assert_eq!(
            violation["severity"].as_str().unwrap(),
            "error",
            "app package should override no-console to error"
        );
    }
}
