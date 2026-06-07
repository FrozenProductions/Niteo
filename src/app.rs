use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::{BaselineCommand, Cli, Command};
use crate::commands;
use crate::watch;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let workspace = std::env::current_dir()?;

    let exit_code = match cli.command.unwrap_or(Command::Lint) {
        Command::Init => {
            create_config(&workspace)?;
            ExitCode::SUCCESS
        }
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
        Command::Rules => {
            commands::rules::list(
                &workspace,
                cli.options.root,
                cli.options.format,
                cli.options.output,
            )?;
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
            let opts = commands::lint::LintOptions {
                verbose: cli.options.verbose,
                git_flag: cli.options.git,
                output_format: cli.options.format,
                output_path: cli.options.output,
                baseline_path: cli.options.baseline,
                report_suppressions: cli.options.report_suppressions,
                fail_on: cli.options.fail_on,
                deny_child_configs: cli.options.deny_child_configs,
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

fn create_config(workspace: &Path) -> Result<()> {
    let config_path = crate::config::write_default_config(workspace)?;
    println!("Created {}", config_path.display());
    Ok(())
}
