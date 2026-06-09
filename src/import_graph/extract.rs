use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ExportAllDeclaration, ExportNamedDeclaration, Expression, ImportDeclaration, ImportExpression,
};
use oxc_ast_visit::Visit;
use oxc_span::Span;

use crate::import_graph::model::{ImportEdge, ImportKind};
use crate::import_graph::resolver::ImportResolverIndex;

pub(crate) fn extract_imports(
    source_file: &Path,
    source: &str,
    resolver: &ImportResolverIndex,
) -> Vec<ImportEdge> {
    let allocator = Allocator::default();
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
        resolver,
        edges: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(&parser_return.program);
    visitor.edges
}

struct ImportVisitor<'a> {
    source_file: PathBuf,
    resolver: &'a ImportResolverIndex,
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
        let resolved_target = self.resolver.resolve(&self.source_file, specifier);

        self.edges.push(ImportEdge {
            source_file: self.source_file.clone(),
            specifier: specifier.to_string(),
            resolved_target,
            kind,
            span,
        });
    }
}
