use crate::harness;
use predicates::prelude::*;
use std::fs;

#[test]
fn git_flag_scans_only_changed_files() {
    let project = harness::copy_fixture("reports/basic").unwrap();
    harness::init_git_repo(project.path()).unwrap();
    harness::git_add_commit(project.path(), "initial").unwrap();

    let new_file = project.path().join("src/new.ts");
    fs::write(&new_file, "console.log(\"new file\");\n").unwrap();

    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .status()
        .unwrap();
    assert!(status.success());

    harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .assert()
        .stdout(predicate::str::contains("new.ts"));
}

#[test]
fn git_flag_ignores_non_typescript_changes() {
    let project = harness::copy_fixture("reports/clean").unwrap();
    harness::init_git_repo(project.path()).unwrap();
    harness::git_add_commit(project.path(), "initial").unwrap();

    let readme = project.path().join("README.md");
    fs::write(&readme, "# Changed\n").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let files = parsed["files"].as_array().unwrap();

    assert!(
        files.is_empty(),
        "no TypeScript files changed, so none should be scanned"
    );
}

#[test]
fn git_flag_outside_repo_fails_clearly() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--git"])
        .env_remove("GIT_DIR")
        .env("GIT_CEILING_DIRECTORIES", project.path())
        .assert()
        .failure();
}

#[test]
fn git_staged_files_are_scanned() {
    let project = harness::copy_fixture("reports/clean").unwrap();
    harness::init_git_repo(project.path()).unwrap();
    harness::git_add_commit(project.path(), "initial").unwrap();

    let new_file = project.path().join("src/staged.ts");
    fs::write(&new_file, "console.log(\"staged\");\n").unwrap();

    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .status()
        .unwrap();
    assert!(status.success());

    harness::niteo_in_project(project.path())
        .args(["lint", "--git", "--format", "json"])
        .assert()
        .stdout(predicate::str::contains("staged.ts"));
}
