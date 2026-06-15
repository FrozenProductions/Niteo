use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

pub fn get_changed_typescript_files() -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output()
        .with_context(|| "failed to execute git diff HEAD")?;

    let staged_output = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .output()
        .with_context(|| "failed to execute git diff --cached")?;

    let mut files: Vec<PathBuf> = Vec::new();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff HEAD failed: {}", stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if is_typescript_file(line) {
            files.push(PathBuf::from(line));
        }
    }

    if !staged_output.status.success() {
        let stderr = String::from_utf8_lossy(&staged_output.stderr);
        bail!("git diff --cached failed: {}", stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&staged_output.stdout);
    for line in stdout.lines() {
        if is_typescript_file(line) {
            let path = PathBuf::from(line);
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }

    Ok(files)
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
    path.ends_with(".ts") || path.ends_with(".tsx")
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    #[test]
    fn recognizes_ts_files() -> Result<()> {
        assert!(is_typescript_file("src/index.ts"));
        assert!(is_typescript_file("foo.ts"));

        Ok(())
    }

    #[test]
    fn recognizes_tsx_files() -> Result<()> {
        assert!(is_typescript_file("src/Component.tsx"));
        assert!(is_typescript_file("App.tsx"));

        Ok(())
    }

    #[test]
    fn rejects_non_typescript_files() -> Result<()> {
        assert!(!is_typescript_file("src/index.js"));
        assert!(!is_typescript_file("src/index.jsx"));
        assert!(!is_typescript_file("src/style.css"));
        assert!(!is_typescript_file("README.md"));
        assert!(!is_typescript_file("Cargo.toml"));

        Ok(())
    }

    #[test]
    fn rejects_partial_extensions() -> Result<()> {
        assert!(!is_typescript_file("src/file.ats"));
        assert!(!is_typescript_file("src/file.atsx"));
        assert!(!is_typescript_file("src/file.d.ts.map"));

        Ok(())
    }

    #[test]
    fn handles_paths_with_directories() -> Result<()> {
        assert!(is_typescript_file("a/b/c/deep.ts"));
        assert!(is_typescript_file("a/b/c/deep.tsx"));
        assert!(!is_typescript_file("a/b/c/deep.js"));

        Ok(())
    }
}
