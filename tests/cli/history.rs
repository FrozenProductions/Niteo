use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;

fn set_history_config(project: &std::path::Path, enabled: bool) -> Result<()> {
    let config_path = project.join("niteo.toml");
    let source = std::fs::read_to_string(&config_path)?;
    let updated = source.replace("[project]\n", &format!("[project]\nhistory = {enabled}\n"));
    std::fs::write(config_path, updated)?;
    Ok(())
}

#[test]
fn lint_appends_history_entry() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .failure();

    let history_path = project.path().join(".niteo").join("history.jsonl");
    let contents = std::fs::read_to_string(&history_path)?;
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 1);

    let entry: serde_json::Value = serde_json::from_str(lines[0])?;
    assert!(entry["timestamp"].as_str().is_some());
    assert_eq!(entry["files"], 3);
    assert!(
        entry["violations"]
            .as_u64()
            .context("expected violations")?
            > 0
    );
    assert!(entry["health_score"].as_u64().context("expected score")? <= 100);
    Ok(())
}

#[test]
fn lint_respects_history_disabled_config() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    set_history_config(project.path(), false)?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();

    let history_path = project.path().join(".niteo").join("history.jsonl");
    assert!(
        !history_path.exists(),
        "history file should not be created when project.history is false"
    );
    Ok(())
}

#[test]
fn lint_history_flag_forces_history_when_config_disabled() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    set_history_config(project.path(), false)?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--history"])
        .assert()
        .success();

    let history_path = project.path().join(".niteo").join("history.jsonl");
    assert!(
        history_path.exists(),
        "--history should create a history entry even when project.history is false"
    );
    Ok(())
}

#[test]
fn history_score_matches_lint_summary_score() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let report: serde_json::Value = serde_json::from_str(&stdout)?;
    let summary_score = report["summary"]["score"]
        .as_u64()
        .context("expected summary score")?;

    let history_path = project.path().join(".niteo").join("history.jsonl");
    let contents = std::fs::read_to_string(&history_path)?;
    let first_line = contents.lines().next().context("expected history entry")?;
    let entry: serde_json::Value = serde_json::from_str(first_line)?;
    let history_score = entry["health_score"]
        .as_u64()
        .context("expected history score")?;

    assert_eq!(history_score, summary_score);
    Ok(())
}

#[test]
fn stats_history_renders_text() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();

    harness::niteo_in_project(project.path())
        .args(["stats", "--history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Health Score History"))
        .stdout(predicate::str::contains("Score"))
        .stdout(predicate::str::contains("100"));
    Ok(())
}

#[test]
fn stats_history_renders_json() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();
    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();

    let output = harness::niteo_in_project(project.path())
        .args(["stats", "--history", "--format", "json"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&stdout)?;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["health_score"], 100);
    assert_eq!(entries[1]["health_score"], 100);
    Ok(())
}
