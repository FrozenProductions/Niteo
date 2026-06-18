use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::syntax;
use std::sync::mpsc;
use std::time::Duration;

pub fn run(
    watch_root: &Path,
    debounce_ms: u64,
    mut lint_fn: impl FnMut() -> Result<ExitCode>,
) -> Result<()> {
    println!(
        "Watching {} for changes... (press Ctrl+C to stop)\n",
        watch_root.display()
    );

    if let Err(error) = lint_fn() {
        eprintln!("initial lint failed: {error}");
    }

    let (tx, rx) = mpsc::channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(debounce_ms), None, tx)
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
                if let Err(error) = lint_fn() {
                    eprintln!("re-lint failed: {error}");
                }
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

fn is_relevant_path(path: &Path) -> bool {
    if is_typescript_file(path) {
        return true;
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "niteo.toml" || name == ".niteo.toml")
}

fn is_typescript_file(path: &Path) -> bool {
    syntax::is_typescript_file(path)
}
