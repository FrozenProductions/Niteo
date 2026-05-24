use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::{Path, PathBuf};

use crate::cli::{Cli, Command};
use crate::{config, discovery, git, report, rules};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let workspace = env::current_dir()?;

    match cli.command.unwrap_or(Command::Lint) {
        Command::Init => create_config(&workspace),
        Command::Lint => lint_workspace(
            &workspace,
            cli.options.root,
            cli.options.scope,
            cli.options.verbose,
            cli.options.git,
        ),
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
    verbose: bool,
    git_flag: bool,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope = scope_override.map(|scope| resolve_path(workspace, scope));

    let files = if git_flag {
        resolve_changed_files(workspace)
    } else {
        let changed_files = git::get_changed_typescript_files();
        if !changed_files.is_empty() && git::prompt_scan_changed_files(&changed_files) {
            resolve_changed_files(workspace)
        } else {
            discovery::discover_files(
                &project_config.root,
                scan_scope.as_deref(),
                &project_config.gitignore,
            )?
        }
    };

    let violations = rules::check_files(
        &files,
        project_config.no_comments,
        project_config.no_logic_in_barrel,
        project_config.no_default_export,
        project_config.no_inline_types,
        project_config.max_file_exports,
        project_config.no_upward_import,
        project_config.no_large_file,
        project_config.no_enums,
        project_config.no_barrel_files,
        project_config.no_console,
        project_config.no_debugger,
        project_config.no_eval,
        project_config.no_logic_in_domain,
    )?;

    let mut dir_violations =
        rules::check_directories(&project_config.root, project_config.no_empty_directories);

    let mut name_violations =
        rules::check_duplicate_file_names(&files, project_config.no_duplicate_file_names);

    let mut all_violations = violations;
    all_violations.append(&mut dir_violations);
    all_violations.append(&mut name_violations);

    let report = report::Report::new(files, all_violations);

    println!("{}", report.render_text(verbose));

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
