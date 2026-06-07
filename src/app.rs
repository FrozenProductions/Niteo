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

    let exit_code = match cli.command.unwrap_or(Command::Lint) {
        Command::Init { preset } => {
            create_config(&workspace, preset)?;
            ExitCode::SUCCESS
        }
        Command::Config { command } => match command {
            ConfigCommand::Check => {
                let source = read_config_source(&workspace);
                let report = crate::config::validation::validate_config_source(&source);

                println!("{}", report.render_text());

                if report.has_errors() {
                    return Ok(ExitCode::FAILURE);
                }
                ExitCode::SUCCESS
            }
            ConfigCommand::Print => {
                let source = read_config_source(&workspace);
                println!("{source}");
                ExitCode::SUCCESS
            }
        },
        Command::Baseline { command } => match command {
            BaselineCommand::Create => {
                commands::baseline::create(
                    &workspace,
                    cli.options.root,
                    cli.options.scope,
                    cli.options.git,
                    cli.options.baseline,
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
                    cli.options.git,
                    cli.options.baseline,
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
                cli.options.git,
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
                cli.options.git,
                cli.options.format,
                cli.options.output,
            )?;
            ExitCode::SUCCESS
        }
        Command::Lint => {
            let cache_enabled = cli.options.cache && !cli.options.no_cache;

            let opts = commands::lint::LintOptions {
                verbose: cli.options.verbose,
                git_flag: cli.options.git,
                output_format: cli.options.format,
                output_path: cli.options.output,
                baseline_path: cli.options.baseline,
                report_suppressions: cli.options.report_suppressions,
                fail_on: cli.options.fail_on,
                deny_child_configs: cli.options.deny_child_configs,
                cache_enabled,
                clear_cache: cli.options.clear_cache,
            };

            if cli.options.watch {
                let watch_root =
                    commands::lint::resolve_watch_root(&workspace, cli.options.root.as_deref())?;
                let workspace_clone = workspace.clone();
                let root = cli.options.root.clone();
                let scope = cli.options.scope.clone();

                watch::run(&watch_root, move || {
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
                    cli.options.root,
                    cli.options.scope,
                    opts,
                    true,
                )?
            }
        }
    };

    Ok(exit_code)
}

fn read_config_source(workspace: &Path) -> String {
    let config_path = workspace.join(crate::config::defaults::CONFIG_FILE_NAME);
    if config_path.exists() {
        std::fs::read_to_string(&config_path).unwrap_or_else(|_| String::new())
    } else {
        crate::config::defaults::DEFAULT_CONFIG_SOURCE.to_owned()
    }
}

fn create_config(workspace: &Path, preset: Option<crate::cli::PresetName>) -> Result<()> {
    let preset_name = preset.map(|name| match name {
        crate::cli::PresetName::Balanced => "balanced",
        crate::cli::PresetName::Strict => "strict",
        crate::cli::PresetName::Migration => "migration",
        crate::cli::PresetName::React => "react",
        crate::cli::PresetName::Library => "library",
        crate::cli::PresetName::NoBarrels => "no-barrels",
    });

    match preset_name {
        Some(name) => {
            let preset = crate::config::presets::PresetName::from_str(name).unwrap();
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
