use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use crate::config::{ConfigSet, ConfigSetOptions, FailurePolicy, ProjectConfig};
use crate::diagnostics::{DiagnosticCategory, Diagnostics};
use crate::discovery;
use crate::git::{self, GitSelection};
use crate::ignore::SuppressionReport;
use crate::import_graph::{self, ImportGraph};
use crate::rules::{self, Violation};
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
    pub fail_on: FailurePolicy,
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

pub struct ProjectContext {
    pub project_root: PathBuf,
    pub scan_scope: Option<PathBuf>,
    pub config_set: ConfigSet,
}

impl ProjectContext {
    fn build(workspace_root: &Path, options: &AnalysisOptions) -> Result<Self> {
        let root_config = ProjectConfig::resolve(workspace_root, options.root_override.clone())?;
        let scan_scope = options
            .scope_override
            .as_ref()
            .map(|scope| resolve_path(&root_config.root, scope.to_path_buf()));

        let config_set = ConfigSet::resolve(
            workspace_root,
            ConfigSetOptions {
                root_override: options.root_override.clone(),
                scan_scope: scan_scope.as_deref(),
                deny_child_configs: options.deny_child_configs,
            },
        )?;

        Ok(ProjectContext {
            project_root: config_set.root().root.clone(),
            scan_scope,
            config_set,
        })
    }

    fn scan_root(&self) -> &Path {
        self.scan_scope.as_deref().unwrap_or(&self.project_root)
    }
}

pub struct TsConfig {
    config: Option<crate::tsconfig::TsConfig>,
}

impl TsConfig {
    fn discover(workspace_root: &Path) -> Result<Self> {
        let config = crate::tsconfig::discover_and_parse(workspace_root)?;
        Ok(Self { config })
    }

    fn as_ref(&self) -> Option<&crate::tsconfig::TsConfig> {
        self.config.as_ref()
    }
}

pub struct FileList {
    files: Vec<PathBuf>,
}

impl FileList {
    fn resolve(
        workspace_root: &Path,
        project_root: &Path,
        scan_scope: Option<&PathBuf>,
        config: &ConfigSet,
        tsconfig: &TsConfig,
        options: &AnalysisOptions,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self> {
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
                    project_root,
                    scan_scope.as_ref().map(|p| p.as_path()),
                    &config.root().gitignore,
                    tsconfig.as_ref(),
                )?
            }
        };
        Ok(Self { files })
    }

    fn as_slice(&self) -> &[PathBuf] {
        &self.files
    }
}

pub struct CacheResult {
    state: Option<crate::cache::lifecycle::CacheState>,
    config_paths: Vec<PathBuf>,
}

impl CacheResult {
    fn prepare(
        workspace_root: &Path,
        files: &[PathBuf],
        config_set: &ConfigSet,
        tsconfig_path: Option<&Path>,
        options: &AnalysisOptions,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self> {
        if options.clear_cache
            && let Err(error) = crate::cache::store::clear_cache(workspace_root)
        {
            diagnostics.warn(
                DiagnosticCategory::Cache,
                format!("failed to clear cache: {error}"),
            );
        }

        let config_paths: Vec<PathBuf> = config_set
            .configs()
            .filter_map(|node| node.config_path.clone())
            .collect();

        let state = if options.cache_enabled {
            match crate::cache::lifecycle::prepare_cache(
                workspace_root,
                files,
                &config_paths,
                tsconfig_path,
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

        Ok(Self {
            state,
            config_paths,
        })
    }

    fn cached_violations(&self) -> Arc<HashMap<PathBuf, Vec<Violation>>> {
        self.state
            .as_ref()
            .map(|s| Arc::clone(&s.cached_violations))
            .unwrap_or_else(|| Arc::new(HashMap::new()))
    }

    fn cached_edges(&self) -> HashMap<PathBuf, Vec<crate::import_graph::ImportEdge>> {
        self.state
            .as_ref()
            .map(|s| s.cached_edges.clone())
            .unwrap_or_default()
    }

    fn cached_topology(&self) -> Option<&crate::cache::store::CachedGraph> {
        self.state.as_ref().and_then(|s| s.cached_topology.as_ref())
    }

    fn is_dirty(&self) -> bool {
        self.state.as_ref().map(|s| s.dirty).unwrap_or(true)
    }
}

pub struct ImportGraphResult {
    pub graph: Arc<ImportGraph>,
}

impl ImportGraphResult {
    fn build(
        files: &[PathBuf],
        config_set: &ConfigSet,
        tsconfig: &TsConfig,
        cache: &CacheResult,
        project_root: &Path,
    ) -> Result<Self> {
        let mut graph = import_graph::build_import_graph_with_cache(
            files,
            |file| {
                config_set
                    .config_for_file(file)
                    .structure
                    .tests
                    .matches_file(file)
            },
            tsconfig.as_ref(),
            &cache.cached_edges(),
        )?;

        let cached_graph = cache.cached_topology().filter(|cached| {
            let edge_hash = graph.compute_edge_hash();
            !cache.is_dirty() || cached.edge_hash == edge_hash
        });

        if let Some(cached_graph) = cached_graph {
            graph.set_cycles_by_file(crate::cache::lifecycle::cached_graph_to_cycles(
                cached_graph,
                project_root,
            ));
            graph.set_imported_files(crate::cache::lifecycle::cached_graph_to_imported_files(
                cached_graph,
                project_root,
            ));
        } else {
            crate::cache::lifecycle::ensure_graph_topology(&mut graph);
        }

        Ok(Self {
            graph: Arc::new(graph),
        })
    }
}

pub struct FileLintResult {
    pub violations: Vec<Violation>,
    pub suppression_report: SuppressionReport,
    pub parse_failures: HashMap<PathBuf, String>,
}

impl FileLintResult {
    fn run(
        files: &[PathBuf],
        config_set: &ConfigSet,
        graph: Arc<ImportGraph>,
        workspace: Option<Arc<Workspace>>,
        cache: &CacheResult,
    ) -> Result<Self> {
        let cached_violations_map = cache.cached_violations();

        let (violations, suppression_report, parse_failures) =
            rules::check_files(files, config_set, graph, workspace, cached_violations_map)?;

        Ok(Self {
            violations,
            suppression_report,
            parse_failures,
        })
    }
}

pub struct DirectoryLintResult {
    pub violations: Vec<Violation>,
}

impl DirectoryLintResult {
    fn run(config_set: &ConfigSet, scan_root: &Path) -> Self {
        let mut violations = Vec::new();

        for (i, node) in config_set.configs().enumerate() {
            let node_root = if node.directory.starts_with(scan_root) {
                &node.directory
            } else {
                scan_root
            };
            let exclude_dirs = config_set.child_directories(i);

            let mut dir_violations =
                rules::check_directory_rules(node_root, &node.config.rules, &exclude_dirs);
            violations.append(&mut dir_violations);
        }

        Self { violations }
    }
}

/// A set of discovered files for the project, after discovery and before graph construction.
/// This is a typed pipeline stage that holds all metadata needed for file-set-level checks.
pub struct FileSet {
    pub files: Vec<PathBuf>,
    pub config_set: ConfigSet,
}

impl FileSet {
    pub fn check_duplicate_file_names(&self) -> Vec<Violation> {
        let config = &self.config_set.root().rules.no_duplicate_file_names;
        if !config.severity.is_enabled() {
            return Vec::new();
        }
        crate::rules::no_duplicate_file_names::check_files(&self.files, config)
    }

    pub fn check_dump_files(&self) -> Vec<Violation> {
        let config = &self.config_set.root().rules.no_dump_files;
        if !config.severity.is_enabled() {
            return Vec::new();
        }
        crate::rules::no_dump_files::check_files(&self.files, config)
    }
}

pub fn collect(workspace_root: &Path, options: AnalysisOptions) -> Result<AnalysisResult> {
    let mut diagnostics = Diagnostics::new();

    let context = ProjectContext::build(workspace_root, &options)?;
    let project_root = context.project_root.clone();
    let scan_root = context.scan_root().to_path_buf();

    let tsconfig = TsConfig::discover(workspace_root)?;

    let file_list = FileList::resolve(
        workspace_root,
        &project_root,
        context.scan_scope.as_ref(),
        &context.config_set,
        &tsconfig,
        &options,
        &mut diagnostics,
    )?;

    let tsconfig_path = workspace_root.join("tsconfig.json");
    let tsconfig_path = if tsconfig_path.exists() {
        Some(tsconfig_path)
    } else {
        None
    };
    let cache = CacheResult::prepare(
        workspace_root,
        file_list.as_slice(),
        &context.config_set,
        tsconfig_path.as_deref(),
        &options,
        &mut diagnostics,
    )?;

    let graph_result = ImportGraphResult::build(
        file_list.as_slice(),
        &context.config_set,
        &tsconfig,
        &cache,
        &project_root,
    )?;

    let workspace = match Workspace::discover(workspace_root) {
        Ok(workspace) => Some(Arc::new(workspace)),
        Err(error) => {
            diagnostics.warn(
                DiagnosticCategory::Workspace,
                format!("failed to discover workspace: {error}"),
            );
            None
        }
    };

    let file_lint = FileLintResult::run(
        file_list.as_slice(),
        &context.config_set,
        graph_result.graph.clone(),
        workspace.clone(),
        &cache,
    )?;

    let dir_lint = DirectoryLintResult::run(&context.config_set, &scan_root);

    let file_set = FileSet {
        files: file_list.files.clone(),
        config_set: context.config_set,
    };
    let mut all_violations = file_lint.violations;
    all_violations.extend(dir_lint.violations);
    all_violations.extend(file_set.check_duplicate_file_names());
    all_violations.extend(file_set.check_dump_files());

    if let Some(ref state) = cache.state
        && let Err(error) = crate::cache::lifecycle::finalize_cache(
            workspace_root,
            file_list.as_slice(),
            &cache.config_paths,
            tsconfig_path.as_deref(),
            state,
            graph_result.graph.as_ref(),
            &all_violations,
            &file_lint.parse_failures,
        )
    {
        diagnostics.warn(
            DiagnosticCategory::Cache,
            format!("failed to write cache: {error}"),
        );
    }

    Ok(AnalysisResult {
        project_root,
        history_enabled: file_set.config_set.root().history,
        files: file_list.files,
        violations: all_violations,
        suppression_report: file_lint.suppression_report,
        import_graph: graph_result.graph,
        workspace,
        diagnostics: diagnostics.into_entries(),
        fail_on: file_set.config_set.root().fail_on.clone(),
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
