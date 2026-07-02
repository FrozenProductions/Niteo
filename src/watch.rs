use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent, new_debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::syntax;
use std::sync::mpsc;
use std::time::Duration;

pub fn run(
    watch_root: &Path,
    debounce_ms: u64,
    mut lint_fn: impl FnMut(Option<&[PathBuf]>) -> Result<ExitCode>,
) -> Result<()> {
    println!(
        "Watching {} for changes... (press Ctrl+C to stop)\n",
        watch_root.display()
    );

    if let Err(error) = lint_fn(None) {
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
                let relevant = collect_relevant_changes(&events);
                if relevant.changed_files.is_empty() && !relevant.config_changed {
                    continue;
                }

                if relevant.config_changed {
                    println!("\n--- config change detected, re-linting project ---\n");
                    if let Err(error) = lint_fn(None) {
                        eprintln!("re-lint failed: {error}");
                    }
                    continue;
                }

                let changed_files: Vec<PathBuf> = relevant.changed_files.into_iter().collect();
                println!(
                    "\n--- change detected, re-linting {} file(s) ---\n",
                    changed_files.len()
                );
                if let Err(error) = lint_fn(Some(&changed_files)) {
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

struct RelevantChanges {
    config_changed: bool,
    changed_files: HashSet<PathBuf>,
}

fn collect_relevant_changes(events: &[DebouncedEvent]) -> RelevantChanges {
    let mut config_changed = false;
    let mut changed_files: HashSet<PathBuf> = HashSet::new();

    for event in events {
        let is_remove = matches!(
            event.kind,
            EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From))
        );

        for path in &event.paths {
            if is_config_file(path) {
                config_changed = true;
                continue;
            }

            if path.is_dir() {
                if is_remove {
                    changed_files.insert(path.clone());
                }
                continue;
            }

            if !is_relevant_path(path) {
                continue;
            }

            changed_files.insert(path.clone());
        }
    }

    RelevantChanges {
        config_changed,
        changed_files,
    }
}

fn is_relevant_path(path: &Path) -> bool {
    if syntax::is_typescript_file(path) {
        return true;
    }

    is_config_file(path)
}

fn is_config_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "niteo.toml" || name == ".niteo.toml")
}
