use anyhow::Result;
use std::fs;

use crate::harness;

#[test]
fn fix_applies_supported_fixes() -> Result<()> {
    let project = harness::copy_fixture("fix")?;

    let mut command = harness::niteo_in_project(project.path());
    command.arg("fix");
    let output = command.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "fix command failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    assert!(stdout.contains("debugger.ts"), "stdout: {stdout}");
    assert!(stdout.contains("focused.ts"), "stdout: {stdout}");
    assert!(stdout.contains("skipped.ts"), "stdout: {stdout}");
    assert!(stdout.contains("empty-interface.ts"), "stdout: {stdout}");

    let debugger = fs::read_to_string(project.path().join("src/debugger.ts"))?;
    assert!(!debugger.contains("debugger"));

    let focused = fs::read_to_string(project.path().join("src/focused.ts"))?;
    assert!(!focused.contains(".only"));
    assert!(focused.contains("describe("));
    assert!(focused.contains("it("));

    let skipped = fs::read_to_string(project.path().join("src/skipped.ts"))?;
    assert!(!skipped.contains(".skip"));
    assert!(skipped.contains("describe("));
    assert!(skipped.contains("test("));

    let empty_interface = fs::read_to_string(project.path().join("src/empty-interface.ts"))?;
    assert!(empty_interface.contains("type Empty = Record<string, never>;"));
    assert!(empty_interface.contains("export type Empty"));
    assert!(empty_interface.contains("type OtherEmpty = Record<string, never>;"));
    Ok(())
}

#[test]
fn fix_dry_run_does_not_write_files() -> Result<()> {
    let project = harness::copy_fixture("fix")?;
    let original_debugger = fs::read_to_string(project.path().join("src/debugger.ts"))?;

    let mut command = harness::niteo_in_project(project.path());
    command.args(["fix", "--dry-run"]);
    let output = command.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "dry-run fix command failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    assert!(stdout.contains("no-debugger"), "stdout: {stdout}");
    assert!(stdout.contains("no-focused-test"), "stdout: {stdout}");
    assert!(stdout.contains("no-skipped-test"), "stdout: {stdout}");
    assert!(stdout.contains("no-empty-interface"), "stdout: {stdout}");

    let debugger = fs::read_to_string(project.path().join("src/debugger.ts"))?;
    assert_eq!(debugger, original_debugger);
    Ok(())
}

#[test]
fn fix_is_idempotent() -> Result<()> {
    let project = harness::copy_fixture("fix")?;

    let mut first = harness::niteo_in_project(project.path());
    first.arg("fix");
    let first_output = first.output()?;
    assert!(first_output.status.success());

    let first_fixed = fs::read_to_string(project.path().join("src/focused.ts"))?;

    let mut second = harness::niteo_in_project(project.path());
    second.arg("fix");
    let second_output = second.output()?;
    let second_stdout = String::from_utf8_lossy(&second_output.stdout);
    assert!(second_output.status.success());
    assert!(
        second_stdout.contains("No fixable violations found."),
        "second fix should find no violations: {second_stdout}"
    );

    let second_fixed = fs::read_to_string(project.path().join("src/focused.ts"))?;
    assert_eq!(second_fixed, first_fixed);
    Ok(())
}

#[test]
fn lint_without_fix_is_unchanged() -> Result<()> {
    let project = harness::copy_fixture("fix")?;
    let original = fs::read_to_string(project.path().join("src/focused.ts"))?;

    let mut command = harness::niteo_in_project(project.path());
    command.arg("lint");
    let output = command.output()?;
    assert!(!output.status.success());

    let after = fs::read_to_string(project.path().join("src/focused.ts"))?;
    assert_eq!(after, original);
    Ok(())
}

#[test]
fn fix_with_watch_is_rejected() -> Result<()> {
    let project = harness::copy_fixture("fix")?;

    let mut command = harness::niteo_in_project(project.path());
    command.args(["--watch", "--fix"]);
    let output = command.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "expected --fix with --watch to fail: {stderr}"
    );
    assert!(
        stderr.contains("--fix") && stderr.contains("--watch"),
        "stderr should mention the conflicting flags: {stderr}"
    );
    Ok(())
}
