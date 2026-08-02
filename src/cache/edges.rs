use std::path::Path;

use oxc_span::Span;
use serde::{Deserialize, Serialize};

use crate::cache::key::{denormalize_path_from_cache, normalize_path_for_cache};
use crate::import_graph::{ImportEdge, ImportKind};
use crate::import_resolver::SpecifierKind;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedImportEdge {
    pub specifier: String,
    pub resolved_target: Option<String>,
    pub specifier_kind: String,
    pub kind: String,
    pub span_start: u32,
    pub span_end: u32,
}

pub fn import_edge_to_cached(edge: &ImportEdge, project_root: &Path) -> CachedImportEdge {
    CachedImportEdge {
        specifier: edge.specifier.clone(),
        resolved_target: edge
            .resolved_target
            .as_ref()
            .map(|t| normalize_path_for_cache(t, project_root)),
        specifier_kind: match edge.specifier_kind {
            SpecifierKind::Relative => "relative".to_string(),
            SpecifierKind::Alias => "alias".to_string(),
            SpecifierKind::Package => "package".to_string(),
            SpecifierKind::External => "external".to_string(),
        },
        kind: match edge.kind {
            ImportKind::Import => "import".to_string(),
            ImportKind::ReExport => "re_export".to_string(),
            ImportKind::DynamicImport => "dynamic_import".to_string(),
        },
        span_start: edge.span.start,
        span_end: edge.span.end,
    }
}

pub fn cached_import_edges_to_import(
    edges: &[CachedImportEdge],
    source_file: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    edges
        .iter()
        .map(|edge| ImportEdge {
            source_file: source_file.to_path_buf(),
            specifier: edge.specifier.clone(),
            specifier_kind: match edge.specifier_kind.as_str() {
                "relative" => SpecifierKind::Relative,
                "alias" => SpecifierKind::Alias,
                "package" => SpecifierKind::Package,
                _ => SpecifierKind::External,
            },
            resolved_target: edge
                .resolved_target
                .as_ref()
                .map(|t| denormalize_path_from_cache(t, project_root)),
            kind: match edge.kind.as_str() {
                "re_export" => ImportKind::ReExport,
                "dynamic_import" => ImportKind::DynamicImport,
                _ => ImportKind::Import,
            },
            span: Span::new(edge.span_start, edge.span_end),
        })
        .collect()
}
