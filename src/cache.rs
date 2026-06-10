pub mod edges;
pub mod key;
pub mod lifecycle;
pub mod store;
#[allow(unused_imports)]
pub use edges::{CachedImportEdge, cached_import_edges_to_import};
#[allow(unused_imports)]
pub use key::{
    CACHE_SCHEMA_VERSION, denormalize_path_from_cache, hash_config_files, hash_content,
    hash_file_list, hash_string, hash_tsconfig, is_cache_valid, normalize_path_for_cache,
};
#[allow(unused_imports)]
pub use lifecycle::{CacheState, finalize_cache, prepare_cache};
#[allow(unused_imports)]
pub use store::{
    CacheFile, CachedFileAnalysis, CachedParseFailure, CachedViolation, cache_path, clear_cache,
    read_cache, write_cache,
};
#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use oxc_span::Span;

    use crate::cache::edges::import_edge_to_cached;
    use crate::import_graph::{ImportEdge, ImportGraph, ImportKind};
    use crate::import_resolver::SpecifierKind;

    use super::*;

    #[test]
    fn hash_content_is_stable() {
        let a = hash_content(b"hello");
        let b = hash_content(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_content_changes_with_input() {
        let a = hash_content(b"hello");
        let b = hash_content(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn hash_file_list_is_sorted() {
        let a = hash_file_list(&[PathBuf::from("b"), PathBuf::from("a")]);
        let b = hash_file_list(&[PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(a, b);
    }

    #[test]
    fn cache_valid_matches_all_fields() {
        let cache = CacheFile {
            version: CACHE_SCHEMA_VERSION,
            niteo_version: "0.2.0".to_string(),
            config_hash: "abc".to_string(),
            tsconfig_hash: Some("def".to_string()),
            file_list_hash: "ghi".to_string(),
            files: HashMap::new(),
        };
        assert!(is_cache_valid(&cache, "0.2.0", "abc", Some("def"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.1", "abc", Some("def"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", "xyz", Some("def"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", "abc", Some("xyz"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", "abc", None, "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", "abc", Some("def"), "xyz"));
    }

    #[test]
    fn cache_version_mismatch_invalidates() {
        let cache = CacheFile {
            version: 999,
            niteo_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash: "abc".to_string(),
            tsconfig_hash: None,
            file_list_hash: "ghi".to_string(),
            files: HashMap::new(),
        };
        assert!(!is_cache_valid(
            &cache,
            env!("CARGO_PKG_VERSION"),
            "abc",
            None,
            "ghi"
        ));
    }

    #[test]
    fn read_missing_cache_returns_none() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = read_cache(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_corrupted_cache_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join(".niteo").join("cache.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not valid json").unwrap();
        let result = read_cache(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn write_and_read_cache_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
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
            config_hash: "cfg".to_string(),
            tsconfig_hash: None,
            file_list_hash: "fl".to_string(),
            files,
        };
        write_cache(temp_dir.path(), &cache).unwrap();
        let read = read_cache(temp_dir.path()).unwrap().unwrap();
        assert_eq!(read.version, CACHE_SCHEMA_VERSION);
        assert_eq!(read.niteo_version, "0.2.0");
        let entry = read.files.get("src/a.ts").unwrap();
        assert_eq!(entry.content_hash, "abc");
        assert_eq!(entry.import_edges.len(), 1);
        assert_eq!(entry.import_edges[0].specifier, "./b");
    }

    #[test]
    fn clear_cache_removes_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = CacheFile {
            version: CACHE_SCHEMA_VERSION,
            niteo_version: "0.2.0".to_string(),
            config_hash: "cfg".to_string(),
            tsconfig_hash: None,
            file_list_hash: "fl".to_string(),
            files: HashMap::new(),
        };
        write_cache(temp_dir.path(), &cache).unwrap();
        assert!(cache_path(temp_dir.path()).exists());
        clear_cache(temp_dir.path()).unwrap();
        assert!(!cache_path(temp_dir.path()).exists());
    }

    #[test]
    fn clear_cache_missing_is_noop() {
        let temp_dir = tempfile::tempdir().unwrap();
        clear_cache(temp_dir.path()).unwrap();
        assert!(!cache_path(temp_dir.path()).exists());
    }

    #[test]
    fn prepare_cache_miss_when_file_hash_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file = project_root.join("a.ts");
        std::fs::write(&file, "original").unwrap();

        let config_path = project_root.join("niteo.toml");
        std::fs::write(&config_path, "test config").unwrap();
        let config_hash = hash_config_files(&[config_path.clone()]);

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
            config_hash,
            tsconfig_hash: None,
            file_list_hash: hash_file_list(&[file.clone()]),
            files: files_map,
        };
        write_cache(project_root, &cache).unwrap();

        std::fs::write(&file, "changed").unwrap();

        let state = prepare_cache(project_root, &[file.clone()], &[config_path], None)
            .unwrap()
            .unwrap();
        assert!(state.cached_edges.get(&file).is_none());
    }

    #[test]
    fn prepare_cache_hit_when_unchanged() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file = project_root.join("a.ts");
        std::fs::write(&file, "original").unwrap();

        let config_path = project_root.join("niteo.toml");
        std::fs::write(&config_path, "test config").unwrap();
        let config_hash = hash_config_files(&[config_path.clone()]);

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
            config_hash,
            tsconfig_hash: None,
            file_list_hash: hash_file_list(&[file.clone()]),
            files: files_map,
        };
        write_cache(project_root, &cache).unwrap();

        let state = prepare_cache(project_root, &[file.clone()], &[config_path], None)
            .unwrap()
            .unwrap();
        let edges = state.cached_edges.get(&file).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].specifier, "./b");
        assert_eq!(edges[0].resolved_target, Some(project_root.join("b.ts")));
        assert_eq!(edges[0].span, Span::new(0, 5));
    }

    #[test]
    fn prepare_cache_invalidates_when_niteo_version_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file = project_root.join("a.ts");
        std::fs::write(&file, "content").unwrap();

        let config_path = project_root.join("niteo.toml");
        std::fs::write(&config_path, "test config").unwrap();
        let config_hash = hash_config_files(&[config_path.clone()]);

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
            config_hash,
            tsconfig_hash: None,
            file_list_hash: hash_file_list(&[file.clone()]),
            files: files_map,
        };
        write_cache(project_root, &cache).unwrap();

        let state = prepare_cache(project_root, &[file.clone()], &[config_path], None)
            .unwrap()
            .unwrap();
        assert!(state.cached_edges.get(&file).is_none());
    }

    #[test]
    fn prepare_cache_invalidates_when_config_hash_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file = project_root.join("a.ts");
        std::fs::write(&file, "content").unwrap();

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
            config_hash: "old-cfg".to_string(),
            tsconfig_hash: None,
            file_list_hash: hash_file_list(&[file.clone()]),
            files: files_map,
        };
        write_cache(project_root, &cache).unwrap();

        let config_path = project_root.join("niteo.toml");
        std::fs::write(&config_path, "new config").unwrap();

        let state = prepare_cache(project_root, &[file.clone()], &[config_path], None)
            .unwrap()
            .unwrap();
        assert!(state.cached_edges.get(&file).is_none());
    }

    #[test]
    fn prepare_cache_invalidates_when_file_list_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file_a = project_root.join("a.ts");
        std::fs::write(&file_a, "content").unwrap();

        let config_path = project_root.join("niteo.toml");
        std::fs::write(&config_path, "test config").unwrap();
        let config_hash = hash_config_files(&[config_path.clone()]);

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
            config_hash,
            tsconfig_hash: None,
            file_list_hash: hash_file_list(&[file_a.clone()]),
            files: files_map,
        };
        write_cache(project_root, &cache).unwrap();

        let file_b = project_root.join("b.ts");
        std::fs::write(&file_b, "other").unwrap();

        let state = prepare_cache(
            project_root,
            &[file_a.clone(), file_b.clone()],
            &[config_path],
            None,
        )
        .unwrap()
        .unwrap();
        assert!(state.cached_edges.get(&file_a).is_none());
    }

    #[test]
    fn prepare_cache_returns_none_when_no_cache_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file = project_root.join("a.ts");
        std::fs::write(&file, "content").unwrap();

        let state = prepare_cache(
            project_root,
            &[file.clone()],
            &[project_root.join("niteo.toml")],
            None,
        )
        .unwrap()
        .unwrap();
        assert!(state.cache.is_none());
        assert!(state.cached_edges.is_empty());
    }

    #[test]
    fn finalize_cache_writes_all_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_root = temp_dir.path();
        let file_a = project_root.join("a.ts");
        let file_b = project_root.join("b.ts");
        std::fs::write(&file_a, "content a").unwrap();
        std::fs::write(&file_b, "content b").unwrap();

        let mut graph = ImportGraph::new();
        graph.add_file(file_a.clone(), false, false);
        graph.add_file(file_b.clone(), false, false);
        graph.edges.push(ImportEdge {
            source_file: file_a.clone(),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(file_b.clone()),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });

        let mut file_hashes = HashMap::new();
        file_hashes.insert(file_a.clone(), hash_content(b"content a"));
        file_hashes.insert(file_b.clone(), hash_content(b"content b"));

        let state = CacheState {
            cache: None,
            file_hashes,
            cached_edges: HashMap::new(),
            dirty: true,
        };

        finalize_cache(
            project_root,
            &[file_a.clone(), file_b.clone()],
            &[project_root.join("niteo.toml")],
            None,
            &state,
            &graph,
        )
        .unwrap();

        let read = read_cache(project_root).unwrap().unwrap();
        assert_eq!(read.files.len(), 2);
        let entry_a = read.files.get("a.ts").unwrap();
        assert_eq!(entry_a.import_edges.len(), 1);
        assert_eq!(entry_a.import_edges[0].specifier, "./b");
    }

    #[test]
    fn normalize_path_for_cache_strips_prefix() {
        let root = Path::new("/project");
        let path = Path::new("/project/src/a.ts");
        assert_eq!(normalize_path_for_cache(path, root), "src/a.ts");
    }

    #[test]
    fn normalize_path_for_cache_falls_back_to_full() {
        let root = Path::new("/project");
        let path = Path::new("/other/src/a.ts");
        assert_eq!(normalize_path_for_cache(path, root), "/other/src/a.ts");
    }

    #[test]
    fn denormalize_path_from_cache_reconstructs() {
        let root = Path::new("/project");
        assert_eq!(
            denormalize_path_from_cache("src/a.ts", root),
            PathBuf::from("/project/src/a.ts")
        );
    }

    #[test]
    fn import_edge_to_cached_roundtrip() {
        let project_root = Path::new("/project");
        let edge = ImportEdge {
            source_file: PathBuf::from("/project/src/a.ts"),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/project/src/b.ts")),
            kind: ImportKind::Import,
            span: Span::new(10, 20),
        };
        let cached = import_edge_to_cached(&edge, project_root);
        assert_eq!(cached.specifier, "./b");
        assert_eq!(cached.resolved_target, Some("src/b.ts".to_string()));
        assert_eq!(cached.kind, "import");
        assert_eq!(cached.span_start, 10);
        assert_eq!(cached.span_end, 20);
    }

    #[test]
    fn cached_import_edges_to_import_roundtrip() {
        let project_root = Path::new("/project");
        let cached = CachedImportEdge {
            specifier: "./b".to_string(),
            resolved_target: Some("src/b.ts".to_string()),
            specifier_kind: "relative".to_string(),
            kind: "re_export".to_string(),
            span_start: 5,
            span_end: 15,
        };
        let edges =
            cached_import_edges_to_import(&[cached], &project_root.join("src/a.ts"), project_root);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].specifier, "./b");
        assert_eq!(
            edges[0].resolved_target,
            Some(project_root.join("src/b.ts"))
        );
        assert_eq!(edges[0].kind, ImportKind::ReExport);
        assert_eq!(edges[0].span, Span::new(5, 15));
    }
}
