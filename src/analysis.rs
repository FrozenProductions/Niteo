use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};

use crate::config::{ConfigSet, ConfigSetOptions, FailurePolicy, ProjectConfig};
use crate::diagnostics::{DiagnosticCategory, Diagnostics};
use crate::discovery;
use crate::git::{self, GitSelection};
use crate::ignore::SuppressionReport;
use crate::import_graph::{self, ImportGraph};
use crate::rules::{self, Violation};
use crate::syntax;
use crate::workspace::Workspace;

#[derive(Clone)]
pub struct AnalysisResult {
    pub project_root: PathBuf,
    pub scan_scope: Option<PathBuf>,
    pub history_enabled: bool,
    pub files: Vec<PathBuf>,
    pub violations: Vec<rules::Violation>,
    pub directory_violations: Vec<rules::Violation>,
    pub suppression_report: SuppressionReport,
    pub import_graph: Arc<ImportGraph>,
    pub workspace: Option<Arc<Workspace>>,
    pub config_set: ConfigSet,
    pub diagnostics: Vec<crate::diagnostics::Diagnostic>,
    pub fail_on: FailurePolicy,
    pub parse_failures: HashMap<PathBuf, String>,
}

pub struct AnalysisOptions {
    pub root_override: Option<PathBuf>,
    pub scope_override: Option<PathBuf>,
    pub git_selection: Option<GitSelection>,
    pub prompt_for_changed_files: bool,
    pub deny_child_configs: bool,
    pub cache_enabled: bool,
    pub clear_cache: bool,
    pub verbose: u8,
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

    fn empty() -> Self {
        Self {
            state: None,
            config_paths: Vec::new(),
        }
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
        verbose: u8,
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
            verbose,
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
        verbose: u8,
    ) -> Result<Self> {
        let cached_violations_map = cache.cached_violations();

        let (violations, suppression_report, parse_failures) = rules::check_files(
            files,
            config_set,
            graph,
            workspace,
            cached_violations_map,
            verbose,
        )?;

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
        let inventory = crate::directory_inventory::collect_directory_inventory(scan_root, &[]);

        for (i, node) in config_set.configs().enumerate() {
            let node_root = if node.directory.starts_with(scan_root) {
                &node.directory
            } else {
                scan_root
            };
            let exclude_dirs = config_set.child_directories(i);

            let mut dir_violations = rules::check_directory_rules(
                &inventory,
                node_root,
                &node.config.rules,
                &exclude_dirs,
            );
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
        options.verbose,
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
        options.verbose,
    )?;

    let dir_lint = DirectoryLintResult::run(&context.config_set, &scan_root);

    let file_set = FileSet {
        files: file_list.files.clone(),
        config_set: context.config_set,
    };
    let mut file_violations = file_lint.violations;
    file_violations.extend(file_set.check_duplicate_file_names());
    file_violations.extend(file_set.check_dump_files());

    let directory_violations = dir_lint.violations;
    let all_violations_for_cache: Vec<rules::Violation> = file_violations
        .iter()
        .cloned()
        .chain(directory_violations.iter().cloned())
        .collect();

    if let Some(ref state) = cache.state
        && let Err(error) = crate::cache::lifecycle::finalize_cache(
            workspace_root,
            file_list.as_slice(),
            &cache.config_paths,
            tsconfig_path.as_deref(),
            state,
            graph_result.graph.as_ref(),
            &all_violations_for_cache,
            &file_lint.parse_failures,
        )
    {
        diagnostics.warn(
            DiagnosticCategory::Cache,
            format!("failed to write cache: {error}"),
        );
    }

    let config_set = file_set.config_set;
    let fail_on = config_set.root().fail_on.clone();
    let history_enabled = config_set.root().history;

    Ok(AnalysisResult {
        project_root,
        scan_scope: context.scan_scope,
        history_enabled,
        files: file_list.files,
        violations: file_violations,
        directory_violations,
        suppression_report: file_lint.suppression_report,
        import_graph: graph_result.graph,
        workspace,
        config_set,
        diagnostics: diagnostics.into_entries(),
        fail_on,
        parse_failures: file_lint.parse_failures,
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

pub fn collect_incremental(
    workspace_root: &Path,
    previous: &AnalysisResult,
    changed_paths: &[PathBuf],
) -> Result<AnalysisResult> {
    if changed_paths.is_empty() {
        return Ok(previous.clone());
    }

    let tsconfig = TsConfig::discover(workspace_root)?;

    let previous_file_set: HashSet<PathBuf> = previous.files.iter().cloned().collect();
    let mut files: Vec<PathBuf> = previous.files.clone();
    let mut file_set: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut changed_files: HashSet<PathBuf> = HashSet::new();
    let mut removed_files: HashSet<PathBuf> = HashSet::new();

    for path in changed_paths {
        if is_config_file(path) {
            bail!("config file changed; full re-lint required");
        }

        if path.is_dir() {
            if !path.exists() {
                let to_remove: Vec<PathBuf> = files
                    .iter()
                    .filter(|file| file.starts_with(path))
                    .cloned()
                    .collect();
                for file in to_remove {
                    file_set.remove(&file);
                    removed_files.insert(file);
                }
                files.retain(|file| !file.starts_with(path));
            }
            continue;
        }

        if !syntax::is_typescript_file(path) {
            continue;
        }

        if path.exists() {
            let in_scope = previous
                .scan_scope
                .as_ref()
                .map(|scope| path.starts_with(scope))
                .unwrap_or(true);
            let included = tsconfig
                .as_ref()
                .is_none_or(|config| config.is_included(path));
            if !in_scope || !included {
                continue;
            }
            if file_set.insert(path.clone()) {
                files.push(path.clone());
            }
            changed_files.insert(path.clone());
        } else if file_set.remove(path) {
            files.retain(|file| file != path);
            removed_files.insert(path.clone());
        }
    }

    if changed_files.is_empty() && removed_files.is_empty() {
        return Ok(previous.clone());
    }

    let mut cached_edges = previous.import_graph.edges_by_file();
    for path in changed_files.iter().chain(removed_files.iter()) {
        cached_edges.remove(path);
    }

    let is_test_file = |file: &Path| {
        previous
            .config_set
            .config_for_file(file)
            .structure
            .tests
            .matches_file(file)
    };

    let mut graph = import_graph::build_import_graph_with_cache(
        &files,
        is_test_file,
        tsconfig.as_ref(),
        &cached_edges,
        0,
    )?;
    graph.set_cycles_by_file(crate::import_graph::topology::compute_cycles(&graph));
    graph.set_imported_files(crate::import_graph::topology::compute_imported_files(
        &graph,
    ));
    let graph = Arc::new(graph);

    let files_to_lint = affected_files(
        &previous.import_graph,
        &graph,
        &changed_files,
        &removed_files,
    );

    let lint_file_list: Vec<PathBuf> = files_to_lint.iter().cloned().collect();
    let file_lint = FileLintResult::run(
        &lint_file_list,
        &previous.config_set,
        graph.clone(),
        previous.workspace.clone(),
        &CacheResult::empty(),
        0,
    )?;

    let mut previous_violations: HashMap<PathBuf, Vec<rules::Violation>> = HashMap::new();
    for violation in &previous.violations {
        previous_violations
            .entry(violation.file.clone())
            .or_default()
            .push(violation.clone());
    }

    let mut new_violations: HashMap<PathBuf, Vec<rules::Violation>> = HashMap::new();
    for violation in &file_lint.violations {
        new_violations
            .entry(violation.file.clone())
            .or_default()
            .push(violation.clone());
    }

    let mut file_violations: Vec<rules::Violation> = Vec::with_capacity(previous.violations.len());
    for file in &files {
        if files_to_lint.contains(file) {
            if let Some(violations) = new_violations.get(file) {
                file_violations.extend(violations.iter().cloned());
            }
        } else if let Some(violations) = previous_violations.get(file) {
            file_violations.extend(violations.iter().cloned());
        }
    }

    let file_set_check = FileSet {
        files: files.clone(),
        config_set: previous.config_set.clone(),
    };
    file_violations.extend(file_set_check.check_duplicate_file_names());
    file_violations.extend(file_set_check.check_dump_files());

    let has_structural_change = !removed_files.is_empty()
        || changed_files
            .iter()
            .any(|file| !previous_file_set.contains(file));
    let directory_violations = if has_structural_change {
        let scan_root = previous
            .scan_scope
            .as_deref()
            .unwrap_or(&previous.project_root);
        DirectoryLintResult::run(&previous.config_set, scan_root).violations
    } else {
        previous.directory_violations.clone()
    };

    let suppression_report = merged_suppression_report(
        &previous.suppression_report,
        &file_lint.suppression_report,
        &files,
        &files_to_lint,
    );

    let mut parse_failures = previous.parse_failures.clone();
    for file in removed_files.iter() {
        parse_failures.remove(file);
    }
    for (file, message) in &file_lint.parse_failures {
        if files_to_lint.contains(file) {
            parse_failures.insert(file.clone(), message.clone());
        }
    }

    Ok(AnalysisResult {
        project_root: previous.project_root.clone(),
        scan_scope: previous.scan_scope.clone(),
        history_enabled: previous.history_enabled,
        files,
        violations: file_violations,
        directory_violations,
        suppression_report,
        import_graph: graph,
        workspace: previous.workspace.clone(),
        config_set: previous.config_set.clone(),
        diagnostics: previous.diagnostics.clone(),
        fail_on: previous.fail_on.clone(),
        parse_failures,
    })
}

fn is_config_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "niteo.toml" || name == ".niteo.toml" || name == "tsconfig.json"
        })
}

fn affected_files(
    previous: &ImportGraph,
    current: &ImportGraph,
    changed: &HashSet<PathBuf>,
    removed: &HashSet<PathBuf>,
) -> HashSet<PathBuf> {
    let mut forward: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    let mut reverse: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

    for graph in [previous, current] {
        for edge in graph.edges() {
            if let Some(target) = &edge.resolved_target {
                forward
                    .entry(edge.source_file.clone())
                    .or_default()
                    .push(target.clone());
                reverse
                    .entry(target.clone())
                    .or_default()
                    .push(edge.source_file.clone());
            }
        }
    }

    let mut affected = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    for seed in changed.iter().chain(removed.iter()) {
        if affected.insert(seed.clone()) {
            stack.push(seed.clone());
        }
    }

    while let Some(file) = stack.pop() {
        for next in forward.get(&file).into_iter().flatten() {
            if affected.insert(next.clone()) {
                stack.push(next.clone());
            }
        }
        for prev in reverse.get(&file).into_iter().flatten() {
            if affected.insert(prev.clone()) {
                stack.push(prev.clone());
            }
        }
    }

    affected
}

fn merged_suppression_report(
    previous: &SuppressionReport,
    new: &SuppressionReport,
    files: &[PathBuf],
    linted: &HashSet<PathBuf>,
) -> SuppressionReport {
    let previous_by_file: HashMap<PathBuf, crate::ignore::FileSuppressionInfo> = previous
        .files
        .iter()
        .map(|info| (info.file.clone(), info.clone()))
        .collect();
    let new_by_file: HashMap<PathBuf, crate::ignore::FileSuppressionInfo> = new
        .files
        .iter()
        .map(|info| (info.file.clone(), info.clone()))
        .collect();

    let mut merged = Vec::new();
    for file in files {
        if linted.contains(file) {
            if let Some(info) = new_by_file.get(file) {
                merged.push(info.clone());
            }
        } else if let Some(info) = previous_by_file.get(file) {
            merged.push(info.clone());
        }
    }
    SuppressionReport { files: merged }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis_options() -> AnalysisOptions {
        AnalysisOptions {
            root_override: None,
            scope_override: None,
            git_selection: None,
            prompt_for_changed_files: false,
            deny_child_configs: false,
            cache_enabled: false,
            clear_cache: false,
            verbose: 0,
        }
    }

    #[test]
    fn incremental_detects_violation_in_changed_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(root.join("src/a.ts"), "export const a = 1;\n")?;
        std::fs::write(root.join("src/b.ts"), "import { a } from './a';\n")?;

        let initial = collect(root, analysis_options())?;
        assert!(
            !initial
                .violations
                .iter()
                .any(|v| v.file == root.join("src/a.ts") && v.rule == rules::NO_CONSOLE_RULE_ID)
        );

        std::fs::write(root.join("src/a.ts"), "console.log(1);\n")?;
        let changed = vec![root.join("src/a.ts")];
        let incremental = collect_incremental(root, &initial, &changed)?;
        assert!(
            incremental
                .violations
                .iter()
                .any(|v| v.file == root.join("src/a.ts") && v.rule == rules::NO_CONSOLE_RULE_ID)
        );

        Ok(())
    }

    #[test]
    fn incremental_preserves_violations_for_unchanged_files() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(root.join("src/a.ts"), "console.log(1);\n")?;
        std::fs::write(root.join("src/b.ts"), "export const b = 1;\n")?;

        let initial = collect(root, analysis_options())?;
        let a_violations_before: Vec<_> = initial
            .violations
            .iter()
            .filter(|v| v.file == root.join("src/a.ts"))
            .cloned()
            .collect();
        assert!(!a_violations_before.is_empty());

        std::fs::write(root.join("src/b.ts"), "console.log(2);\n")?;
        let changed = vec![root.join("src/b.ts")];
        let incremental = collect_incremental(root, &initial, &changed)?;
        let a_violations_after: Vec<_> = incremental
            .violations
            .iter()
            .filter(|v| v.file == root.join("src/a.ts"))
            .cloned()
            .collect();
        assert_eq!(a_violations_before.len(), a_violations_after.len());
        assert_eq!(
            a_violations_before
                .iter()
                .map(|v| v.rule)
                .collect::<Vec<_>>(),
            a_violations_after
                .iter()
                .map(|v| v.rule)
                .collect::<Vec<_>>()
        );

        Ok(())
    }
}
