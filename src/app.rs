use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::{BaselineCommand, Cli, Command, ConfigCommand};
use crate::commands;
use crate::watch;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let workspace = std::env::current_dir()?;
    let baseline_path =
        crate::config::resolve_baseline_path(&workspace, cli.options.baseline.clone())?;
    let git_selection = cli.options.git_selection();

    let exit_code = match cli.command.unwrap_or(Command::Lint { fix: false }) {
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
        Command::Stats => {
            commands::stats::show(
                &workspace,
                cli.options.root,
                cli.options.scope,
                git_selection.clone(),
                cli.options.format,
                cli.options.output,
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
        Command::Lint { fix } => {
            if fix && cli.options.watch {
                anyhow::bail!("--fix cannot be used with --watch");
            }

            let cache_enabled = cli.options.cache && !cli.options.no_cache;

            let opts = commands::lint::LintOptions {
                verbose: cli.options.verbose,
                git_selection: git_selection.clone(),
                output_format: cli.options.format,
                output_path: cli.options.output,
                baseline_path: baseline_path.clone(),
                report_suppressions: cli.options.report_suppressions,
                fail_on: cli.options.fail_on,
                deny_child_configs: cli.options.deny_child_configs,
                cache_enabled,
                clear_cache: cli.options.clear_cache,
            };

            let exit_code = if cli.options.watch {
                let watch_root =
                    commands::lint::resolve_watch_root(&workspace, cli.options.root.as_deref())?;
                let workspace_clone = workspace.clone();
                let root = cli.options.root.clone();
                let scope = cli.options.scope.clone();

                watch::run(&watch_root, cli.options.watch_debounce_ms, move || {
                    commands::lint::lint_workspace(
                        &workspace_clone,
                        root.clone(),
                        scope.clone(),
                        opts.clone(),
                        false,
                    )
                })?;
                ExitCode::SUCCESS
            } else {
                commands::lint::lint_workspace(
                    &workspace,
                    cli.options.root.clone(),
                    cli.options.scope.clone(),
                    opts,
                    true,
                )?
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
    let preset_name = preset.map(|name| match name {
        crate::cli::PresetName::Balanced => crate::config::presets::PresetName::Balanced,
        crate::cli::PresetName::Strict => crate::config::presets::PresetName::Strict,
        crate::cli::PresetName::Migration => crate::config::presets::PresetName::Migration,
        crate::cli::PresetName::React => crate::config::presets::PresetName::React,
        crate::cli::PresetName::Library => crate::config::presets::PresetName::Library,
        crate::cli::PresetName::NoBarrels => crate::config::presets::PresetName::NoBarrels,
    });

    match preset_name {
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
