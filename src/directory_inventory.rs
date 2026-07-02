use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_IGNORED_DIRECTORIES: &[&str] = &[
    "node_modules",
    ".git",
    ".vscode",
    ".idea",
    "dist",
    "build",
    "out",
    ".next",
    ".svelte-kit",
    "target",
];

pub const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx"];

const BARREL_FILE_NAMES: &[&str] = &["index.ts", "index.tsx"];

#[derive(Debug, Clone)]
pub struct DirectoryInventory {
    pub directories: Vec<DirectoryFacts>,
}

#[derive(Debug, Clone)]
pub struct DirectoryFacts {
    pub path: PathBuf,
    pub depth: usize,
    pub source_files: Vec<PathBuf>,
    pub subdirectories: Vec<PathBuf>,
    pub barrel_files: Vec<BarrelFileFacts>,
}

#[derive(Debug, Clone)]
pub struct BarrelFileFacts {
    pub is_empty: bool,
}

pub fn collect_directory_inventory(root: &Path, exclude_dirs: &[PathBuf]) -> DirectoryInventory {
    let mut directories = Vec::new();
    walk_and_collect(root, exclude_dirs, 0, &mut directories);
    DirectoryInventory { directories }
}

/// Return a view of `inventory` limited to directories under `root`, excluding
/// any directories under `exclude_dirs`. Depths are rebased so that `root`
/// itself is at depth 0, matching the values produced by a fresh collection
/// starting at `root`.
pub fn filter_inventory(
    inventory: &DirectoryInventory,
    root: &Path,
    exclude_dirs: &[PathBuf],
) -> DirectoryInventory {
    let is_excluded = |path: &Path| {
        exclude_dirs
            .iter()
            .any(|excluded| path.starts_with(excluded))
    };

    let depth_offset = inventory
        .directories
        .iter()
        .find(|facts| facts.path == root)
        .map(|facts| facts.depth)
        .unwrap_or(0);

    let directories: Vec<DirectoryFacts> = inventory
        .directories
        .iter()
        .filter(|facts| {
            let path = &facts.path;
            (path == root || path.starts_with(root)) && !is_excluded(path)
        })
        .map(|facts| {
            let subdirectories: Vec<PathBuf> = facts
                .subdirectories
                .iter()
                .filter(|sub| {
                    let sub_path = sub.as_path();
                    (sub_path == root || sub_path.starts_with(root)) && !is_excluded(sub_path)
                })
                .cloned()
                .collect();
            DirectoryFacts {
                path: facts.path.clone(),
                depth: facts.depth.saturating_sub(depth_offset),
                source_files: facts.source_files.clone(),
                subdirectories,
                barrel_files: facts.barrel_files.clone(),
            }
        })
        .collect();

    DirectoryInventory { directories }
}

fn walk_and_collect(
    current: &Path,
    exclude_dirs: &[PathBuf],
    depth: usize,
    directories: &mut Vec<DirectoryFacts>,
) {
    if exclude_dirs.iter().any(|excl| current == excl.as_path()) {
        return;
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut source_files = Vec::new();
    let mut subdirectories = Vec::new();
    let mut barrel_files = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if DEFAULT_IGNORED_DIRECTORIES.contains(&name_str.as_ref()) {
                continue;
            }
            if exclude_dirs.contains(&path) {
                continue;
            }
            subdirectories.push(path);
        } else if path.is_file() && is_source_file(&path) {
            source_files.push(path.clone());
            if is_barrel_file(&path) {
                let barrel_facts = analyze_barrel_file(&path);
                barrel_files.push(barrel_facts);
            }
        }
    }

    directories.push(DirectoryFacts {
        path: current.to_path_buf(),
        depth,
        source_files,
        subdirectories: subdirectories.clone(),
        barrel_files,
    });

    for subdir in subdirectories {
        walk_and_collect(&subdir, exclude_dirs, depth + 1, directories);
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}

fn is_barrel_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|os_name| os_name.to_str())
        .map(|name| BARREL_FILE_NAMES.contains(&name))
        .unwrap_or(false)
}

fn analyze_barrel_file(path: &Path) -> BarrelFileFacts {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return BarrelFileFacts { is_empty: false },
    };

    let is_empty = is_empty_barrel_source(&source);

    BarrelFileFacts { is_empty }
}

fn is_empty_barrel_source(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = 0;
    let mut has_content = false;

    while cursor < bytes.len() {
        skip_trivia(bytes, &mut cursor);

        if cursor >= bytes.len() {
            break;
        }

        let statement_start = cursor;
        read_statement(bytes, &mut cursor);
        let statement = bytes.get(statement_start..cursor).unwrap_or(b"");

        if has_re_export_content(statement) {
            has_content = true;
        } else if !is_re_export_or_empty(statement) {
            return false;
        }
    }

    !has_content
}

fn has_re_export_content(statement: &[u8]) -> bool {
    let trimmed = skip_whitespace(statement);
    if trimmed.is_empty() {
        return false;
    }

    let mut scanner = TokenScanner::new(trimmed);

    if scanner.next_token() != Some("export") {
        return false;
    }

    let second = scanner.next_token();
    match second {
        Some("type") => {
            let third = scanner.next_token();
            match third {
                Some("*") => true,
                Some("{") => scanner.has_identifier_before_close_brace(),
                _ => false,
            }
        }
        Some("*") => true,
        Some("{") => scanner.has_identifier_before_close_brace(),
        _ => false,
    }
}

fn is_re_export_or_empty(statement: &[u8]) -> bool {
    let trimmed = skip_whitespace(statement);
    if trimmed.is_empty() {
        return true;
    }

    let mut scanner = TokenScanner::new(trimmed);

    if scanner.next_token() != Some("export") {
        return false;
    }

    let second = scanner.next_token();
    match second {
        Some("type") => {
            let third = scanner.next_token();
            match third {
                Some("*") => scanner.contains_token("from"),
                Some("{") => scanner.contains_token("from"),
                _ => false,
            }
        }
        Some("*") => scanner.contains_token("from"),
        Some("{") => scanner.contains_token("from"),
        _ => false,
    }
}

fn skip_trivia(bytes: &[u8], cursor: &mut usize) {
    loop {
        while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
            *cursor += 1;
        }

        if *cursor >= bytes.len() {
            break;
        }

        if starts_with(bytes, *cursor, b"//") {
            skip_line_comment(bytes, cursor);
            continue;
        }

        if starts_with(bytes, *cursor, b"/*") {
            skip_block_comment(bytes, cursor);
            continue;
        }

        break;
    }
}

fn read_statement(bytes: &[u8], cursor: &mut usize) {
    let mut string_quote: Option<u8> = None;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;

    while *cursor < bytes.len() {
        let current = bytes[*cursor];
        let next = bytes.get(*cursor + 1).copied();

        if let Some(quote) = string_quote {
            if current == b'\\' {
                *cursor += 1;
                if *cursor < bytes.len() {
                    *cursor += 1;
                }
                continue;
            }

            if current == quote {
                string_quote = None;
            }

            *cursor += 1;
            continue;
        }

        match (current, next) {
            (b'\'', _) | (b'"', _) | (b'`', _) => {
                string_quote = Some(current);
                *cursor += 1;
            }
            (b'/', Some(b'/')) => skip_line_comment(bytes, cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, cursor),
            (b'{', _) => {
                brace_depth += 1;
                *cursor += 1;
            }
            (b'}', _) => {
                brace_depth = brace_depth.saturating_sub(1);
                *cursor += 1;
            }
            (b'(', _) => {
                paren_depth += 1;
                *cursor += 1;
            }
            (b')', _) => {
                paren_depth = paren_depth.saturating_sub(1);
                *cursor += 1;
            }
            (b';', _) if brace_depth == 0 && paren_depth == 0 => {
                *cursor += 1;
                break;
            }
            _ => *cursor += 1,
        }
    }
}

fn skip_line_comment(bytes: &[u8], cursor: &mut usize) {
    while *cursor < bytes.len() && bytes[*cursor] != b'\n' {
        *cursor += 1;
    }
}

fn skip_block_comment(bytes: &[u8], cursor: &mut usize) {
    *cursor += 1;
    *cursor += 1;

    while *cursor < bytes.len() {
        let current = bytes[*cursor];
        let next = bytes.get(*cursor + 1).copied();

        *cursor += 1;

        if current == b'*' && next == Some(b'/') {
            *cursor += 1;
            break;
        }
    }
}

fn starts_with(bytes: &[u8], index: usize, pattern: &[u8]) -> bool {
    bytes.get(index..index + pattern.len()) == Some(pattern)
}

fn skip_whitespace(bytes: &[u8]) -> &[u8] {
    let mut index = 0;
    while bytes.get(index).is_some_and(|&b| b.is_ascii_whitespace()) {
        index += 1;
    }
    bytes.get(index..).unwrap_or(b"")
}

#[derive(Debug)]
struct TokenScanner<'a> {
    source: &'a [u8],
    index: usize,
}

impl<'a> TokenScanner<'a> {
    fn new(source: &'a [u8]) -> Self {
        Self { source, index: 0 }
    }

    fn next_token(&mut self) -> Option<&'a str> {
        self.skip_non_tokens();

        if self.index >= self.source.len() {
            return None;
        }

        let start = self.index;
        if self.source[self.index].is_ascii_alphabetic() {
            while self.index < self.source.len() && self.source[self.index].is_ascii_alphabetic() {
                self.index += 1;
            }
        } else {
            self.index += 1;
        }

        std::str::from_utf8(self.source.get(start..self.index).unwrap_or(b"")).ok()
    }

    fn contains_token(&mut self, expected: &str) -> bool {
        while let Some(token) = self.next_token() {
            if token == expected {
                return true;
            }
        }

        false
    }

    fn has_identifier_before_close_brace(&mut self) -> bool {
        while self.index < self.source.len() {
            if self.source[self.index] == b'}' {
                return false;
            }
            if self.source[self.index].is_ascii_alphabetic()
                || self.source[self.index] == b'_'
                || self.source[self.index] == b'$'
            {
                return true;
            }
            self.index += 1;
        }

        false
    }

    fn skip_non_tokens(&mut self) {
        while self.index < self.source.len() {
            if self.source[self.index].is_ascii_alphanumeric()
                || matches!(self.source[self.index], b'{' | b'}' | b'*')
            {
                break;
            }

            self.index += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::{Context, Result};
    use std::fs;

    fn create_temp_dir() -> Result<PathBuf> {
        let dir = std::env::temp_dir().join(format!(
            "niteo_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn cleanup_temp_dir(dir: &Path) {
        match fs::remove_dir_all(dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to remove {}: {error}", dir.display()),
        }
    }

    #[test]
    fn inventory_walks_tree_and_records_facts() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("src/components"))?;
        fs::write(root.join("src/index.ts"), "export * from './components';")?;
        fs::write(
            root.join("src/components/Button.tsx"),
            "export const Button = () => {};",
        )?;

        let inventory = collect_directory_inventory(&root, &[]);

        assert!(inventory.directories.len() >= 3);
        let src_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("src"))
            .context("expected src directory")?;
        assert_eq!(src_facts.source_files.len(), 1);
        assert_eq!(src_facts.subdirectories.len(), 1);
        assert_eq!(src_facts.depth, 1);

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_excludes_default_ignored_directories() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("node_modules/pkg"))?;
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("node_modules/pkg/index.ts"), "export {};")?;
        fs::write(root.join("src/index.ts"), "export {};")?;

        let inventory = collect_directory_inventory(&root, &[]);

        assert!(
            !inventory
                .directories
                .iter()
                .any(|d| d.path.ends_with("node_modules"))
        );
        assert!(
            inventory
                .directories
                .iter()
                .any(|d| d.path.ends_with("src"))
        );

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_excludes_specified_directories() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("src"))?;
        fs::create_dir_all(root.join("generated"))?;
        fs::write(root.join("src/index.ts"), "export {};")?;
        fs::write(root.join("generated/types.ts"), "export type Foo = {};")?;

        let exclude = vec![root.join("generated")];
        let inventory = collect_directory_inventory(&root, &exclude);

        assert!(
            !inventory
                .directories
                .iter()
                .any(|d| d.path.ends_with("generated"))
        );
        assert!(
            inventory
                .directories
                .iter()
                .any(|d| d.path.ends_with("src"))
        );

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_handles_unreadable_paths_gracefully() -> Result<()> {
        let root = create_temp_dir()?;
        let nonexistent = root.join("nonexistent");

        let inventory = collect_directory_inventory(&nonexistent, &[]);

        assert!(inventory.directories.is_empty());

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_detects_empty_barrel_files() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("empty"))?;
        fs::write(root.join("empty/index.ts"), "")?;

        let inventory = collect_directory_inventory(&root, &[]);

        let empty_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("empty"))
            .context("expected empty directory")?;
        assert_eq!(empty_facts.barrel_files.len(), 1);
        assert!(empty_facts.barrel_files[0].is_empty);

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_detects_non_empty_barrel_files() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("components"))?;
        fs::write(
            root.join("components/index.ts"),
            "export { Button } from './Button';",
        )?;
        fs::write(
            root.join("components/Button.tsx"),
            "export const Button = () => {};",
        )?;

        let inventory = collect_directory_inventory(&root, &[]);

        let comp_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("components"))
            .context("expected components directory")?;
        assert_eq!(comp_facts.barrel_files.len(), 1);
        assert!(!comp_facts.barrel_files[0].is_empty);

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn inventory_records_correct_depth() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("a/b/c"))?;

        let inventory = collect_directory_inventory(&root, &[]);

        let a_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("a"))
            .context("expected a directory")?;
        let b_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("b"))
            .context("expected b directory")?;
        let c_facts = inventory
            .directories
            .iter()
            .find(|d| d.path.ends_with("c"))
            .context("expected c directory")?;

        assert_eq!(a_facts.depth, 1);
        assert_eq!(b_facts.depth, 2);
        assert_eq!(c_facts.depth, 3);

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn is_empty_barrel_source_detects_empty() -> Result<()> {
        assert!(is_empty_barrel_source(""));
        assert!(is_empty_barrel_source("// just a comment\n"));
        assert!(is_empty_barrel_source("/* block comment */\n"));
        Ok(())
    }

    #[test]
    fn is_empty_barrel_source_detects_re_exports() -> Result<()> {
        assert!(!is_empty_barrel_source(
            "export { Button } from './Button';\n"
        ));
        assert!(!is_empty_barrel_source("export * from './Button';\n"));
        assert!(!is_empty_barrel_source(
            "export type { Props } from './types';\n"
        ));
        Ok(())
    }

    #[test]
    fn is_empty_barrel_source_rejects_logic() -> Result<()> {
        assert!(!is_empty_barrel_source("const x = 1;\n"));
        assert!(!is_empty_barrel_source("export const x = 1;\n"));
        assert!(!is_empty_barrel_source("export { Button };\n"));
        Ok(())
    }

    #[test]
    fn filter_inventory_selects_subtree_and_rebases_depth() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("a/b"))?;
        fs::write(root.join("a/index.ts"), "export {};")?;
        fs::write(root.join("a/b/index.ts"), "export {};")?;

        let inventory = collect_directory_inventory(&root, &[]);
        let subtree = filter_inventory(&inventory, &root.join("a"), &[]);

        assert_eq!(subtree.directories.len(), 2);
        let a_facts = subtree
            .directories
            .iter()
            .find(|d| d.path.ends_with("a"))
            .context("expected a directory")?;
        let b_facts = subtree
            .directories
            .iter()
            .find(|d| d.path.ends_with("b"))
            .context("expected b directory")?;
        assert_eq!(a_facts.depth, 0);
        assert_eq!(b_facts.depth, 1);

        cleanup_temp_dir(&root);
        Ok(())
    }

    #[test]
    fn filter_inventory_excludes_directories_and_descendants() -> Result<()> {
        let root = create_temp_dir()?;
        fs::create_dir_all(root.join("a/keep"))?;
        fs::create_dir_all(root.join("a/skip"))?;
        fs::write(root.join("a/keep/index.ts"), "export {};")?;
        fs::write(root.join("a/skip/index.ts"), "export {};")?;

        let inventory = collect_directory_inventory(&root, &[]);
        let filtered = filter_inventory(&inventory, &root.join("a"), &[root.join("a/skip")]);

        assert!(
            !filtered
                .directories
                .iter()
                .any(|d| d.path.ends_with("skip"))
        );
        assert!(
            filtered
                .directories
                .iter()
                .any(|d| d.path.ends_with("keep"))
        );
        let a_facts = filtered
            .directories
            .iter()
            .find(|d| d.path.ends_with("a"))
            .context("expected a directory")?;
        assert!(!a_facts.subdirectories.iter().any(|d| d.ends_with("skip")));

        cleanup_temp_dir(&root);
        Ok(())
    }
}
