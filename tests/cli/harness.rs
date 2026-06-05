use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

pub fn niteo_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_niteo"))
}

pub fn niteo_in_project(project: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_niteo"));
    cmd.current_dir(project);
    cmd
}

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(FIXTURES_DIR).join(name)
}

pub fn copy_fixture(name: &str) -> Result<TempDir> {
    let source = fixture_path(name);
    let temp_dir = TempDir::new()?;
    copy_dir_recursive(&source, temp_dir.path())?;
    Ok(temp_dir)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    if !destination.exists() {
        fs::create_dir_all(destination)?;
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path)?;
        }
    }

    Ok(())
}

pub fn normalize_path(output: &str, root: &Path) -> String {
    let root_string = root.display().to_string();
    output.replace(&root_string, "<ROOT>")
}

pub fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn parse_json_output(output: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(output)?;
    Ok(value)
}

pub fn init_git_repo(project: &Path) -> Result<()> {
    let output = StdCommand::new("git")
        .args(["init"])
        .current_dir(project)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(project)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git config email failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(project)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git config name failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn git_add_commit(project: &Path, message: &str) -> Result<()> {
    let output = StdCommand::new("git")
        .args(["add", "."])
        .current_dir(project)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = StdCommand::new("git")
        .args(["commit", "-m", message, "--allow-empty"])
        .current_dir(project)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn assert_exit_code(command: &mut Command, expected: i32) -> assert_cmd::assert::Assert {
    command.assert().code(expected)
}
