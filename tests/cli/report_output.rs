use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;
use std::fs;
use std::io::Write;

#[test]
fn clean_project_exits_zero() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn project_with_violations_exits_nonzero() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn explicit_lint_subcommand_works() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn format_text_produces_output() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--format", "text"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Niteo Structure Health"));
    Ok(())
}

#[test]
fn format_json_produces_valid_json() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(assert.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("files").is_some());
    assert!(parsed.get("violations").is_some());
    Ok(())
}

#[test]
fn format_sarif_produces_valid_sarif() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(assert.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    assert_eq!(
        parsed["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(parsed["version"], "2.1.0");
    assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "Niteo");
    Ok(())
}

#[test]
fn output_flag_writes_to_file() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    let output_path = project.path().join("report.json");

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "json",
            "--output",
            output_path
                .to_str()
                .context("expected output path to be valid UTF-8")?,
        ])
        .assert()
        .failure();

    assert!(output_path.exists());
    let contents = std::fs::read_to_string(&output_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&contents)?;
    assert!(parsed.get("summary").is_some());
    Ok(())
}

#[test]
fn fail_on_error_exits_zero_for_warnings_only() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "error"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn fail_on_any_exits_nonzero_for_info_violations() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "any"])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn fail_on_rule_overrides_default_for_specific_rule() -> Result<()> {
    let project = harness::copy_fixture("reports/warnings_only")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "error"])
        .assert()
        .success();

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--fail-on",
            "error",
            "--fail-on-rule",
            "no-console=warn",
        ])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn fail_on_category_overrides_default_for_category() -> Result<()> {
    let project = harness::copy_fixture("reports/warnings_only")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "error"])
        .assert()
        .success();

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--fail-on",
            "error",
            "--fail-on-category",
            "hygiene=warn",
        ])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn verbose_flag_shows_timing() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "-v"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Done in"));
    Ok(())
}

#[test]
fn expected_rule_ids_appear_in_json() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(assert.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;
    let rule_ids: Vec<&str> = violations
        .iter()
        .map(|v| v["rule"].as_str().context("expected rule string"))
        .collect::<Result<Vec<_>, _>>()?;

    assert!(rule_ids.contains(&"no-console"));
    assert!(rule_ids.contains(&"no-debugger"));
    assert!(rule_ids.contains(&"no-any"));
    Ok(())
}

#[test]
fn expected_file_paths_appear_in_json() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(assert.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;
    let file_paths: Vec<String> = violations
        .iter()
        .map(|v| {
            v["file"]
                .as_str()
                .context("expected file string")
                .map(|s| s.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let has_console = file_paths.iter().any(|p| p.contains("console.ts"));
    let has_any = file_paths.iter().any(|p| p.contains("any.ts"));
    assert!(has_console);
    assert!(has_any);
    Ok(())
}

#[test]
fn violation_counts_match() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let assert = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(assert.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    let summary = &parsed["summary"];
    let violation_count = summary["violations"]
        .as_u64()
        .context("expected violation count")?;
    let violations_array = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    assert_eq!(violation_count as usize, violations_array.len());
    assert!(violation_count > 0);
    Ok(())
}

#[test]
fn format_ndjson_produces_valid_ndjson() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(!lines.is_empty());

    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line)?;
        assert!(
            parsed["type"].is_string(),
            "missing 'type' field in line: {line}"
        );
    }
    Ok(())
}

#[test]
fn output_flag_writes_ndjson_to_file() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    let output_path = project.path().join("report.ndjson");

    harness::niteo_in_project(project.path())
        .args([
            "lint",
            "--format",
            "ndjson",
            "--output",
            output_path
                .to_str()
                .context("expected output path to be valid UTF-8")?,
        ])
        .assert()
        .failure();

    assert!(output_path.exists());
    let contents = std::fs::read_to_string(&output_path)?;
    let lines: Vec<&str> = contents.lines().collect();
    assert!(!lines.is_empty());

    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line)?;
        assert!(parsed["type"].is_string());
    }
    Ok(())
}

#[test]
fn fail_on_config_rule_override_makes_warning_fail() -> Result<()> {
    let project = harness::copy_fixture("reports/warnings_only")?;
    let config_path = project.path().join("niteo.toml");

    let mut config = fs::read_to_string(&config_path)?;
    config.push_str("\n[fail-on]\ndefault = \"error\"\n\n[fail-on.rules]\nno-console = \"warn\"\n");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&config_path)?;
    file.write_all(config.as_bytes())?;
    drop(file);

    harness::niteo_in_project(project.path())
        .args(["lint", "--fail-on", "error"])
        .assert()
        .failure();

    Ok(())
}
