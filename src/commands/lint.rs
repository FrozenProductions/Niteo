use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use crate::analysis::{self, AnalysisOptions};
use crate::baseline;
use crate::cli::{FailOn, OutputFormat};
use crate::report;
#[derive(Clone)]
pub struct LintOptions {
    pub verbose: bool,
    pub git_flag: bool,
    pub output_format: OutputFormat,
    pub output_path: Option<PathBuf>,
    pub baseline_path: PathBuf,
    pub report_suppressions: bool,
    pub fail_on: FailOn,
    pub deny_child_configs: bool,
}

pub fn lint_workspace(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    opts: LintOptions,
    prompt_for_changed_files: bool,
) -> anyhow::Result<ExitCode> {
    let start = Instant::now();
    let collected = analysis::collect(
        workspace,
        AnalysisOptions {
            root_override: root_override.clone(),
            scope_override,
            git_flag: opts.git_flag,
            prompt_for_changed_files,
            deny_child_configs: opts.deny_child_configs,
        },
    )?;
    let resolved_baseline_path = crate::analysis::resolve_path(workspace, opts.baseline_path);
    let filtered_violations = match baseline::read_baseline(&resolved_baseline_path)? {
        Some(baseline) => {
            baseline.filter_new_violations(&collected.project_root, collected.violations)
        }
        None => collected.violations,
    };

    let mut report = report::Report::new(collected.files, filtered_violations);
    if opts.report_suppressions {
        report = report.with_suppression_report(collected.suppression_report);
    }
    let threshold = match opts.fail_on {
        FailOn::Error => report::FailureThreshold::Error,
        FailOn::Warn => report::FailureThreshold::Warn,
        FailOn::Any => report::FailureThreshold::Any,
    };
    let has_violations = report.has_findings_at_or_above(threshold);
    let rendered_report = match opts.output_format {
        OutputFormat::Text => report.render_text(opts.verbose),
        OutputFormat::Json => report.render_json()?,
        OutputFormat::Sarif => report.render_sarif()?,
        OutputFormat::Ndjson => report.render_ndjson()?,
    };

    super::write_report(workspace, opts.output_path, &rendered_report)?;

    if opts.verbose && matches!(opts.output_format, OutputFormat::Text) {
        let elapsed = start.elapsed();
        println!("\nDone in {:.2?}", elapsed);
    }

    if has_violations {
        return Ok(ExitCode::FAILURE);
    }

    Ok(ExitCode::SUCCESS)
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
