use anyhow::{Context, Result, bail};
use clap::Parser;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::cli::{BaselineCommand, Cli, Command, OutputFormat};
use crate::ignore::SuppressionReport;
use crate::rules::Violation;
use crate::{baseline, config, discovery, git, report, rule_documentation, rules};

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let workspace = env::current_dir()?;

    let exit_code = match cli.command.unwrap_or(Command::Lint) {
        Command::Init => {
            create_config(&workspace)?;
            ExitCode::SUCCESS
        }
        Command::Baseline { command } => match command {
            BaselineCommand::Create => {
                create_baseline(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    cli.options.git,
                    cli.options.baseline,
                    cli.options.report_suppressions,
                )?;
                ExitCode::SUCCESS
            }
            BaselineCommand::Prune => {
                prune_baseline(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    cli.options.git,
                    cli.options.baseline,
                )?;
                ExitCode::SUCCESS
            }
        },
        Command::Rules => {
            list_rules(
                &workspace,
                cli.options.root,
                cli.options.format,
                cli.options.output,
            )?;
            ExitCode::SUCCESS
        }
        Command::Explain { rule } => {
            explain_rule(
                &workspace,
                cli.options.root,
                cli.options.format,
                cli.options.output,
                &rule,
            )?;
            ExitCode::SUCCESS
        }
        Command::Lint => lint_workspace(
            &workspace,
            cli.options.root,
            cli.options.scope,
            LintOptions {
                verbose: cli.options.verbose,
                git_flag: cli.options.git,
                output_format: cli.options.format,
                output_path: cli.options.output,
                baseline_path: cli.options.baseline,
                report_suppressions: cli.options.report_suppressions,
            },
        )?,
    };

    Ok(exit_code)
}

fn create_config(workspace: &Path) -> Result<()> {
    let config_path = config::write_default_config(workspace)?;
    println!("Created {}", config_path.display());

    Ok(())
}

fn create_baseline(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_flag: bool,
    baseline_path: PathBuf,
    report_suppressions: bool,
) -> Result<()> {
    let collected = collect_violations(workspace, root_override, scope_override, git_flag, false)?;

    if report_suppressions {
        let rendered = report::render_suppression_report_text(&collected.suppression_report);
        if !rendered.is_empty() {
            print!("{rendered}");
        }
    }

    let resolved_baseline_path = resolve_path(workspace, baseline_path);
    let baseline =
        baseline::Baseline::from_violations(&collected.project_root, &collected.violations);

    baseline::write_baseline(&resolved_baseline_path, &baseline)?;

    println!(
        "Created {} with {} violations",
        resolved_baseline_path.display(),
        baseline.violation_count()
    );

    Ok(())
}

fn prune_baseline(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_flag: bool,
    baseline_path: PathBuf,
) -> Result<()> {
    let resolved_baseline_path = resolve_path(workspace, baseline_path.clone());
    let Some(existing_baseline) = baseline::read_baseline(&resolved_baseline_path)? else {
        bail!("No baseline file found at {}", baseline_path.display());
    };

    let collected = collect_violations(workspace, root_override, scope_override, git_flag, false)?;
    let result = existing_baseline.prune(&collected.project_root, &collected.violations);

    baseline::write_baseline(&resolved_baseline_path, &result.baseline)?;

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

fn list_rules(
    workspace: &Path,
    root_override: Option<PathBuf>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let rows = rule_documentation::configured_rules(&project_config);

    let rendered = match output_format {
        OutputFormat::Text => {
            let name_width = rows
                .iter()
                .map(|row| row.name.len())
                .max()
                .unwrap_or("rule".len());

            let mut output = String::new();
            output.push_str(&format!("{:<name_width$}  severity\n", "rule"));
            output.push_str(&format!("{:-<name_width$}  --------\n", ""));
            for row in &rows {
                output.push_str(&format!(
                    "{:<name_width$}  {}\n",
                    row.name,
                    row.severity.as_str()
                ));
            }
            output
        }
        OutputFormat::Json => rule_documentation::render_rules_json(&rows)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'rules' command"),
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

fn explain_rule(
    workspace: &Path,
    root_override: Option<PathBuf>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
    rule_name: &str,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let explanation = rule_documentation::explain_rule(rule_name, &project_config)?;

    let rendered = match output_format {
        OutputFormat::Text => rule_documentation::render_explanation_text(&explanation),
        OutputFormat::Json => rule_documentation::render_explanation_json(&explanation)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'explain' command"),
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

struct LintOptions {
    verbose: bool,
    git_flag: bool,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
    baseline_path: PathBuf,
    report_suppressions: bool,
}

fn lint_workspace(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    opts: LintOptions,
) -> Result<ExitCode> {
    let collected = collect_violations(
        workspace,
        root_override,
        scope_override,
        opts.git_flag,
        true,
    )?;
    let resolved_baseline_path = resolve_path(workspace, opts.baseline_path);
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
    let has_violations = report.has_violations();
    let rendered_report = match opts.output_format {
        OutputFormat::Text => report.render_text(opts.verbose),
        OutputFormat::Json => report.render_json()?,
        OutputFormat::Sarif => report.render_sarif()?,
    };

    write_report(workspace, opts.output_path, &rendered_report)?;

    if has_violations {
        return Ok(ExitCode::FAILURE);
    }

    Ok(ExitCode::SUCCESS)
}

struct CollectedViolations {
    project_root: PathBuf,
    files: Vec<PathBuf>,
    violations: Vec<Violation>,
    suppression_report: SuppressionReport,
}

fn collect_violations(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_flag: bool,
    prompt_for_changed_files: bool,
) -> Result<CollectedViolations> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope = scope_override.map(|scope| resolve_path(workspace, scope));

    let files = if git_flag {
        resolve_changed_files(workspace)
    } else {
        let changed_files = git::get_changed_typescript_files();
        if prompt_for_changed_files
            && !changed_files.is_empty()
            && git::prompt_scan_changed_files(&changed_files)
        {
            resolve_changed_files(workspace)
        } else {
            discovery::discover_files(
                &project_config.root,
                scan_scope.as_deref(),
                &project_config.gitignore,
            )?
        }
    };

    let (file_violations, suppression_report) = rules::check_files(&files, &project_config)?;

    let mut dir_violations = rules::check_directories(
        &project_config.root,
        project_config.rules.no_empty_directories,
    );

    let mut name_violations =
        rules::check_duplicate_file_names(&files, project_config.rules.no_duplicate_file_names);

    let mut max_items_violations = rules::check_max_items_per_directory(
        &project_config.root,
        project_config.rules.max_items_per_directory,
    );

    let mut min_items_violations = rules::check_min_items_per_directory(
        &project_config.root,
        project_config.rules.min_items_per_directory,
    );

    let mut depth_violations = rules::check_max_directory_depth(
        &project_config.root,
        project_config.rules.max_directory_depth,
    );

    let mut dump_violations = rules::check_dump_files(&files, project_config.rules.no_dump_files);

    let mut all_violations = file_violations;
    all_violations.append(&mut dir_violations);
    all_violations.append(&mut name_violations);
    all_violations.append(&mut max_items_violations);
    all_violations.append(&mut min_items_violations);
    all_violations.append(&mut depth_violations);
    all_violations.append(&mut dump_violations);

    Ok(CollectedViolations {
        project_root: project_config.root,
        files,
        violations: all_violations,
        suppression_report,
    })
}

fn write_report(
    workspace: &Path,
    output_path: Option<PathBuf>,
    rendered_report: &str,
) -> Result<()> {
    let Some(output_path) = output_path else {
        println!("{rendered_report}");
        return Ok(());
    };

    let resolved_output_path = resolve_path(workspace, output_path);
    if let Some(parent) = resolved_output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&resolved_output_path, rendered_report)
        .with_context(|| format!("failed to write {}", resolved_output_path.display()))?;

    Ok(())
}

fn resolve_changed_files(workspace: &Path) -> Vec<PathBuf> {
    git::get_changed_typescript_files()
        .into_iter()
        .map(|f: PathBuf| {
            if f.is_absolute() {
                f
            } else {
                workspace.join(f)
            }
        })
        .filter(|f: &PathBuf| f.exists())
        .collect()
}

fn resolve_path(workspace: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    workspace.join(path)
}
