use anyhow::Result;
use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

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


