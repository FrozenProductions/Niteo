pub mod edges;
pub mod key;
pub mod lifecycle;
pub mod store;
pub mod violations;
#[cfg(test)]
mod tests {

    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use oxc_span::Span;

    use crate::cache::edges::{
        CachedImportEdge, cached_import_edges_to_import, import_edge_to_cached,
    };
    use crate::cache::key::{
        CACHE_SCHEMA_VERSION, denormalize_path_from_cache, hash_content, hash_file_list,
        is_cache_valid, normalize_path_for_cache,
    };
    use crate::cache::store::CacheFile;
    use crate::import_graph::{ImportEdge, ImportKind};
    use crate::import_resolver::SpecifierKind;

    #[test]
    fn hash_content_is_stable() -> Result<()> {
        let a = hash_content(b"hello");
        let b = hash_content(b"hello");
        assert_eq!(a, b);
        Ok(())
    }

    #[test]
    fn hash_content_changes_with_input() -> Result<()> {
        let a = hash_content(b"hello");
        let b = hash_content(b"world");
        assert_ne!(a, b);
        Ok(())
    }

    #[test]
    fn hash_file_list_is_sorted() -> Result<()> {
        let a = hash_file_list(&[PathBuf::from("b"), PathBuf::from("a")]);
        let b = hash_file_list(&[PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(a, b);
        Ok(())
    }

    #[test]
    fn cache_valid_matches_all_fields() -> Result<()> {
        let cache = CacheFile {
            version: CACHE_SCHEMA_VERSION,
            niteo_version: "0.2.0".to_string(),
            rule_hashes: HashMap::new(),
            tsconfig_hash: Some("def".to_string()),
            file_list_hash: "ghi".to_string(),
            files: HashMap::new(),
            graph: None,
        };
        assert!(is_cache_valid(&cache, "0.2.0", Some("def"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.1", Some("def"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", Some("xyz"), "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", None, "ghi"));
        assert!(!is_cache_valid(&cache, "0.2.0", Some("def"), "xyz"));
        Ok(())
    }

    #[test]
    fn cache_version_mismatch_invalidates() -> Result<()> {
        let cache = CacheFile {
            version: 999,
            niteo_version: env!("CARGO_PKG_VERSION").to_string(),
            rule_hashes: HashMap::new(),
            tsconfig_hash: None,
            file_list_hash: "ghi".to_string(),
            files: HashMap::new(),
            graph: None,
        };
        assert!(!is_cache_valid(
            &cache,
            env!("CARGO_PKG_VERSION"),
            None,
            "ghi"
        ));
        Ok(())
    }

    #[test]
    fn normalize_path_for_cache_strips_prefix() -> Result<()> {
        let root = Path::new("/project");
        let path = Path::new("/project/src/a.ts");
        assert_eq!(normalize_path_for_cache(path, root), "src/a.ts");
        Ok(())
    }

    #[test]
    fn normalize_path_for_cache_falls_back_to_full() -> Result<()> {
        let root = Path::new("/project");
        let path = Path::new("/other/src/a.ts");
        assert_eq!(normalize_path_for_cache(path, root), "/other/src/a.ts");
        Ok(())
    }

    #[test]
    fn denormalize_path_from_cache_reconstructs() -> Result<()> {
        let root = Path::new("/project");
        assert_eq!(
            denormalize_path_from_cache("src/a.ts", root),
            PathBuf::from("/project/src/a.ts")
        );
        Ok(())
    }

    #[test]
    fn import_edge_to_cached_roundtrip() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn cached_import_edges_to_import_roundtrip() -> Result<()> {
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
        Ok(())
    }
}
