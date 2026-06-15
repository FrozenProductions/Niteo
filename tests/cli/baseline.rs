use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;

#[test]
fn baseline_create_writes_file() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    assert!(baseline_path.exists());
    Ok(())
}

#[test]
fn baseline_create_contains_violations() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&baseline_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&contents)?;

    assert_eq!(parsed["version"], 1);
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;
    assert!(!violations.is_empty());
    Ok(())
}

#[test]
fn lint_after_baseline_suppresses_known_violations() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn new_violations_still_fail_after_baseline() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let new_file = project.path().join("src/new.ts");
    std::fs::write(&new_file, "debugger;\n")?;

    let mut config_content = std::fs::read_to_string(project.path().join("niteo.toml"))?;
    config_content = config_content.replace(
        "[rules.no-debugger]\nseverity = \"off\"",
        "[rules.no-debugger]\nseverity = \"error\"",
    );
    std::fs::write(project.path().join("niteo.toml"), config_content)?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn baseline_prune_removes_stale_entries() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let console_file = project.path().join("src/app.ts");
    std::fs::write(&console_file, "export const clean = true;\n")?;

    harness::niteo_in_project(project.path())
        .args(["baseline", "prune"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pruned").or(predicate::str::contains("up to date")));
    Ok(())
}

#[test]
fn custom_baseline_path_works() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;
    let custom_path = project.path().join("custom-baseline.json");

    harness::niteo_in_project(project.path())
        .args([
            "baseline",
            "create",
            "--baseline",
            custom_path
                .to_str()
                .context("expected custom baseline path to be valid UTF-8")?,
        ])
        .assert()
        .success();

    assert!(custom_path.exists());
    Ok(())
}

#[test]
fn baseline_uses_relative_paths() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;
    let baseline_path = project.path().join("niteo-baseline.json");

    harness::niteo_in_project(project.path())
        .args(["baseline", "create"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&baseline_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&contents)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    for violation in violations {
        let file_path = violation["file"]
            .as_str()
            .context("expected file path string")?;
        assert!(
            !file_path.starts_with('/'),
            "baseline file path should be relative: {file_path}"
        );
    }
    Ok(())
}

#[test]
fn malformed_baseline_fails_with_useful_error() -> Result<()> {
    let project = harness::copy_fixture("baseline")?;
    let baseline_path = project.path().join("niteo-baseline.json");
    std::fs::write(&baseline_path, "not valid json{{{")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .failure();
    Ok(())
}
