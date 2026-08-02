use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

#[derive(Debug, Default, Clone)]
pub struct FileNode {
    pub is_barrel: bool,
    pub is_test: bool,
}

#[derive(Debug, Default, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub node: FileNode,
}

#[derive(Debug, Default)]
pub struct ImportGraph {
    pub(crate) files: Vec<FileEntry>,
    file_index: HashMap<PathBuf, u32>,
    edges: Vec<ImportEdge>,
    edges_by_source: Vec<Vec<usize>>,
    edges_by_target: Vec<Vec<u32>>,
    pub(crate) cycles_by_file: Option<HashMap<PathBuf, Vec<PathBuf>>>,
    pub(crate) imported_files: Option<HashSet<PathBuf>>,
    pub(crate) graph_parse_failures: HashSet<PathBuf>,
    cached_edge_hash: Mutex<Option<String>>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_cycles_by_file(&mut self, cycles: HashMap<PathBuf, Vec<PathBuf>>) {
        self.cycles_by_file = Some(cycles);
    }

    pub fn cycles_by_file(&self) -> Option<&HashMap<PathBuf, Vec<PathBuf>>> {
        self.cycles_by_file.as_ref()
    }

    pub fn set_imported_files(&mut self, files: HashSet<PathBuf>) {
        self.imported_files = Some(files);
    }

    pub fn imported_files(&self) -> Option<&HashSet<PathBuf>> {
        self.imported_files.as_ref()
    }

    pub fn add_graph_parse_failure(&mut self, file: PathBuf) {
        self.graph_parse_failures.insert(file);
    }

    pub fn has_graph_parse_failure(&self, file: &Path) -> bool {
        self.graph_parse_failures.contains(file)
    }

    pub fn graph_parse_failures(&self) -> &HashSet<PathBuf> {
        &self.graph_parse_failures
    }

    pub fn compute_edge_hash(&self) -> String {
        {
            let cached = self
                .cached_edge_hash
                .lock()
                .expect("edge hash lock poisoned");
            if let Some(hash) = cached.as_ref() {
                return hash.clone();
            }
        }

        let hash = self.calculate_edge_hash();
        let mut cached = self
            .cached_edge_hash
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *cached = Some(hash.clone());
        hash
    }

    fn calculate_edge_hash(&self) -> String {
        let mut indices: Vec<usize> = (0..self.edges.len()).collect();
        indices.sort_unstable_by(|&a, &b| {
            let edge_a = &self.edges[a];
            let edge_b = &self.edges[b];
            let a_source = edge_a.source_file.as_os_str().as_encoded_bytes();
            let b_source = edge_b.source_file.as_os_str().as_encoded_bytes();
            a_source
                .cmp(b_source)
                .then_with(|| {
                    let a_target = edge_a
                        .resolved_target
                        .as_ref()
                        .map(|path| path.as_os_str().as_encoded_bytes());
                    let b_target = edge_b
                        .resolved_target
                        .as_ref()
                        .map(|path| path.as_os_str().as_encoded_bytes());
                    a_target.cmp(&b_target)
                })
                .then_with(|| edge_a.specifier.as_bytes().cmp(edge_b.specifier.as_bytes()))
                .then_with(|| import_kind_byte(edge_a.kind).cmp(&import_kind_byte(edge_b.kind)))
                .then_with(|| {
                    specifier_kind_byte(edge_a.specifier_kind)
                        .cmp(&specifier_kind_byte(edge_b.specifier_kind))
                })
        });

        let mut hasher = blake3::Hasher::new();
        for index in indices {
            let edge = &self.edges[index];
            hasher.update(edge.source_file.as_os_str().as_encoded_bytes());
            hasher.update(b"\0");
            match &edge.resolved_target {
                Some(target) => hasher.update(target.as_os_str().as_encoded_bytes()),
                None => hasher.update(b"_"),
            };
            hasher.update(b"\0");
            hasher.update(edge.specifier.as_bytes());
            hasher.update(b"\0");
            hasher.update(&[import_kind_byte(edge.kind)]);
            hasher.update(b"\0");
            hasher.update(&[specifier_kind_byte(edge.specifier_kind)]);
            hasher.update(b"\n");
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn add_edge(&mut self, edge: ImportEdge) {
        self.edges.push(edge);
        self.invalidate_edge_hash();
    }

    pub fn extend_edges(&mut self, new_edges: impl IntoIterator<Item = ImportEdge>) {
        self.edges.extend(new_edges);
        self.invalidate_edge_hash();
    }

    fn invalidate_edge_hash(&self) {
        let mut cached = self
            .cached_edge_hash
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *cached = None;
    }

    pub fn add_file(&mut self, path: PathBuf, is_barrel: bool, is_test: bool) {
        let index = self.files.len() as u32;
        self.files.push(FileEntry {
            path: path.clone(),
            node: FileNode { is_barrel, is_test },
        });
        self.file_index.insert(path, index);
    }

    pub fn build_edges_by_source(&mut self) {
        self.edges_by_source = vec![Vec::new(); self.files.len()];
        self.edges_by_target = vec![Vec::new(); self.files.len()];
        for (edge_index, edge) in self.edges.iter().enumerate() {
            if let Some(&source_index) = self.file_index.get(&edge.source_file) {
                self.edges_by_source[source_index as usize].push(edge_index);
                if let Some(ref target_path) = edge.resolved_target
                    && let Some(&target_index) = self.file_index.get(target_path)
                {
                    self.edges_by_target[source_index as usize].push(target_index);
                }
            }
        }
        for targets in self.edges_by_target.iter_mut() {
            targets.sort_unstable();
            targets.dedup();
        }
    }

    pub(crate) fn edges_by_target(&self) -> &[Vec<u32>] {
        &self.edges_by_target
    }

    pub fn edges(&self) -> &[ImportEdge] {
        &self.edges
    }

    pub fn edges_by_file(&self) -> HashMap<PathBuf, &[ImportEdge]> {
        let mut map = HashMap::new();
        for (file_index, edge_indices) in self.edges_by_source.iter().enumerate() {
            let path = self.files[file_index].path.clone();
            let slice = match (edge_indices.first(), edge_indices.last()) {
                (Some(&first), Some(&last)) => {
                    debug_assert!(
                        edge_indices
                            .iter()
                            .enumerate()
                            .all(|(offset, &index)| index == first + offset),
                        "edges for a single source file must be contiguous"
                    );
                    &self.edges[first..=last]
                }
                _ => &[],
            };
            map.insert(path, slice);
        }
        map
    }

    pub fn iter_files(&self) -> impl Iterator<Item = (&Path, &FileNode)> + '_ {
        self.files
            .iter()
            .map(|entry| (entry.path.as_path(), &entry.node))
    }

    pub fn edges_from<'a>(&'a self, file: &Path) -> impl Iterator<Item = &'a ImportEdge> + 'a {
        self.file_index
            .get(file)
            .and_then(|&file_index| self.edges_by_source.get(file_index as usize))
            .into_iter()
            .flat_map(|indices| indices.iter())
            .map(|&edge_index| &self.edges[edge_index])
    }

    pub fn file_node(&self, path: &Path) -> Option<&FileNode> {
        self.file_index
            .get(path)
            .map(|&file_index| &self.files[file_index as usize].node)
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
                        SpecifierKind::Relative | SpecifierKind::Alias | SpecifierKind::Package
                    )
            })
            .count()
    }

    pub fn unresolved_by_kind(&self) -> UnresolvedBreakdown {
        let mut relative = 0;
        let mut alias = 0;
        let mut package = 0;
        for edge in &self.edges {
            if edge.resolved_target.is_some() {
                continue;
            }
            match edge.specifier_kind {
                SpecifierKind::Relative => relative += 1,
                SpecifierKind::Alias => alias += 1,
                SpecifierKind::Package => package += 1,
                SpecifierKind::External => {}
            }
        }
        UnresolvedBreakdown {
            relative,
            alias,
            package,
        }
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
    pub package: usize,
}

fn import_kind_byte(kind: ImportKind) -> u8 {
    match kind {
        ImportKind::Import => 0,
        ImportKind::ReExport => 1,
        ImportKind::DynamicImport => 2,
    }
}

fn specifier_kind_byte(kind: SpecifierKind) -> u8 {
    match kind {
        SpecifierKind::Relative => 0,
        SpecifierKind::Alias => 1,
        SpecifierKind::Package => 2,
        SpecifierKind::External => 3,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use oxc_span::Span;

    use crate::import_graph::ImportKind;
    use crate::import_graph::model::{ImportEdge, ImportGraph};
    use crate::import_resolver::SpecifierKind;

    #[test]
    fn edge_hash_is_stable_for_same_edges() {
        let mut graph = ImportGraph::new();
        graph.add_file(PathBuf::from("/repo/a.ts"), false, false);
        graph.add_file(PathBuf::from("/repo/b.ts"), false, false);
        graph.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/a.ts"),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/b.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });
        graph.build_edges_by_source();

        let first = graph.compute_edge_hash();
        let second = graph.compute_edge_hash();
        assert_eq!(first, second);
    }

    #[test]
    fn edge_hash_is_order_independent() {
        let mut forward = ImportGraph::new();
        forward.add_file(PathBuf::from("/repo/a.ts"), false, false);
        forward.add_file(PathBuf::from("/repo/b.ts"), false, false);
        forward.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/a.ts"),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/b.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });
        forward.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/b.ts"),
            specifier: "./a".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/a.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });

        let mut reverse = ImportGraph::new();
        reverse.add_file(PathBuf::from("/repo/a.ts"), false, false);
        reverse.add_file(PathBuf::from("/repo/b.ts"), false, false);
        reverse.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/b.ts"),
            specifier: "./a".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/a.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });
        reverse.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/a.ts"),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/b.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });

        assert_eq!(forward.compute_edge_hash(), reverse.compute_edge_hash());
    }

    #[test]
    fn edge_hash_changes_when_edges_change() {
        let mut graph = ImportGraph::new();
        graph.add_file(PathBuf::from("/repo/a.ts"), false, false);
        graph.add_file(PathBuf::from("/repo/b.ts"), false, false);
        graph.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/a.ts"),
            specifier: "./b".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/b.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });
        graph.build_edges_by_source();

        let before = graph.compute_edge_hash();
        graph.add_edge(ImportEdge {
            source_file: PathBuf::from("/repo/b.ts"),
            specifier: "./a".to_string(),
            specifier_kind: SpecifierKind::Relative,
            resolved_target: Some(PathBuf::from("/repo/a.ts")),
            kind: ImportKind::Import,
            span: Span::new(0, 10),
        });
        graph.build_edges_by_source();

        let after = graph.compute_edge_hash();
        assert_ne!(before, after);
    }
}
