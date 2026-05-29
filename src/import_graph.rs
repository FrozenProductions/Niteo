use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, Expression, ImportDeclaration, ImportExpression,
};
use oxc_ast_visit::Visit;
use oxc_span::Span;

#[cfg(test)]
use crate::config::structure::DomainConfig;

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
            .filter(|edge| edge.resolved_target.is_none() && is_relative_specifier(&edge.specifier))
            .count()
    }

    pub fn most_imported_files(&self, limit: usize) -> Vec<(PathBuf, usize)> {
        let mut import_counts: HashMap<PathBuf, usize> = HashMap::new();
        for edge in &self.edges {
            if let Some(target) = &edge.resolved_target {
                *import_counts.entry(target.clone()).or_insert(0) += 1;
            }
        }
        let mut sorted: Vec<_> = import_counts.into_iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        sorted.truncate(limit);
        sorted
    }

    pub fn highest_fanout_files(&self, limit: usize) -> Vec<(PathBuf, usize)> {
        let mut fanout_counts: HashMap<PathBuf, usize> = HashMap::new();
        for edge in &self.edges {
            *fanout_counts.entry(edge.source_file.clone()).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = fanout_counts.into_iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        sorted.truncate(limit);
        sorted
    }
}

pub fn build_import_graph(files: &[PathBuf], is_test_file: impl Fn(&Path) -> bool) -> ImportGraph {
    let mut graph = ImportGraph::new();

    for file in files {
        let is_barrel = is_barrel_file(file);
        let is_test = is_test_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    for file in files {
        if let Ok(source) = std::fs::read_to_string(file) {
            let edges = extract_imports(file, &source, files);
            graph.edges.extend(edges);
        }
    }

    graph
}

#[cfg(test)]
pub fn build_import_graph_from_sources(
    files_with_sources: &[(&str, &str)],
    tests_config: &DomainConfig,
) -> ImportGraph {
    let mut graph = ImportGraph::new();
    let files: Vec<PathBuf> = files_with_sources
        .iter()
        .map(|(path, _)| PathBuf::from(path))
        .collect();

    for file in &files {
        let is_barrel = is_barrel_file(file);
        let is_test = tests_config.matches_file(file);
        graph.add_file(file.clone(), is_barrel, is_test);
    }

    for (path, source) in files_with_sources {
        let file = PathBuf::from(path);
        let edges = extract_imports(&file, source, &files);
        graph.edges.extend(edges);
    }

    graph
}

fn extract_imports(source_file: &Path, source: &str, all_files: &[PathBuf]) -> Vec<ImportEdge> {
    let allocator = oxc_allocator::Allocator::default();
    let source_type = crate::syntax::source_type_from_path(source_file);
    let Some(source_type) = source_type else {
        return Vec::new();
    };

    let parser_return = oxc_parser::Parser::new(&allocator, source, source_type).parse();
    if parser_return.panicked {
        return Vec::new();
    }

    let mut visitor = ImportVisitor {
        source_file: source_file.to_path_buf(),
        all_files,
        edges: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(&parser_return.program);
    visitor.edges
}

struct ImportVisitor<'a> {
    source_file: PathBuf,
    all_files: &'a [PathBuf],
    edges: Vec<ImportEdge>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visit<'a> for ImportVisitor<'a> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        self.add_edge(&decl.source.value, ImportKind::Import, decl.span);
        oxc_ast_visit::walk::walk_import_declaration(self, decl);
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        self.add_edge(&decl.source.value, ImportKind::ReExport, decl.span);
        oxc_ast_visit::walk::walk_export_all_declaration(self, decl);
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
            self.add_edge(&source.value, ImportKind::ReExport, decl.span);
        }
        oxc_ast_visit::walk::walk_export_named_declaration(self, decl);
    }

    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(source) = &expr.source {
            self.add_edge(&source.value, ImportKind::DynamicImport, expr.span);
        }
        oxc_ast_visit::walk::walk_import_expression(self, expr);
    }
}

impl ImportVisitor<'_> {
    fn add_edge(&mut self, specifier: &str, kind: ImportKind, span: Span) {
        let resolved_target =
            resolve_import_specifier(&self.source_file, specifier, self.all_files);

        self.edges.push(ImportEdge {
            source_file: self.source_file.clone(),
            specifier: specifier.to_string(),
            resolved_target,
            kind,
            span,
        });
    }
}

pub fn is_relative_specifier(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}

fn resolve_import_specifier(
    source_file: &Path,
    specifier: &str,
    all_files: &[PathBuf],
) -> Option<PathBuf> {
    if !is_relative_specifier(specifier) {
        return None;
    }

    let parent = source_file.parent()?;
    let target = normalize_path(&parent.join(specifier));

    for candidate in all_files {
        let normalized_candidate = normalize_path(candidate);
        if normalized_candidate == target
            || extensionless(&normalized_candidate) == target
            || normalized_candidate.parent() == Some(target.as_path())
        {
            return Some(candidate.clone());
        }
    }

    None
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn extensionless(path: &Path) -> PathBuf {
    const TYPESCRIPT_EXTENSIONS: &[&str] = &["ts", "tsx"];
    if TYPESCRIPT_EXTENSIONS.contains(&path.extension().and_then(|ext| ext.to_str()).unwrap_or(""))
    {
        return path.with_extension("");
    }
    path.to_path_buf()
}

fn is_barrel_file(file: &Path) -> bool {
    file.file_name().and_then(|name| name.to_str()) == Some("index.ts")
}

mod serialization {
    use super::*;

    #[derive(Debug)]
    pub struct GraphFormatter {
        graph: ImportGraph,
    }

    impl GraphFormatter {
        pub fn new(graph: ImportGraph) -> Self {
            Self { graph }
        }

        pub fn to_dot(&self) -> String {
            let mut output = String::new();
            output.push_str("digraph imports {\n");
            output.push_str("  rankdir=LR;\n");
            output.push_str("  node [shape=box];\n\n");

            for (path, node) in &self.graph.files {
                let label = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let style = if node.is_barrel {
                    ", style=filled, fillcolor=lightblue"
                } else if node.is_test {
                    ", style=filled, fillcolor=lightyellow"
                } else {
                    ""
                };
                output.push_str(&format!(
                    "  \"{}\" [label=\"{}\"{}];\n",
                    path.display(),
                    label,
                    style
                ));
            }

            output.push('\n');

            for edge in &self.graph.edges {
                if let Some(target) = &edge.resolved_target {
                    let style = match edge.kind {
                        ImportKind::Import => "",
                        ImportKind::ReExport => ", style=bold",
                        ImportKind::DynamicImport => ", style=dotted",
                    };
                    output.push_str(&format!(
                        "  \"{}\" -> \"{}\" [label=\"{}\"{}];\n",
                        edge.source_file.display(),
                        target.display(),
                        edge.specifier,
                        style
                    ));
                }
            }

            output.push_str("}\n");
            output
        }
    }
}

use serialization::GraphFormatter;

impl ImportGraph {
    pub fn format_dot(&self) -> String {
        GraphFormatter::new(ImportGraph {
            files: self.files.clone(),
            edges: self.edges.clone(),
        })
        .to_dot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn identifies_relative_specifiers() {
        assert!(is_relative_specifier("./foo"));
        assert!(is_relative_specifier("../bar"));
        assert!(is_relative_specifier("/absolute"));
        assert!(!is_relative_specifier("lodash"));
        assert!(!is_relative_specifier("@scope/package"));
    }

    #[test]
    fn normalizes_paths_correctly() {
        let path = Path::new("src/components/../utils/./helper");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("src/utils/helper"));
    }

    #[test]
    fn detects_barrel_files() {
        assert!(is_barrel_file(Path::new("src/index.ts")));
        assert!(is_barrel_file(Path::new("components/index.ts")));
        assert!(!is_barrel_file(Path::new("src/Button.ts")));
    }

    #[test]
    fn resolves_relative_import() {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));
    }

    #[test]
    fn resolves_import_with_extension() {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b.ts", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));
    }

    #[test]
    fn resolves_directory_import_to_barrel() {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components/index.ts"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./components", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/components/index.ts")));
    }

    #[test]
    fn returns_none_for_external_import() {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "lodash", &files);
        assert_eq!(resolved, None);
    }

    #[test]
    fn returns_none_for_unresolved_import() {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./nonexistent", &files);
        assert_eq!(resolved, None);
    }
}
