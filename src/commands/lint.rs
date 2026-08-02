use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use crate::analysis::{self, AnalysisOptions, AnalysisResult};
use crate::baseline;
use crate::cli::OutputFormat;
use crate::config::{FailurePolicy, FailureThreshold, RuleCategory};
use crate::history;
use crate::report;

#[derive(Clone)]
pub struct LintOptions {
    pub verbose: u8,
    pub git_selection: Option<crate::git::GitSelection>,
    pub output_format: OutputFormat,
    pub output_path: Option<PathBuf>,
    pub baseline_path: PathBuf,
    pub report_suppressions: bool,
    pub fail_on: Option<FailureThreshold>,
    pub fail_on_rules: HashMap<String, FailureThreshold>,
    pub fail_on_categories: HashMap<RuleCategory, FailureThreshold>,
    pub deny_child_configs: bool,
    pub cache_enabled: bool,
    pub clear_cache: bool,
    pub force_history: bool,
    pub directory_inventory: Option<Arc<crate::directory_inventory::DirectoryInventory>>,
}

pub fn lint_workspace_with_result(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    opts: LintOptions,
    prompt_for_changed_files: bool,
) -> anyhow::Result<(ExitCode, Arc<AnalysisResult>)> {
    let collected = analysis::collect(
        workspace,
        AnalysisOptions {
            root_override: root_override.clone(),
            scope_override,
            git_selection: opts.git_selection.clone(),
            prompt_for_changed_files,
            deny_child_configs: opts.deny_child_configs,
            cache_enabled: opts.cache_enabled,
            clear_cache: opts.clear_cache,
            verbose: opts.verbose,
            directory_inventory: opts.directory_inventory.clone(),
        },
    )?;
    publish_report(workspace, collected, opts)
}

pub fn lint_workspace_incremental(
    workspace: &Path,
    previous: &Arc<AnalysisResult>,
    changed_files: &[PathBuf],
    opts: LintOptions,
) -> anyhow::Result<(ExitCode, Arc<AnalysisResult>)> {
    let collected = analysis::collect_incremental(workspace, previous, changed_files)?;
    publish_report(workspace, collected, opts)
}

fn publish_report(
    workspace: &Path,
    collected: Arc<AnalysisResult>,
    opts: LintOptions,
) -> anyhow::Result<(ExitCode, Arc<AnalysisResult>)> {
    let start = Instant::now();
    let resolved_baseline_path = crate::analysis::resolve_path(workspace, opts.baseline_path);
    let all_violations: Vec<crate::rules::Violation> = collected
        .violations
        .iter()
        .cloned()
        .chain(collected.directory_violations.iter().cloned())
        .collect();
    let filtered_violations = match baseline::read_baseline(&resolved_baseline_path)? {
        Some(baseline) => baseline.filter_new_violations(&collected.project_root, all_violations),
        None => all_violations,
    };

    let mut report = report::Report::new(collected.files.clone(), filtered_violations);
    if opts.report_suppressions {
        report = report.with_suppression_report(collected.suppression_report.clone());
    }
    report = report.with_diagnostics(collected.diagnostics.clone());
    report = report.with_parse_failures(collected.parse_failures.clone());
    let threshold = match opts.fail_on {
        Some(threshold) => threshold,
        None => collected.fail_on.default,
    };
    let mut policy = FailurePolicy {
        default: threshold,
        rules: collected.fail_on.rules.clone(),
        categories: collected.fail_on.categories.clone(),
    };
    policy.rules.extend(opts.fail_on_rules);
    policy.categories.extend(opts.fail_on_categories);
    let has_violations = report.has_findings_matching(&policy);
    let rendered_report = match opts.output_format {
        OutputFormat::Text => report.render_text(opts.verbose),
        OutputFormat::Json => report.render_json()?,
        OutputFormat::Sarif => report.render_sarif()?,
        OutputFormat::Ndjson => report.render_ndjson()?,
        OutputFormat::Markdown => report.render_markdown()?,
    };

    super::write_report(workspace, opts.output_path, &rendered_report)?;
    if collected.history_enabled || opts.force_history {
        history::append_to_workspace(workspace, &report)?;
    }

    if opts.verbose >= 1 && matches!(opts.output_format, OutputFormat::Text) {
        let elapsed = start.elapsed();
        println!("\nDone in {:.2?}", elapsed);
    }

    // A parse failure is an operational failure: the source is invalid or
    // incomplete, so the lint result is not trustworthy. It fails the run even
    // when no violation meets the configured severity threshold.
    let exit_code = if has_violations || report.has_parse_failures() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    };

    Ok((exit_code, collected))
}

pub fn resolve_watch_root(
    workspace: &Path,
    root_override: Option<&Path>,
) -> anyhow::Result<PathBuf> {
    if let Some(root) = root_override {
        return Ok(crate::analysis::resolve_path(workspace, root.to_path_buf()));
    }

    let project_config = crate::config::ProjectConfig::resolve(workspace, None)?;
    Ok(project_config.root)
}
