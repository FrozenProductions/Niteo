use std::cell::Cell;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_span::{SourceType, Span};

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct LineIndex {
    line_starts: Vec<u32>,
}

thread_local! {
    static REUSABLE_LINE_INDEX: Cell<Option<LineIndex>> = const { Cell::new(None) };
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut this = Self {
            line_starts: Vec::with_capacity(source.len() / 40 + 1),
        };
        this.populate(source);
        this
    }

    pub fn populate(&mut self, source: &str) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                self.line_starts.push(offset as u32 + 1);
            }
        }
    }

    pub fn position(&self, byte_offset: u32) -> Position {
        let line_index = match self.line_starts.binary_search(&byte_offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        Position {
            line: line_index + 1,
            column: (byte_offset - line_start) as usize + 1,
        }
    }

    pub fn position_for(&self, span: Span) -> Position {
        self.position(span.start)
    }
}

pub(crate) fn with_reusable_line_index<R>(source: &str, f: impl FnOnce(&LineIndex) -> R) -> R {
    REUSABLE_LINE_INDEX.with(|cell| {
        let mut index = cell.replace(None).unwrap_or_else(|| LineIndex::new(source));
        index.populate(source);
        let result = f(&index);
        cell.set(Some(index));
        result
    })
}

pub fn is_typescript_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts") | Some("tsx")
    )
}

pub fn source_type_from_path(path: &Path) -> Option<SourceType> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "ts" => Some(SourceType::ts()),
        "tsx" => Some(SourceType::tsx()),
        "mts" => Some(SourceType::ts().with_module(true)),
        "cts" => Some(SourceType::ts().with_commonjs(true)),
        "js" => Some(SourceType::default()),
        "jsx" => Some(SourceType::jsx()),
        "mjs" => Some(SourceType::mjs()),
        "cjs" => Some(SourceType::cjs()),
        _ => None,
    }
}

/// A structured record of a syntax error (or parser panic) in one source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    pub file: PathBuf,
    pub message: String,
    pub span: Option<Span>,
}

/// The result of parsing one source file: the (possibly partial) program and
/// every diagnostic the parser produced. A non-empty `failures` means the
/// source is invalid or incomplete even when `program` is usable.
#[derive(Debug)]
pub struct ParsedFile<'a> {
    pub program: Program<'a>,
    pub failures: Vec<ParseFailure>,
}

/// Parse `source` once and convert the oxc parser result into a structured
/// `ParsedFile`. This is the single parser-result conversion used by import
/// graph extraction, linting, and fixing.
pub fn parse_program<'a>(
    allocator: &'a Allocator,
    file: &Path,
    source: &'a str,
    source_type: SourceType,
) -> ParsedFile<'a> {
    let parser_return = oxc_parser::Parser::new(allocator, source, source_type).parse();

    let mut failures: Vec<ParseFailure> = parser_return
        .errors
        .iter()
        .map(|error| ParseFailure {
            file: file.to_path_buf(),
            message: error.message.to_string(),
            span: error.labels.as_ref().and_then(|labels| {
                labels.first().map(|label| {
                    let start = label.offset() as u32;
                    Span::new(start, start + label.len() as u32)
                })
            }),
        })
        .collect();

    // A panicked parser always records the fatal error in `errors`, so this
    // synthetic entry only covers the unlikely case of a panic with no
    // diagnostic attached.
    if parser_return.panicked && parser_return.errors.is_empty() {
        failures.push(ParseFailure {
            file: file.to_path_buf(),
            message: "parser panicked".to_string(),
            span: None,
        });
    }

    ParsedFile {
        program: parser_return.program,
        failures,
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::{Context, Result};
    use std::path::Path;

    #[test]
    fn detects_typescript() -> Result<()> {
        let source_type = source_type_from_path(Path::new("foo.ts")).context("ts")?;
        assert!(source_type.is_typescript());
        assert!(!source_type.is_jsx());
        Ok(())
    }

    #[test]
    fn detects_tsx() -> Result<()> {
        let source_type = source_type_from_path(Path::new("Component.tsx")).context("tsx")?;
        assert!(source_type.is_typescript());
        assert!(source_type.is_jsx());
        Ok(())
    }

    #[test]
    fn detects_jsx() -> Result<()> {
        let source_type = source_type_from_path(Path::new("Component.jsx")).context("jsx")?;
        assert!(source_type.is_jsx());
        assert!(!source_type.is_typescript());
        Ok(())
    }

    #[test]
    fn detects_javascript_module_kinds() -> Result<()> {
        let mjs = source_type_from_path(Path::new("module.mjs")).context("mjs")?;
        assert!(mjs.is_module());
        let cjs = source_type_from_path(Path::new("module.cjs")).context("cjs")?;
        assert!(cjs.is_commonjs());
        Ok(())
    }

    #[test]
    fn rejects_unknown_extension() -> Result<()> {
        assert!(source_type_from_path(Path::new("README.md")).is_none());
        Ok(())
    }

    #[test]
    fn detects_ts_file() {
        assert!(is_typescript_file(Path::new("foo.ts")));
    }

    #[test]
    fn detects_tsx_file() {
        assert!(is_typescript_file(Path::new("Component.tsx")));
    }

    #[test]
    fn rejects_non_ts_extension() {
        assert!(!is_typescript_file(Path::new("foo.js")));
        assert!(!is_typescript_file(Path::new("foo.jsx")));
        assert!(!is_typescript_file(Path::new("foo.css")));
        assert!(!is_typescript_file(Path::new("Cargo.toml")));
        assert!(!is_typescript_file(Path::new("README.md")));
    }

    #[test]
    fn rejects_partial_matches() {
        assert!(!is_typescript_file(Path::new("file.ats")));
        assert!(!is_typescript_file(Path::new("file.atsx")));
        assert!(!is_typescript_file(Path::new("file.d.ts.map")));
    }

    #[test]
    fn handles_paths_with_directories() {
        assert!(is_typescript_file(Path::new("a/b/c/deep.ts")));
        assert!(is_typescript_file(Path::new("a/b/c/deep.tsx")));
        assert!(!is_typescript_file(Path::new("a/b/c/deep.js")));
    }

    #[test]
    fn line_index_reports_lines_and_columns() -> Result<()> {
        let source = "first\nsecond line\nthird";
        let index = LineIndex::new(source);
        assert_eq!(index.position(0).line, 1);
        assert_eq!(index.position(0).column, 1);
        assert_eq!(index.position(6).line, 2);
        assert_eq!(index.position(6).column, 1);
        assert_eq!(index.position(18).line, 3);
        assert_eq!(index.position(18).column, 1);
        Ok(())
    }

    #[test]
    fn parse_program_reports_invalid_source_as_failure() {
        let allocator = Allocator::default();
        let file = Path::new("src/broken.ts");
        let parsed = parse_program(&allocator, file, "const x = ;\n", SourceType::ts());
        assert_eq!(parsed.failures.len(), 1);
        assert_eq!(parsed.failures[0].file, file);
        assert!(!parsed.failures[0].message.is_empty());
        let span = parsed.failures[0]
            .span
            .expect("failure should carry a span");
        assert!(span.end >= span.start);
    }

    #[test]
    fn parse_program_never_panics_on_invalid_source() {
        let allocator = Allocator::default();
        let file = Path::new("src/broken.ts");
        let parsed = parse_program(&allocator, file, "export const = ;\n", SourceType::ts());
        assert_eq!(parsed.failures.len(), 1);
        assert!(parsed.program.source_type.is_typescript());
    }

    #[test]
    fn parse_program_is_clean_for_valid_source() {
        let allocator = Allocator::default();
        let file = Path::new("src/clean.ts");
        let parsed = parse_program(&allocator, file, "export const a = 1;\n", SourceType::ts());
        assert!(parsed.failures.is_empty());
        assert!(!parsed.program.body.is_empty());
    }
}
