use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::{self, ConfigSet};
use crate::discovery;
use crate::git;
use crate::ignore::SuppressionReport;
use crate::import_graph::{self, ImportGraph};
use crate::rules;
use crate::workspace::Workspace;

pub struct AnalysisResult {
    pub project_root: PathBuf,
    pub files: Vec<PathBuf>,
    pub violations: Vec<rules::Violation>,
    pub suppression_report: SuppressionReport,
    pub import_graph: ImportGraph,
    pub workspace: Option<Workspace>,
}

pub struct AnalysisOptions {
    pub root_override: Option<PathBuf>,
    pub scope_override: Option<PathBuf>,
    pub git_flag: bool,
    pub prompt_for_changed_files: bool,
    pub deny_child_configs: bool,
    pub cache_enabled: bool,
    pub clear_cache: bool,
}

pub fn collect(workspace_root: &Path, options: AnalysisOptions) -> Result<AnalysisResult> {
    let root_config =
        config::ProjectConfig::resolve(workspace_root, options.root_override.clone())?;
    let scan_scope = options
        .scope_override
        .map(|scope| resolve_path(&root_config.root, scope));

    let config_set = ConfigSet::resolve(
        workspace_root,
        config::ConfigSetOptions {
            root_override: options.root_override,
            scan_scope: scan_scope.as_deref(),
            deny_child_configs: options.deny_child_configs,
        },
    )?;
    let project_root = config_set.root().root.clone();
    let scan_root = scan_scope.as_deref().unwrap_or(&project_root);

    if options.clear_cache
        && let Err(error) = crate::cache::clear_cache(workspace_root)
    {
        eprintln!("warning: failed to clear cache: {error}");
    }

    let tsconfig = crate::tsconfig::discover_and_parse(workspace_root)?;

    let files = if options.git_flag {
        resolve_changed_files(workspace_root)?
    } else {
        // Always detect changed files for the optional prompt, even without --git
        let changed_files = git::get_changed_typescript_files().unwrap_or_else(|err| {
            eprintln!("warning: could not detect changed files via git: {err}");
            Vec::new()
        });
        if options.prompt_for_changed_files
            && !changed_files.is_empty()
            && git::prompt_scan_changed_files(&changed_files)?
        {
            resolve_changed_files(workspace_root)?
        } else {
            discovery::discover_files(
                &project_root,
                scan_scope.as_deref(),
                &config_set.root().gitignore,
                tsconfig.as_ref(),
            )?
        }
    };

    let config_paths: Vec<PathBuf> = config_set
        .configs()
        .filter_map(|node| node.config_path.clone())
        .collect();
    let tsconfig_path = workspace_root.join("tsconfig.json");
    let tsconfig_path = if tsconfig_path.exists() {
        Some(tsconfig_path)
    } else {
        None
    };

    let cache_state = if options.cache_enabled {
        match crate::cache::prepare_cache(
            workspace_root,
            &files,
            &config_paths,
            tsconfig_path.as_deref(),
        ) {
            Ok(state) => state,
            Err(error) => {
                eprintln!("warning: failed to prepare cache: {error}");
                None
            }
        }
    } else {
        None
    };

    let cached_edges_map = cache_state
        .as_ref()
        .map(|state| state.cached_edges.clone())
        .unwrap_or_default();
    let cached_violations_map = cache_state
        .as_ref()
        .map(|state| state.cached_violations.clone())
        .unwrap_or_default();

    let graph = import_graph::build_import_graph_with_cache(
        &files,
        |file| {
            config_set
                .config_for_file(file)
                .structure
                .tests
                .matches_file(file)
        },
        tsconfig.as_ref(),
        &cached_edges_map,
    )?;

    let workspace = crate::workspace::Workspace::discover(workspace_root).ok();

    let (file_violations, suppression_report, parse_failures) = rules::check_files(
        &files,
        &config_set,
        &graph,
        workspace.as_ref(),
        &cached_violations_map,
    )?;

    if let Some(ref cache_state) = cache_state
        && let Err(error) = crate::cache::finalize_cache(
            workspace_root,
            &files,
            &config_paths,
            tsconfig_path.as_deref(),
            cache_state,
            &graph,
            &file_violations,
            &parse_failures,
        )
    {
        eprintln!("warning: failed to write cache: {error}");
    }

    let mut all_violations = file_violations;

    for (i, node) in config_set.configs().enumerate() {
        let node_root = if node.directory.starts_with(scan_root) {
            &node.directory
        } else {
            scan_root
        };
        let exclude_dirs = config_set.child_directories(i);

        let mut dir_violations =
            rules::check_directory_rules(node_root, &node.config.rules, &exclude_dirs);
        all_violations.append(&mut dir_violations);
    }

    let root_config_ref = config_set.root();
    let mut name_violations = rules::check_duplicate_file_names(
        &files,
        root_config_ref.rules.no_duplicate_file_names.clone(),
    );
    all_violations.append(&mut name_violations);

    let mut dump_violations =
        rules::check_dump_files(&files, root_config_ref.rules.no_dump_files.clone());
    all_violations.append(&mut dump_violations);

    Ok(AnalysisResult {
        project_root,
        files,
        violations: all_violations,
        suppression_report,
        import_graph: graph,
        workspace,
    })
}

pub fn resolve_changed_files(workspace: &Path) -> Result<Vec<PathBuf>> {
    git::get_changed_typescript_files().map(|files| {
        files
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
    })
}

pub fn resolve_path(base: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    base.join(path)
}
