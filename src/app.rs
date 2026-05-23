use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, Command};
use crate::{config, discovery, report, rules};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let workspace = env::current_dir()?;

    match cli.command.unwrap_or(Command::Lint) {
        Command::Init => create_config(&workspace),
        Command::Lint => lint_workspace(&workspace, cli.options.root, cli.options.scope),
    }
}

fn create_config(workspace: &Path) -> Result<()> {
    let config_path = config::write_default_config(workspace)?;
    println!("Created {}", config_path.display());

    Ok(())
}

fn lint_workspace(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope = scope_override.map(|scope| resolve_path(workspace, scope));
    let files = discovery::discover_files(&project_config.root, scan_scope.as_deref())?;
    let violations = rules::check_files(
        &files,
        project_config.no_comments,
        project_config.no_logic_in_barrel,
    )?;
    let report = report::Report::new(files, violations);

    println!("{}", report.render_text());

    Ok(())
}

fn resolve_path(workspace: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    workspace.join(path)
}
