use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::analysis::{self, AnalysisOptions};
use crate::baseline as baseline_mod;
use crate::report;

pub fn create(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_selection: Option<crate::git::GitSelection>,
    baseline_path: PathBuf,
    report_suppressions: bool,
    deny_child_configs: bool,
) -> Result<()> {
    let collected = analysis::collect(
        workspace,
        AnalysisOptions {
            root_override: root_override.clone(),
            scope_override,
            git_selection: git_selection.clone(),
            prompt_for_changed_files: false,
            deny_child_configs,
            cache_enabled: false,
            clear_cache: false,
        },
    )?;

    if report_suppressions {
        let rendered = report::render_suppression_report_text(&collected.suppression_report);
        if !rendered.is_empty() {
            print!("{rendered}");
        }
    }

    let resolved_baseline_path = crate::analysis::resolve_path(workspace, baseline_path);
    let baseline =
        baseline_mod::Baseline::from_violations(&collected.project_root, &collected.violations);

    baseline_mod::write_baseline(&resolved_baseline_path, &baseline)?;

    println!(
        "Created {} with {} violations",
        resolved_baseline_path.display(),
        baseline.violation_count()
    );

    Ok(())
}

pub fn prune(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_selection: Option<crate::git::GitSelection>,
    baseline_path: PathBuf,
    deny_child_configs: bool,
) -> Result<()> {
    let resolved_baseline_path = crate::analysis::resolve_path(workspace, baseline_path.clone());
    let Some(existing_baseline) = baseline_mod::read_baseline(&resolved_baseline_path)? else {
        bail!("No baseline file found at {}", baseline_path.display());
    };

    let collected = analysis::collect(
        workspace,
        AnalysisOptions {
            root_override,
            scope_override,
            git_selection: git_selection.clone(),
            prompt_for_changed_files: false,
            deny_child_configs,
            cache_enabled: false,
            clear_cache: false,
        },
    )?;
    let result = existing_baseline.prune(&collected.project_root, &collected.violations);

    baseline_mod::write_baseline(&resolved_baseline_path, &result.baseline)?;

    if result.removed_count > 0 {
        println!(
            "Pruned {}: removed {} stale entries ({} remaining)",
            resolved_baseline_path.display(),
            result.removed_count,
            result.baseline.violation_count(),
        );
    } else {
        println!(
            "Baseline is up to date ({} entries)",
            result.baseline.violation_count(),
        );
    }

    Ok(())
}
