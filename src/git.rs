use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::syntax;

#[derive(Debug, Clone)]
pub enum GitSelection {
    /// Working-tree changes plus staged changes (diff against HEAD plus index).
    WorkingTree,
    /// Only staged changes (index vs HEAD).
    Staged,
    /// Only unstaged working-tree changes (working tree vs index).
    Unstaged,
    /// Files changed in a revision range, e.g. `main..HEAD`.
    Range(String),
}

pub fn get_changed_typescript_files(selection: &GitSelection) -> Result<Vec<PathBuf>> {
    match selection {
        GitSelection::WorkingTree => {
            let mut files = run_git_paths(&["diff", "--name-only", "HEAD"])?;
            merge_unique(
                &mut files,
                run_git_paths(&["diff", "--name-only", "--cached"])?,
            );
            merge_unique(&mut files, run_git_paths(&untracked_args())?);
            Ok(files)
        }
        GitSelection::Staged => run_git_paths(&["diff", "--name-only", "--cached"]),
        GitSelection::Unstaged => {
            let mut files = run_git_paths(&["diff", "--name-only"])?;
            merge_unique(&mut files, run_git_paths(&untracked_args())?);
            Ok(files)
        }
        GitSelection::Range(range) => {
            validate_range(range)?;
            run_git_paths(&["diff", "--name-only", range])
        }
    }
}

fn untracked_args() -> [&'static str; 3] {
    ["ls-files", "--others", "--exclude-standard"]
}

fn merge_unique(target: &mut Vec<PathBuf>, additions: Vec<PathBuf>) {
    for path in additions {
        if !target.contains(&path) {
            target.push(path);
        }
    }
}

fn run_git_paths(args: &[&str]) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout.lines() {
        if is_typescript_file(line) {
            files.push(PathBuf::from(line));
        }
    }
    Ok(files)
}

fn validate_range(range: &str) -> Result<()> {
    if range.is_empty() {
        bail!("git range cannot be empty");
    }
    if range.starts_with('-') {
        bail!("git range cannot start with '-': {range}");
    }
    Ok(())
}

pub fn prompt_scan_changed_files(changed_files: &[PathBuf]) -> Result<bool> {
    if changed_files.is_empty() {
        return Ok(false);
    }

    println!("Found {} changed TypeScript file(s):", changed_files.len());
    for file in changed_files {
        println!("  {}", file.display());
    }
    println!();

    print!("Scan only changed files? [Y/n] ");
    io::stdout()
        .flush()
        .with_context(|| "failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .with_context(|| "failed to read stdin")?;

    let input = input.trim().to_lowercase();
    Ok(input.is_empty() || input == "y" || input == "yes")
}

fn is_typescript_file(path: &str) -> bool {
    syntax::is_typescript_file(Path::new(path))
}
