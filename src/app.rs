use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use crate::cli::{BaselineCommand, Cli, Command, ConfigCommand};
use crate::commands;
use crate::config::{FailureThreshold, RuleCategory};
use crate::watch;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let workspace = std::env::current_dir()?;
    let baseline_path =
        crate::config::resolve_baseline_path(&workspace, cli.options.baseline.clone())?;
    let git_selection = cli.options.git_selection();

    let exit_code = match cli.command.unwrap_or(Command::Lint {
        fix: false,
        history: false,
    }) {
        Command::Init { preset } => {
            create_config(&workspace, preset)?;
            ExitCode::SUCCESS
        }
        Command::Config { command } => match command {
            ConfigCommand::Check => commands::config::check(&workspace)?,
            ConfigCommand::Print => commands::config::print(&workspace)?,
        },
        Command::Baseline { command } => match command {
            BaselineCommand::Create => {
                commands::baseline::create(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    git_selection.clone(),
                    baseline_path,
                    cli.options.report_suppressions,
                    cli.options.deny_child_configs,
                )?;
                ExitCode::SUCCESS
            }
            BaselineCommand::Prune => {
                commands::baseline::prune(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    git_selection.clone(),
                    baseline_path,
                    cli.options.deny_child_configs,
                )?;
                ExitCode::SUCCESS
            }
        },
        Command::Rules { preset } => {
            if let Some(preset_name) = preset {
                commands::rules::list_with_preset(
                    &workspace,
                    preset_name,
                    cli.options.format,
                    cli.options.output,
                )?;
            } else {
                commands::rules::list(
                    &workspace,
                    cli.options.root,
                    cli.options.format,
                    cli.options.output,
                )?;
            }
            ExitCode::SUCCESS
        }
        Command::Explain { rule } => {
            commands::rules::explain(
                &workspace,
                cli.options.root,
                cli.options.format,
                cli.options.output,
                &rule,
            )?;
            ExitCode::SUCCESS
        }
        Command::Stats { history } => {
            commands::stats::show(
                &workspace,
                cli.options.root,
                cli.options.scope,
                git_selection.clone(),
                cli.options.format,
                cli.options.output,
                history,
            )?;
            ExitCode::SUCCESS
        }
        Command::Graph => {
            commands::graph::show(
                &workspace,
                cli.options.root,
                cli.options.scope,
                git_selection.clone(),
                cli.options.format,
                cli.options.output,
            )?;
            ExitCode::SUCCESS
        }
        Command::Lint { fix, history } => {
            if fix && cli.options.watch {
                anyhow::bail!("--fix cannot be used with --watch");
            }

            let cache_enabled = cli.options.cache && !cli.options.no_cache;

            let (fail_on_rules, fail_on_categories) =
                parse_fail_on_overrides(&cli.options.fail_on_rule, &cli.options.fail_on_category)?;

            let opts = commands::lint::LintOptions {
                verbose: cli.options.verbose,
                git_selection: git_selection.clone(),
                output_format: cli.options.format,
                output_path: cli.options.output,
                baseline_path: baseline_path.clone(),
                report_suppressions: cli.options.report_suppressions,
                fail_on: cli.options.fail_on.map(FailureThreshold::from),
                fail_on_rules,
                fail_on_categories,
                deny_child_configs: cli.options.deny_child_configs,
                cache_enabled,
                clear_cache: cli.options.clear_cache,
                force_history: history,
            };

            let exit_code = if cli.options.watch {
                let watch_root =
                    commands::lint::resolve_watch_root(&workspace, cli.options.root.as_deref())?;
                let workspace_clone = workspace.clone();
                let root = cli.options.root.clone();
                let scope = cli.options.scope.clone();
                let mut previous_result: Option<Arc<crate::analysis::AnalysisResult>> = None;

                watch::run(
                    &watch_root,
                    cli.options.watch_debounce_ms,
                    |changed_files| {
                        if let Some(changed) = changed_files
                            && let Some(ref previous) = previous_result
                        {
                            let (code, result) = commands::lint::lint_workspace_incremental(
                                &workspace_clone,
                                previous,
                                changed,
                                opts.clone(),
                            )?;
                            previous_result = Some(result);
                            return Ok(code);
                        }

                        let (code, result) = commands::lint::lint_workspace_with_result(
                            &workspace_clone,
                            root.clone(),
                            scope.clone(),
                            opts.clone(),
                            false,
                        )?;
                        previous_result = Some(result);
                        Ok(code)
                    },
                )?;
                ExitCode::SUCCESS
            } else {
                let (code, _result) = commands::lint::lint_workspace_with_result(
                    &workspace,
                    cli.options.root.clone(),
                    cli.options.scope.clone(),
                    opts,
                    true,
                )?;
                code
            };

            if fix {
                commands::fix::fix_workspace(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    commands::fix::FixOptions {
                        dry_run: false,
                        git_selection: git_selection.clone(),
                        baseline_path: baseline_path.clone(),
                        deny_child_configs: cli.options.deny_child_configs,
                    },
                )?;
            }

            exit_code
        }
        Command::Fix { dry_run } => {
            commands::fix::fix_workspace(
                &workspace,
                cli.options.root,
                cli.options.scope,
                commands::fix::FixOptions {
                    dry_run,
                    git_selection,
                    baseline_path,
                    deny_child_configs: cli.options.deny_child_configs,
                },
            )?;
            ExitCode::SUCCESS
        }
    };

    Ok(exit_code)
}

fn create_config(workspace: &Path, preset: Option<crate::cli::PresetName>) -> Result<()> {
    match preset {
        Some(preset) => {
            let source = crate::config::presets::default_config_for_preset(preset);
            let config_path = workspace.join(crate::config::defaults::CONFIG_FILE_NAME);
            if config_path.exists() {
                anyhow::bail!("{} already exists", config_path.display());
            }
            std::fs::write(&config_path, source)?;
            println!("Created {}", config_path.display());
        }
        None => {
            let config_path = crate::config::write_default_config(workspace)?;
            println!("Created {}", config_path.display());
        }
    }
    Ok(())
}

fn parse_fail_on_overrides(
    rule_overrides: &[crate::cli::FailOnOverride],
    category_overrides: &[crate::cli::FailOnOverride],
) -> Result<(
    HashMap<String, FailureThreshold>,
    HashMap<RuleCategory, FailureThreshold>,
)> {
    let mut fail_on_rules = HashMap::new();
    for override_value in rule_overrides {
        fail_on_rules.insert(
            override_value.target.clone(),
            FailureThreshold::from(override_value.threshold),
        );
    }

    let mut fail_on_categories = HashMap::new();
    for override_value in category_overrides {
        let category = override_value
            .target
            .parse::<RuleCategory>()
            .map_err(anyhow::Error::msg)?;
        fail_on_categories.insert(category, FailureThreshold::from(override_value.threshold));
    }

    Ok((fail_on_rules, fail_on_categories))
}
