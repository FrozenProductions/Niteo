use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::{self, ConfigSet};
use crate::diagnostics::{DiagnosticCategory, Diagnostics};
use crate::discovery;
use crate::git::{self, GitSelection};
use crate::ignore::SuppressionReport;
use crate::import_graph::{self, ImportGraph};
use crate::rules;
use crate::workspace::Workspace;

pub struct AnalysisResult {
    pub project_root: PathBuf,
    pub history_enabled: bool,
    pub files: Vec<PathBuf>,
    pub violations: Vec<rules::Violation>,
    pub suppression_report: SuppressionReport,
    pub import_graph: Arc<ImportGraph>,
    pub workspace: Option<Arc<Workspace>>,
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
}

pub struct AnalysisOptions {
    pub root_override: Option<PathBuf>,
    pub scope_override: Option<PathBuf>,
    pub git_selection: Option<GitSelection>,
    pub prompt_for_changed_files: bool,
    pub deny_child_configs: bool,
    pub cache_enabled: bool,
    pub clear_cache: bool,
}

/// A set of discovered files for the project, after discovery and before graph construction.
/// This is a typed pipeline stage that holds all metadata needed for file-set-level checks.
pub struct FileSet {
    pub files: Vec<PathBuf>,
    pub config_set: ConfigSet,
}

impl FileSet {
    pub fn check_duplicate_file_names(&self) -> Vec<rules::Violation> {
        let config = &self.config_set.root().rules.no_duplicate_file_names;
        if !config.severity.is_enabled() {
            return Vec::new();
        }
        crate::rules::no_duplicate_file_names::check_files(&self.files, config)
    }

    pub fn check_dump_files(&self) -> Vec<rules::Violation> {
        let config = &self.config_set.root().rules.no_dump_files;
        if !config.severity.is_enabled() {
            return Vec::new();
        }
        crate::rules::no_dump_files::check_files(&self.files, config)
    }
}

pub fn collect(workspace_root: &Path, options: AnalysisOptions) -> Result<AnalysisResult> {
    let mut diagnostics = Diagnostics::new();
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
        && let Err(error) = crate::cache::store::clear_cache(workspace_root)
    {
        diagnostics.warn(
            DiagnosticCategory::Cache,
            format!("failed to clear cache: {error}"),
        );
    }

    let tsconfig = crate::tsconfig::discover_and_parse(workspace_root)?;

    let files = if let Some(selection) = options.git_selection.as_ref() {
        resolve_changed_files(workspace_root, selection)?
    } else {
        let prompt_changed = if options.prompt_for_changed_files {
            match git::get_changed_typescript_files(&GitSelection::WorkingTree) {
                Ok(changed) => changed,
                Err(err) => {
                    diagnostics.warn(
                        DiagnosticCategory::Git,
                        format!("could not detect changed files via git: {err}"),
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        if !prompt_changed.is_empty() && git::prompt_scan_changed_files(&prompt_changed)? {
            resolve_changed_files(workspace_root, &GitSelection::WorkingTree)?
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
        match crate::cache::lifecycle::prepare_cache(
            workspace_root,
            &files,
            &config_paths,
            tsconfig_path.as_deref(),
        ) {
            Ok(state) => state,
            Err(error) => {
                diagnostics.warn(
                    DiagnosticCategory::Cache,
                    format!("failed to prepare cache: {error}"),
                );
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

    let mut graph = import_graph::build_import_graph_with_cache(
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

    let cached_graph = cache_state.as_ref().and_then(|state| {
        state.cached_topology.as_ref().filter(|cached_graph| {
            !state.dirty || cached_graph.edge_hash == graph.compute_edge_hash()
        })
    });

    if let Some(cached_graph) = cached_graph {
        graph.set_cycles_by_file(crate::cache::lifecycle::cached_graph_to_cycles(
            cached_graph,
            &project_root,
        ));
        graph.set_imported_files(crate::cache::lifecycle::cached_graph_to_imported_files(
            cached_graph,
            &project_root,
        ));
    } else {
        crate::cache::lifecycle::ensure_graph_topology(&mut graph);
    }

    let graph = Arc::new(graph);

    let workspace = match crate::workspace::Workspace::discover(workspace_root) {
        Ok(workspace) => Some(Arc::new(workspace)),
        Err(error) => {
            diagnostics.warn(
                DiagnosticCategory::Workspace,
                format!("failed to discover workspace: {error}"),
            );
            None
        }
    };

    let (file_violations, suppression_report, parse_failures) = rules::check_files(
        &files,
        &config_set,
        graph.clone(),
        workspace.clone(),
        &cached_violations_map,
    )?;

    if let Some(ref cache_state) = cache_state
        && let Err(error) = crate::cache::lifecycle::finalize_cache(
            workspace_root,
            &files,
            &config_paths,
            tsconfig_path.as_deref(),
            cache_state,
            graph.as_ref(),
            &file_violations,
            &parse_failures,
        )
    {
        diagnostics.warn(
            DiagnosticCategory::Cache,
            format!("failed to write cache: {error}"),
        );
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

    let history = config_set.root().history;

    let file_set = FileSet {
        files: files.clone(),
        config_set,
    };

    all_violations.extend(file_set.check_duplicate_file_names());
    all_violations.extend(file_set.check_dump_files());

    Ok(AnalysisResult {
        project_root,
        history_enabled: history,
        files,
        violations: all_violations,
        suppression_report,
        import_graph: graph,
        workspace,
        diagnostics: diagnostics.into_entries(),
    })
}

pub fn resolve_changed_files(workspace: &Path, selection: &GitSelection) -> Result<Vec<PathBuf>> {
    git::get_changed_typescript_files(selection).map(|files| {
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
