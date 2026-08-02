use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn root_config_applies_to_root_scan() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let has_console_violations = violations
        .iter()
        .any(|v| v["rule"].as_str() == Some("no-console"));

    assert!(
        has_console_violations,
        "root config should detect no-console violations"
    );
    Ok(())
}

#[test]
fn nested_config_changes_severity() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let app_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .is_ok_and(|s| s.contains("packages/app"))
        })
        .collect();

    for violation in &app_violations {
        if violation["rule"].as_str() == Some("no-console") {
            assert_eq!(
                violation["severity"]
                    .as_str()
                    .context("expected severity string")?,
                "error",
                "app package should have no-console as error"
            );
        }
    }
    Ok(())
}

#[test]
fn scoped_run_limits_to_package() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    let scope_path = project.path().join("packages/app");

    let output = harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "json",
            "--scope",
            scope_path
                .to_str()
                .context("expected scope path to be valid UTF-8")?,
        ])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let lib_violations: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .is_ok_and(|s| s.contains("packages/lib"))
        })
        .collect();

    assert!(
        lib_violations.is_empty(),
        "scoped run should not include violations from packages/lib"
    );
    Ok(())
}

#[test]
fn nested_scope_inherits_ancestor_child_config() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    let scope_path = project.path().join("packages/app/src");

    let output = harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "json",
            "--scope",
            scope_path
                .to_str()
                .context("expected scope path to be valid UTF-8")?,
        ])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    assert!(!violations.is_empty(), "scoped run should lint app files");

    let app_console: Vec<&Value> = violations
        .iter()
        .filter(|v| v["rule"].as_str() == Some("no-console"))
        .collect();

    for violation in &app_console {
        assert_eq!(
            violation["severity"]
                .as_str()
                .context("expected severity string")?,
            "error",
            "nested scope should inherit the packages/app child config"
        );
    }
    Ok(())
}

#[test]
fn deny_child_configs_fails_with_nested_configs() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--deny-child-configs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--deny-child-configs"));
    Ok(())
}

#[test]
fn deny_child_configs_succeeds_with_scope_excluding_children() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    let scope_path = project.path().join("packages/lib");

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--deny-child-configs",
            "--fail-on",
            "error",
            "--scope",
            scope_path
                .to_str()
                .context("expected scope path to be valid UTF-8")?,
        ])
        .assert()
        .success();
    Ok(())
}

#[test]
fn config_merging_produces_expected_severities() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let lib_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .is_ok_and(|s| s.contains("packages/lib"))
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    for violation in &lib_console {
        assert_eq!(
            violation["severity"]
                .as_str()
                .context("expected severity string")?,
            "warning",
            "lib package should inherit root no-console severity (warning)"
        );
    }

    let app_console: Vec<&Value> = violations
        .iter()
        .filter(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .is_ok_and(|s| s.contains("packages/app"))
                && v["rule"].as_str() == Some("no-console")
        })
        .collect();

    for violation in &app_console {
        assert_eq!(
            violation["severity"]
                .as_str()
                .context("expected severity string")?,
            "error",
            "app package should override no-console to error"
        );
    }
    Ok(())
}
