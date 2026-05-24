use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

pub fn get_changed_typescript_files() -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output();

    let staged_output = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .output();

    let mut files: Vec<PathBuf> = Vec::new();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if is_typescript_file(line) {
                files.push(PathBuf::from(line));
            }
        }
    }

    if let Ok(output) = staged_output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if is_typescript_file(line) {
                let path = PathBuf::from(line);
                if !files.contains(&path) {
                    files.push(path);
                }
            }
        }
    }

    files
}

pub fn prompt_scan_changed_files(changed_files: &[PathBuf]) -> bool {
    if changed_files.is_empty() {
        return false;
    }

    println!("Found {} changed TypeScript file(s):", changed_files.len());
    for file in changed_files {
        println!("  {}", file.display());
    }
    println!();

    print!("Scan only changed files? [Y/n] ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

    let input = input.trim().to_lowercase();
    input.is_empty() || input == "y" || input == "yes"
}

fn is_typescript_file(path: &str) -> bool {
    path.ends_with(".ts") || path.ends_with(".tsx")
}
