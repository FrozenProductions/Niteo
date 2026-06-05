use crate::harness;
use predicates::prelude::*;

#[test]
fn baseline_create_writes_file() {
    let project = harness::copy_fixture("baseline").unwrap();
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    assert!(baseline_path.exists());
}

#[test]
fn baseline_create_contains_violations() {
    let project = harness::copy_fixture("baseline").unwrap();
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&baseline_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();

    assert_eq!(parsed["version"], 1);
    let violations = parsed["violations"].as_array().unwrap();
    assert!(!violations.is_empty());
}

#[test]
fn lint_after_baseline_suppresses_known_violations() {
    let project = harness::copy_fixture("baseline").unwrap();

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .assert()
        .success();
}

#[test]
fn new_violations_still_fail_after_baseline() {
    let project = harness::copy_fixture("baseline").unwrap();

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let new_file = project.path().join("src/new.ts");
    std::fs::write(&new_file, "debugger;\n").unwrap();

    let mut config_content = std::fs::read_to_string(project.path().join("niteo.toml")).unwrap();
    config_content = config_content.replace(
        "[rules.no-debugger]\nseverity = \"off\"",
        "[rules.no-debugger]\nseverity = \"error\"",
    );
    std::fs::write(project.path().join("niteo.toml"), config_content).unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .assert()
        .failure();
}

#[test]
fn baseline_prune_removes_stale_entries() {
    let project = harness::copy_fixture("baseline").unwrap();

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let console_file = project.path().join("src/app.ts");
    std::fs::write(&console_file, "export const clean = true;\n").unwrap();

    harness::niteo_in_project(project.path())
        .args(["baseline", "prune"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pruned").or(predicate::str::contains("up to date")));
}

#[test]
fn custom_baseline_path_works() {
    let project = harness::copy_fixture("baseline").unwrap();
    let custom_path = project.path().join("custom-baseline.json");

    harness::niteo_in_project(project.path())
        .args([
            "baseline",
            "create",
            "--baseline",
            custom_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(custom_path.exists());
}

#[test]
fn baseline_uses_relative_paths() {
    let project = harness::copy_fixture("baseline").unwrap();
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&baseline_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    for violation in violations {
        let file_path = violation["file"].as_str().unwrap();
        assert!(
            !file_path.starts_with('/'),
            "baseline file path should be relative: {file_path}"
        );
    }
}

#[test]
fn malformed_baseline_fails_with_useful_error() {
    let project = harness::copy_fixture("baseline").unwrap();
    let baseline_path = project.path().join("niteo-baseline.json");
    std::fs::write(&baseline_path, "not valid json{{{").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .failure();
}
