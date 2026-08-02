use crate::harness;
use anyhow::{Context, Result};
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn lint_json_files(project: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = harness::niteo_in_project(project).args(args).output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    Ok(parsed["files"]
        .as_array()
        .context("expected files array")?
        .iter()
        .filter_map(|file| file.as_str().map(|path| path.to_string()))
        .collect())
}

fn ui_scope(project: &Path) -> Result<String> {
    Ok(project
        .join("packages/ui")
        .to_str()
        .context("scope must be valid UTF-8")?
        .to_string())
}

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
fn git_staged_flag_excludes_unstaged_changes() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let staged_file = project.path().join("src/staged_only.ts");
    fs::write(&staged_file, "console.log(\"staged\");\n")?;
    let status = std::process::Command::new("git")
        .args(["add", "src/staged_only.ts"])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let working_tree_file = project.path().join("src/working_tree_only.ts");
    fs::write(&working_tree_file, "console.log(\"working tree\");\n")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git-staged", "--format", "json"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("staged_only.ts"));
    assert!(!stdout.contains("working_tree_only.ts"));
    Ok(())
}

#[test]
fn git_unstaged_flag_excludes_staged_changes() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let staged_file = project.path().join("src/staged_only.ts");
    fs::write(&staged_file, "console.log(\"staged\");\n")?;
    let status = std::process::Command::new("git")
        .args(["add", "src/staged_only.ts"])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let working_tree_file = project.path().join("src/working_tree_only.ts");
    fs::write(&working_tree_file, "console.log(\"working tree\");\n")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git-unstaged", "--format", "json"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("working_tree_only.ts"));
    assert!(!stdout.contains("staged_only.ts"));
    Ok(())
}

#[test]
fn git_range_scans_files_changed_in_range() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let baseline = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project.path())
        .output()?;
    let base_sha = String::from_utf8(baseline.stdout)?.trim().to_string();

    let added = project.path().join("src/in_range.ts");
    fs::write(&added, "console.log(\"in range\");\n")?;
    harness::git_add_commit(project.path(), "add in_range")?;

    let range = format!("{base_sha}..HEAD");
    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git", &range, "--format", "json"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("in_range.ts"),
        "expected in_range.ts in output, got: {stdout}"
    );
    Ok(())
}

#[test]
fn git_flags_are_mutually_exclusive() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--git-staged", "--git-unstaged"])
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

#[test]
fn git_flag_excludes_changes_outside_project_root() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let in_root = project.path().join("src/any.ts");
    fs::write(&in_root, "let x: any = 1;\nconsole.log(x);\n")?;
    let outside_root = project.path().join("out-of-root.ts");
    fs::write(&outside_root, "console.log(\"outside\");\n")?;

    let files = lint_json_files(project.path(), &["lint", "--git", "--format", "json"])?;
    assert!(files.iter().any(|file| file.ends_with("src/any.ts")));
    assert!(
        files.iter().all(|file| !file.ends_with("out-of-root.ts")),
        "changed file outside [project].root must be excluded: {files:?}"
    );
    Ok(())
}

#[test]
fn git_scope_flag_excludes_changes_outside_scope() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    fs::write(
        project.path().join("packages/ui/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;
    let out_of_scope = project.path().join("packages/app/src/App.tsx");
    fs::write(
        &out_of_scope,
        fs::read_to_string(&out_of_scope)?.replace("export", "export /* changed */"),
    )?;

    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git",
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("packages/ui/src/button.ts"))
    );
    assert!(
        files.iter().all(|file| !file.ends_with("App.tsx")),
        "changed file outside --scope must be excluded: {files:?}"
    );
    Ok(())
}

#[test]
fn git_flag_excludes_tsconfig_excluded_changed_file() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::write(project.path().join("tsconfig.json"), r#"{"include": []}"#)?;
    fs::write(
        project.path().join("src/new.ts"),
        "console.log(\"excluded\");\n",
    )?;

    let files = lint_json_files(project.path(), &["lint", "--git", "--format", "json"])?;
    assert!(
        files.is_empty(),
        "tsconfig-excluded changed file must not be linted: {files:?}"
    );
    Ok(())
}

#[test]
fn git_flag_excludes_ignored_tracked_file_when_respect_gitignore() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let config = project.path().join("niteo.toml");
    let source = fs::read_to_string(&config)?;
    fs::write(&config, source.replace("root = \"src\"", "root = \".\""))?;
    fs::write(project.path().join(".gitignore"), "src/index.ts\n")?;
    let tracked = project.path().join("src/index.ts");
    fs::write(&tracked, "console.log(\"changed\");\n")?;

    let files = lint_json_files(project.path(), &["lint", "--git", "--format", "json"])?;
    assert!(
        files.is_empty(),
        "gitignored changed file must follow respect-gitignore: {files:?}"
    );
    Ok(())
}

#[test]
fn git_flag_lints_ignored_tracked_file_when_respect_gitignore_false() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let config = project.path().join("niteo.toml");
    let source = fs::read_to_string(&config)?;
    fs::write(
        &config,
        source.replace("root = \"src\"", "root = \".\"\nrespect-gitignore = false"),
    )?;
    fs::write(project.path().join(".gitignore"), "src/index.ts\n")?;
    let tracked = project.path().join("src/index.ts");
    fs::write(&tracked, "console.log(\"changed\");\n")?;

    let files = lint_json_files(project.path(), &["lint", "--git", "--format", "json"])?;
    assert!(
        files.iter().any(|file| file.ends_with("src/index.ts")),
        "gitignored file must be linted when respect-gitignore is false: {files:?}"
    );
    Ok(())
}

#[test]
fn git_staged_flag_applies_scope_filter() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    fs::write(
        project.path().join("packages/ui/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;
    let out_of_scope = project.path().join("packages/app/src/App.tsx");
    fs::write(
        &out_of_scope,
        fs::read_to_string(&out_of_scope)?.replace("export", "export /* changed */"),
    )?;
    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git-staged",
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("packages/ui/src/button.ts"))
    );
    assert!(
        files.iter().all(|file| !file.ends_with("App.tsx")),
        "staged file outside --scope must be excluded: {files:?}"
    );
    Ok(())
}

#[test]
fn git_unstaged_flag_applies_scope_filter() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let out_of_scope = project.path().join("packages/app/src/App.tsx");
    fs::write(
        &out_of_scope,
        fs::read_to_string(&out_of_scope)?.replace("export", "export /* changed */"),
    )?;
    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    fs::write(
        project.path().join("packages/ui/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;

    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git-unstaged",
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("packages/ui/src/button.ts"))
    );
    assert!(
        files.iter().all(|file| !file.ends_with("App.tsx")),
        "unstaged file outside --scope must be excluded: {files:?}"
    );
    Ok(())
}

#[test]
fn git_range_flag_applies_scope_filter() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let baseline = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project.path())
        .output()?;
    let base_sha = String::from_utf8(baseline.stdout)?.trim().to_string();

    let out_of_scope = project.path().join("packages/app/src/App.tsx");
    fs::write(
        &out_of_scope,
        fs::read_to_string(&out_of_scope)?.replace("export", "export /* changed */"),
    )?;
    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    fs::write(
        project.path().join("packages/ui/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;
    harness::git_add_commit(project.path(), "changes")?;

    let range = format!("{base_sha}..HEAD");
    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git",
            &range,
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("packages/ui/src/button.ts"))
    );
    assert!(
        files.iter().all(|file| !file.ends_with("App.tsx")),
        "range file outside --scope must be excluded: {files:?}"
    );
    Ok(())
}

#[test]
fn git_staged_rename_selects_new_path_once() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    let status = std::process::Command::new("git")
        .args(["mv", "src/clean.ts", "src/renamed.ts"])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let files = lint_json_files(project.path(), &["lint", "--git", "--format", "json"])?;
    assert_eq!(
        files.len(),
        1,
        "rename must be selected exactly once: {files:?}"
    );
    assert!(files[0].ends_with("src/renamed.ts"), "files: {files:?}");
    Ok(())
}

#[test]
fn git_rename_into_scope_selects_new_path_once() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::create_dir_all(project.path().join("packages/app/src"))?;
    fs::write(
        project.path().join("packages/app/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;
    harness::git_add_commit(project.path(), "add button")?;

    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    let status = std::process::Command::new("git")
        .args([
            "mv",
            "packages/app/src/button.ts",
            "packages/ui/src/button.ts",
        ])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git",
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert_eq!(
        files.len(),
        1,
        "rename must be selected exactly once: {files:?}"
    );
    assert!(
        files[0].ends_with("packages/ui/src/button.ts"),
        "files: {files:?}"
    );
    Ok(())
}

#[test]
fn git_rename_out_of_scope_leaks_nothing() -> Result<()> {
    let project = harness::copy_fixture("monorepo")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::create_dir_all(project.path().join("packages/ui/src"))?;
    fs::write(
        project.path().join("packages/ui/src/button.ts"),
        "export function button() {\n  console.log(\"button\");\n}\n",
    )?;
    harness::git_add_commit(project.path(), "add button")?;

    fs::create_dir_all(project.path().join("packages/app/src"))?;
    let status = std::process::Command::new("git")
        .args([
            "mv",
            "packages/ui/src/button.ts",
            "packages/app/src/button.ts",
        ])
        .current_dir(project.path())
        .status()?;
    assert!(status.success());

    let files = lint_json_files(
        project.path(),
        &[
            "lint",
            "--git",
            "--scope",
            &ui_scope(project.path())?,
            "--format",
            "json",
        ],
    )?;
    assert!(
        files.is_empty(),
        "rename out of scope must not leak or duplicate a path: {files:?}"
    );
    Ok(())
}

#[test]
fn git_verbose_diagnostic_explains_excluded_file() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    harness::init_git_repo(project.path())?;
    harness::git_add_commit(project.path(), "initial")?;

    fs::write(
        project.path().join("out-of-root.ts"),
        "console.log(\"outside\");\n",
    )?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--git", "-v", "--format", "json"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let diagnostics = parsed["diagnostics"]
        .as_array()
        .context("expected diagnostics array")?;
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["message"].as_str().is_some_and(|message| {
                message.contains("skipping") && message.contains("out-of-root.ts")
            })
        }),
        "verbose output should explain the excluded file: {diagnostics:?}"
    );
    Ok(())
}
