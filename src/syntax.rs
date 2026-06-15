use std::path::Path;

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

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = Vec::with_capacity(source.len() / 40 + 1);
        line_starts.push(0);
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset as u32 + 1);
            }
        }
        Self { line_starts }
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
}
