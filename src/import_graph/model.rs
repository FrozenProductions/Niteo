use std::collections::HashMap;
use std::path::{Path, PathBuf};

use oxc_span::Span;

use crate::import_resolver::SpecifierKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Import,
    ReExport,
    DynamicImport,
}

#[derive(Debug, Clone)]
pub struct ImportEdge {
    pub source_file: PathBuf,
    pub specifier: String,
    pub specifier_kind: SpecifierKind,
    pub resolved_target: Option<PathBuf>,
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub is_barrel: bool,
    pub is_test: bool,
}

#[derive(Debug, Default)]
pub struct ImportGraph {
    pub files: HashMap<PathBuf, FileNode>,
    pub edges: Vec<ImportEdge>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: PathBuf, is_barrel: bool, is_test: bool) {
        self.files.insert(path, FileNode { is_barrel, is_test });
    }

    pub fn edges_from<'a>(&'a self, file: &'a Path) -> impl Iterator<Item = &'a ImportEdge> + 'a {
        self.edges
            .iter()
            .filter(move |edge| edge.source_file == file)
    }

    pub fn file_node(&self, path: &Path) -> Option<&FileNode> {
        self.files.get(path)
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn unresolved_count(&self) -> usize {
        self.edges
            .iter()
            .filter(|edge| {
                edge.resolved_target.is_none()
                    && matches!(
                        edge.specifier_kind,
                        SpecifierKind::Relative | SpecifierKind::Alias
                    )
            })
            .count()
    }

    pub fn unresolved_by_kind(&self) -> UnresolvedBreakdown {
        let mut relative = 0;
        let mut alias = 0;
        for edge in &self.edges {
            if edge.resolved_target.is_some() {
                continue;
            }
            match edge.specifier_kind {
                SpecifierKind::Relative => relative += 1,
                SpecifierKind::Alias => alias += 1,
                SpecifierKind::External => {}
            }
        }
        UnresolvedBreakdown { relative, alias }
    }

    pub fn most_imported_files(&self, limit: usize) -> Vec<(PathBuf, usize)> {
        let mut import_counts: HashMap<PathBuf, usize> = HashMap::new();
        for edge in &self.edges {
            if let Some(target) = &edge.resolved_target {
                *import_counts.entry(target.clone()).or_insert(0) += 1;
            }
        }
        let mut sorted: Vec<_> = import_counts.into_iter().collect();
        sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        sorted.truncate(limit);
        sorted
    }

    pub fn highest_fanout_files(&self, limit: usize) -> Vec<(PathBuf, usize)> {
        let mut fanout_counts: HashMap<PathBuf, usize> = HashMap::new();
        for edge in &self.edges {
            *fanout_counts.entry(edge.source_file.clone()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = fanout_counts.into_iter().collect();
        sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        sorted.truncate(limit);
        sorted
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UnresolvedBreakdown {
    pub relative: usize,
    pub alias: usize,
}
