use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;
use std::fs;

#[test]
fn git_flag_scans_only_changed_files() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let new_file = project.path().join("src/new.ts");
    fs::write(&new_file, "console.log(\"new file\");\n")?;

    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .assert()
        .stdout(predicate::str::contains("new.ts"));
    Ok(())
}

#[test]
fn git_flag_ignores_non_typescript_changes() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let readme = project.path().join("README.md");
    fs::write(&readme, "# Changed\n")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let files = parsed["files"].as_array().context("expected files array")?;

    assert!(
        files.is_empty(),
        "no TypeScript files changed, so none should be scanned"
    );
    Ok(())
}

#[test]
fn git_flag_outside_repo_fails_clearly() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--git"])
        .env_remove("GIT_DIR")
        .env("GIT_CEILING_DIRECTORIES", project.path())
        .assert()
        .failure();
    Ok(())
}

#[test]
fn git_staged_files_are_scanned() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let new_file = project.path().join("src/staged.ts");
    fs::write(&new_file, "console.log(\"staged\");\n")?;

    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .assert()
        .stdout(predicate::str::contains("staged.ts"));
    Ok(())
}
