use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

const DEBOUNCE_MS: u64 = 300;

pub fn run(watch_root: &Path, mut lint_fn: impl FnMut() -> Result<ExitCode>) -> Result<()> {
    println!(
        "Watching {} for changes... (press Ctrl+C to stop)\n",
        watch_root.display()
    );

    let _ = lint_fn();

    let (tx, rx) = mpsc::channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), None, tx)
        .context("failed to start filesystem watcher")?;

    debouncer
        .watch(watch_root, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch {}", watch_root.display()))?;

    for result in rx {
        match result {
            Ok(events) => {
                if !events.iter().any(|event| has_relevant_change(&event.paths)) {
                    continue;
                }
                println!("\n--- change detected, re-linting ---\n");
                let _ = lint_fn();
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("watch error: {error}");
                }
            }
        }
    }

    Ok(())
}

fn has_relevant_change(paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| is_relevant_path(path))
}

// Only TypeScript and config file changes trigger a re-lint
fn is_relevant_path(path: &Path) -> bool {
    if is_typescript_file(path) {
        return true;
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "niteo.toml" || name == ".niteo.toml")
}

fn is_typescript_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts") | Some("tsx")
    )
}
