use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use oxc_span::Span;

use niteo::cache::edges::CachedImportEdge;
use niteo::cache::key::{CACHE_SCHEMA_VERSION, compute_rule_hashes, hash_content, hash_file_list};
use niteo::cache::lifecycle::{CacheState, finalize_cache, prepare_cache};
use niteo::cache::store::{
    CacheFile, CachedFileAnalysis, CachedViolation, cache_path, clear_cache, read_cache,
    write_cache,
};
use niteo::config::Severity;
use niteo::config::{ConfigSet, ConfigSetOptions};
use niteo::import_graph::{ImportEdge, ImportGraph, ImportKind};
use niteo::import_resolver::SpecifierKind;
use niteo::rules::{NO_CONSOLE_RULE_ID, Violation};

fn write_minimal_config(project_root: &std::path::Path) -> Result<std::path::PathBuf> {
    let config_path = project_root.join("niteo.toml");
    std::fs::write(&config_path, "[project]\nroot = \".\"\n")?;
    Ok(config_path)
}

fn resolve_config_set(project_root: &std::path::Path) -> Result<ConfigSet> {
    ConfigSet::resolve(
        project_root,
        ConfigSetOptions {
            root_override: None,
            scan_scope: None,
            deny_child_configs: false,
        },
    )
}

#[test]
fn read_missing_cache_returns_none() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let result = read_cache(temp_dir.path())?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn read_corrupted_cache_returns_error() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let path = temp_dir.path().join(".niteo").join("cache.json");
    std::fs::create_dir_all(path.parent().context("expected parent directory")?)?;
    std::fs::write(&path, "not valid json")?;
    let result = read_cache(temp_dir.path());
    assert!(result.is_err());
    Ok(())
}

#[test]
fn write_and_read_cache_roundtrip() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut files = HashMap::new();
    files.insert(
        "src/a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: "abc".to_string(),
            import_edges: vec![CachedImportEdge {
                specifier: "./b".to_string(),
                resolved_target: Some("src/b.ts".to_string()),
                specifier_kind: "relative".to_string(),
                kind: "import".to_string(),
                span_start: 10,
                span_end: 20,
            }],
            violations: Vec::new(),
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: "0.2.0".to_string(),
        rule_hashes: HashMap::new(),
        tsconfig_hash: None,
        file_list_hash: "fl".to_string(),
        files,
        graph: None,
    };
    write_cache(temp_dir.path(), &cache)?;
    let read = read_cache(temp_dir.path())?.context("missing cache")?;
    assert_eq!(read.version, CACHE_SCHEMA_VERSION);
    assert_eq!(read.niteo_version, "0.2.0");
    let entry = read
        .files
        .get("src/a.ts")
        .context("missing a.ts cache entry")?;
    assert_eq!(entry.content_hash, "abc");
    assert_eq!(entry.import_edges.len(), 1);
    assert_eq!(entry.import_edges[0].specifier, "./b");
    Ok(())
}

#[test]
fn clear_cache_removes_file() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: "0.2.0".to_string(),
        rule_hashes: HashMap::new(),
        tsconfig_hash: None,
        file_list_hash: "fl".to_string(),
        files: HashMap::new(),
        graph: None,
    };
    write_cache(temp_dir.path(), &cache)?;
    assert!(cache_path(temp_dir.path()).exists());
    clear_cache(temp_dir.path())?;
    assert!(!cache_path(temp_dir.path()).exists());
    Ok(())
}

#[test]
fn clear_cache_missing_is_noop() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    clear_cache(temp_dir.path())?;
    assert!(!cache_path(temp_dir.path()).exists());
    Ok(())
}

#[test]
fn prepare_cache_miss_when_file_hash_changes() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "original")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"original"),
            import_edges: Vec::new(),
            violations: Vec::new(),
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    std::fs::write(&file, "changed")?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;
    assert!(!state.cached_edges.contains_key(&file));
    Ok(())
}

#[test]
fn prepare_cache_hit_when_unchanged() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "original")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"original"),
            import_edges: vec![CachedImportEdge {
                specifier: "./b".to_string(),
                resolved_target: Some("b.ts".to_string()),
                specifier_kind: "relative".to_string(),
                kind: "import".to_string(),
                span_start: 0,
                span_end: 5,
            }],
            violations: Vec::new(),
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;
    let edges = state
        .cached_edges
        .get(&file)
        .context("missing cached edges for file")?;
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].specifier, "./b");
    assert_eq!(edges[0].resolved_target, Some(project_root.join("b.ts")));
    assert_eq!(edges[0].span, Span::new(0, 5));
    Ok(())
}

#[test]
fn prepare_cache_hit_restores_violations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "original")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"original"),
            import_edges: Vec::new(),
            violations: vec![CachedViolation {
                line: Some(1),
                column: Some(2),
                rule: "no-console".to_string(),
                message: "message".to_string(),
                severity: "warn".to_string(),
                detail: Some("detail".to_string()),
                subject: Some("subject".to_string()),
            }],
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;
    let violations = state
        .cached_violations
        .get(&file)
        .context("missing cached violations")?;
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-console");
    assert_eq!(violations[0].message, "message");
    assert_eq!(violations[0].severity, Severity::Warn);
    assert_eq!(violations[0].detail, Some("detail".to_string()));
    assert_eq!(violations[0].subject, Some("subject".to_string()));
    Ok(())
}

#[test]
fn finalize_cache_writes_violations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "content")?;

    let file_list_hash = hash_file_list(std::slice::from_ref(&file));

    let mut file_hashes = HashMap::new();
    file_hashes.insert(file.clone(), hash_content(b"content"));

    let state = CacheState {
        file_hashes,
        sources: HashMap::new(),
        cached_edges: HashMap::new(),
        cached_violations: Arc::new(HashMap::new()),
        cached_parse_failures: HashMap::new(),
        cached_topology: None,
        dirty: true,
        file_list_hash,
        rule_hashes: HashMap::new(),
        changed_rules: Arc::new(HashSet::new()),
        tsconfig_hash: None,
    };

    let violation = Violation {
        file: file.clone(),
        span: None,
        line: Some(1),
        column: Some(2),
        rule: NO_CONSOLE_RULE_ID,
        message: "Disallow console statements.",
        severity: Severity::Warn,
        detail: None,
        subject: None,
    };

    finalize_cache(
        project_root,
        std::slice::from_ref(&file),
        &state,
        &ImportGraph::new(),
        &[violation],
        &HashMap::new(),
    )?;

    let read = read_cache(project_root)?.context("missing cache")?;
    let entry = read.files.get("a.ts").context("missing a.ts entry")?;
    assert_eq!(entry.violations.len(), 1);
    assert_eq!(entry.violations[0].rule, "no-console");
    assert_eq!(entry.violations[0].message, "Disallow console statements.");
    Ok(())
}

#[test]
fn finalize_cache_preserves_cached_violations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file_a = project_root.join("a.ts");
    let file_b = project_root.join("b.ts");
    std::fs::write(&file_a, "content a")?;
    std::fs::write(&file_b, "content b")?;

    let file_list_hash = hash_file_list(&[file_a.clone(), file_b.clone()]);

    let mut file_hashes = HashMap::new();
    file_hashes.insert(file_a.clone(), hash_content(b"content a"));
    file_hashes.insert(file_b.clone(), hash_content(b"content b"));

    let mut cached_violations = HashMap::new();
    cached_violations.insert(
        file_a.clone(),
        vec![Violation {
            file: file_a.clone(),
            span: None,
            line: Some(1),
            column: Some(1),
            rule: NO_CONSOLE_RULE_ID,
            message: "cached violation",
            severity: Severity::Warn,
            detail: None,
            subject: None,
        }],
    );

    let state = CacheState {
        file_hashes,
        sources: HashMap::new(),
        cached_edges: HashMap::new(),
        cached_violations: Arc::new(cached_violations),
        cached_parse_failures: HashMap::new(),
        cached_topology: None,
        dirty: true,
        file_list_hash,
        rule_hashes: HashMap::new(),
        changed_rules: Arc::new(HashSet::new()),
        tsconfig_hash: None,
    };

    let new_violation = Violation {
        file: file_b.clone(),
        span: None,
        line: Some(2),
        column: Some(2),
        rule: NO_CONSOLE_RULE_ID,
        message: "new violation",
        severity: Severity::Error,
        detail: None,
        subject: None,
    };

    finalize_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &state,
        &ImportGraph::new(),
        &[new_violation],
        &HashMap::new(),
    )?;

    let read = read_cache(project_root)?.context("missing cache")?;
    let entry_a = read.files.get("a.ts").context("missing a.ts entry")?;
    assert_eq!(entry_a.violations.len(), 1);
    assert_eq!(entry_a.violations[0].message, "cached violation");

    let entry_b = read.files.get("b.ts").context("missing b.ts entry")?;
    assert_eq!(entry_b.violations.len(), 1);
    assert_eq!(entry_b.violations[0].message, "new violation");
    Ok(())
}

#[test]
fn prepare_cache_invalidates_when_niteo_version_changes() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "content")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"content"),
            import_edges: Vec::new(),
            violations: Vec::new(),
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: "old-version".to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;
    assert!(!state.cached_edges.contains_key(&file));
    Ok(())
}

#[test]
fn prepare_cache_invalidates_only_changed_rule_violations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "content")?;

    let config_path = project_root.join("niteo.toml");
    std::fs::write(
        &config_path,
        "[project]\nroot = \".\"\n[rules.no-console]\nseverity = \"warn\"\n[rules.no-debugger]\nseverity = \"warn\"\n",
    )?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"content"),
            import_edges: Vec::new(),
            violations: vec![
                CachedViolation {
                    line: Some(1),
                    column: Some(1),
                    rule: "no-console".to_string(),
                    message: "no-console violation".to_string(),
                    severity: "warn".to_string(),
                    detail: None,
                    subject: None,
                },
                CachedViolation {
                    line: Some(2),
                    column: Some(2),
                    rule: "no-debugger".to_string(),
                    message: "no-debugger violation".to_string(),
                    severity: "warn".to_string(),
                    detail: None,
                    subject: None,
                },
            ],
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    std::fs::write(
        &config_path,
        "[project]\nroot = \".\"\n[rules.no-console]\nseverity = \"error\"\n[rules.no-debugger]\nseverity = \"warn\"\n",
    )?;
    let config_set = resolve_config_set(project_root)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;

    assert!(state.dirty);
    let violations = state
        .cached_violations
        .get(&file)
        .context("expected cached violations")?;
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-debugger");
    Ok(())
}

#[test]
fn prepare_cache_keeps_violations_for_unchanged_rules() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "content")?;

    let config_path = project_root.join("niteo.toml");
    std::fs::write(
        &config_path,
        "[project]\nroot = \".\"\n[rules.no-console]\nseverity = \"warn\"\n[rules.no-debugger]\nseverity = \"warn\"\n",
    )?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"content"),
            import_edges: Vec::new(),
            violations: vec![
                CachedViolation {
                    line: Some(1),
                    column: Some(1),
                    rule: "no-console".to_string(),
                    message: "no-console violation".to_string(),
                    severity: "warn".to_string(),
                    detail: None,
                    subject: None,
                },
                CachedViolation {
                    line: Some(2),
                    column: Some(2),
                    rule: "no-debugger".to_string(),
                    message: "no-debugger violation".to_string(),
                    severity: "warn".to_string(),
                    detail: None,
                    subject: None,
                },
            ],
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    std::fs::write(
        &config_path,
        "[project]\nroot = \".\"\n[rules.no-console]\nseverity = \"warn\"\n[rules.no-debugger]\nseverity = \"error\"\n",
    )?;
    let config_set = resolve_config_set(project_root)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;

    assert!(state.dirty);
    let violations = state
        .cached_violations
        .get(&file)
        .context("expected cached violations")?;
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "no-console");
    Ok(())
}

#[test]
fn prepare_cache_invalidates_when_file_list_changes() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file_a = project_root.join("a.ts");
    std::fs::write(&file_a, "content")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let rule_hashes = compute_rule_hashes(&config_set);

    let mut files_map = HashMap::new();
    files_map.insert(
        "a.ts".to_string(),
        CachedFileAnalysis {
            content_hash: hash_content(b"content"),
            import_edges: Vec::new(),
            violations: Vec::new(),
            parse_failure: None,
        },
    );
    let cache = CacheFile {
        version: CACHE_SCHEMA_VERSION,
        niteo_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_hashes,
        tsconfig_hash: None,
        file_list_hash: hash_file_list(std::slice::from_ref(&file_a)),
        files: files_map,
        graph: None,
    };
    write_cache(project_root, &cache)?;

    let file_b = project_root.join("b.ts");
    std::fs::write(&file_b, "other")?;

    let state = prepare_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &config_set,
        None,
    )?
    .context("missing cache")?;
    assert!(!state.cached_edges.contains_key(&file_a));
    Ok(())
}

#[test]
fn prepare_cache_returns_none_when_no_cache_exists() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file = project_root.join("a.ts");
    std::fs::write(&file, "content")?;

    let config_set = resolve_config_set(project_root)?;

    let state = prepare_cache(project_root, std::slice::from_ref(&file), &config_set, None)?
        .context("missing cache")?;
    assert!(state.cached_edges.is_empty());
    assert!(state.cached_violations.is_empty());
    Ok(())
}

#[test]
fn finalize_cache_writes_all_files() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file_a = project_root.join("a.ts");
    let file_b = project_root.join("b.ts");
    std::fs::write(&file_a, "content a")?;
    std::fs::write(&file_b, "content b")?;

    let file_list_hash = hash_file_list(&[file_a.clone(), file_b.clone()]);

    let mut graph = ImportGraph::new();
    graph.add_file(file_a.clone(), false, false);
    graph.add_file(file_b.clone(), false, false);
    graph.add_edge(ImportEdge {
        source_file: file_a.clone(),
        specifier: "./b".to_string(),
        specifier_kind: SpecifierKind::Relative,
        resolved_target: Some(file_b.clone()),
        kind: ImportKind::Import,
        span: Span::new(0, 10),
    });
    graph.build_edges_by_source();

    let mut file_hashes = HashMap::new();
    file_hashes.insert(file_a.clone(), hash_content(b"content a"));
    file_hashes.insert(file_b.clone(), hash_content(b"content b"));

    let state = CacheState {
        file_hashes,
        sources: HashMap::new(),
        cached_edges: HashMap::new(),
        cached_violations: Arc::new(HashMap::new()),
        cached_parse_failures: HashMap::new(),
        cached_topology: None,
        dirty: true,
        file_list_hash,
        rule_hashes: HashMap::new(),
        changed_rules: Arc::new(HashSet::new()),
        tsconfig_hash: None,
    };

    finalize_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &state,
        &graph,
        &[],
        &HashMap::new(),
    )?;

    let read = read_cache(project_root)?.context("missing cache")?;
    assert_eq!(read.files.len(), 2);
    let entry_a = read.files.get("a.ts").context("missing a.ts cache entry")?;
    assert_eq!(entry_a.import_edges.len(), 1);
    assert_eq!(entry_a.import_edges[0].specifier, "./b");
    Ok(())
}

#[test]
fn finalize_cache_writes_graph_topology() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file_a = project_root.join("a.ts");
    let file_b = project_root.join("b.ts");
    std::fs::write(&file_a, "content a")?;
    std::fs::write(&file_b, "content b")?;

    let file_list_hash = hash_file_list(&[file_a.clone(), file_b.clone()]);

    let mut graph = ImportGraph::new();
    graph.add_file(file_a.clone(), false, false);
    graph.add_file(file_b.clone(), false, false);
    graph.add_edge(ImportEdge {
        source_file: file_a.clone(),
        specifier: "./b".to_string(),
        specifier_kind: SpecifierKind::Relative,
        resolved_target: Some(file_b.clone()),
        kind: ImportKind::Import,
        span: Span::new(0, 10),
    });
    graph.add_edge(ImportEdge {
        source_file: file_b.clone(),
        specifier: "./a".to_string(),
        specifier_kind: SpecifierKind::Relative,
        resolved_target: Some(file_a.clone()),
        kind: ImportKind::Import,
        span: Span::new(0, 10),
    });
    graph.build_edges_by_source();
    niteo::cache::lifecycle::ensure_graph_topology(&mut graph);

    let mut file_hashes = HashMap::new();
    file_hashes.insert(file_a.clone(), hash_content(b"content a"));
    file_hashes.insert(file_b.clone(), hash_content(b"content b"));

    let state = CacheState {
        file_hashes,
        sources: HashMap::new(),
        cached_edges: HashMap::new(),
        cached_violations: Arc::new(HashMap::new()),
        cached_parse_failures: HashMap::new(),
        cached_topology: None,
        dirty: true,
        file_list_hash,
        rule_hashes: HashMap::new(),
        changed_rules: Arc::new(HashSet::new()),
        tsconfig_hash: None,
    };

    finalize_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &state,
        &graph,
        &[],
        &HashMap::new(),
    )?;

    let read = read_cache(project_root)?.context("missing cache")?;
    let cached_graph = read.graph.context("missing graph topology")?;
    assert!(!cached_graph.edge_hash.is_empty());
    assert_eq!(cached_graph.cycles.len(), 2);
    assert_eq!(cached_graph.imported_files.len(), 2);
    Ok(())
}

#[test]
fn prepare_cache_restores_graph_topology_when_unchanged() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_root = temp_dir.path();
    let file_a = project_root.join("a.ts");
    let file_b = project_root.join("b.ts");
    std::fs::write(&file_a, "content a")?;
    std::fs::write(&file_b, "content b")?;

    let _config_path = write_minimal_config(project_root)?;
    let config_set = resolve_config_set(project_root)?;
    let file_list_hash = hash_file_list(&[file_a.clone(), file_b.clone()]);

    let mut graph = ImportGraph::new();
    graph.add_file(file_a.clone(), false, false);
    graph.add_file(file_b.clone(), false, false);
    graph.add_edge(ImportEdge {
        source_file: file_a.clone(),
        specifier: "./b".to_string(),
        specifier_kind: SpecifierKind::Relative,
        resolved_target: Some(file_b.clone()),
        kind: ImportKind::Import,
        span: Span::new(0, 10),
    });
    graph.build_edges_by_source();
    niteo::cache::lifecycle::ensure_graph_topology(&mut graph);

    let mut file_hashes = HashMap::new();
    file_hashes.insert(file_a.clone(), hash_content(b"content a"));
    file_hashes.insert(file_b.clone(), hash_content(b"content b"));

    let state = CacheState {
        file_hashes: file_hashes.clone(),
        sources: HashMap::new(),
        cached_edges: HashMap::new(),
        cached_violations: Arc::new(HashMap::new()),
        cached_parse_failures: HashMap::new(),
        cached_topology: None,
        dirty: true,
        file_list_hash,
        rule_hashes: HashMap::new(),
        changed_rules: Arc::new(HashSet::new()),
        tsconfig_hash: None,
    };

    finalize_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &state,
        &graph,
        &[],
        &HashMap::new(),
    )?;

    let state = prepare_cache(
        project_root,
        &[file_a.clone(), file_b.clone()],
        &config_set,
        None,
    )?
    .context("missing cache")?;

    let cached_graph = state.cached_topology.context("missing cached topology")?;
    assert_eq!(cached_graph.edge_hash, graph.compute_edge_hash());
    assert_eq!(
        cached_graph.cycles.len(),
        graph.cycles_by_file().unwrap().len()
    );
    Ok(())
}
