use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{NoEmptyDirectoriesRuleConfig, Severity};
use crate::rules::{NO_EMPTY_DIRECTORIES_RULE_ID, Violation};
const MESSAGE: &str = "Remove directories with no source files or only empty barrel files.";

const IGNORED_DIRECTORIES: &[&str] = &[
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

const SOURCE_EXTENSIONS: &[&str] = &["ts", "tsx"];
const BARREL_FILE: &str = "index.ts";

pub fn check_directories(
    root: &Path,
    config: &NoEmptyDirectoriesRuleConfig,
    exclude_dirs: &[PathBuf],
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut ignored = config.ignore_dirs.clone();
    ignored.extend(IGNORED_DIRECTORIES.iter().map(|s| s.to_string()));

    walk_directories(root, &ignored, exclude_dirs, &mut violations);

    violations
}

fn walk_directories(
    current: &Path,
    ignored: &[String],
    exclude_dirs: &[PathBuf],
    violations: &mut Vec<Violation>,
) {
    if exclude_dirs.iter().any(|excl| current == excl.as_path()) {
        return;
    }

    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();
    let mut has_source_file = false;
    let mut barrel_path: Option<PathBuf> = None;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if path.is_dir() {
            if ignored.iter().any(|ign| name_str == *ign) {
                continue;
            }
            if exclude_dirs.contains(&path) {
                continue;
            }
            subdirs.push(path);
        } else if path.is_file() && is_source_file(&path) {
            has_source_file = true;
            if is_barrel_file(&path) {
                barrel_path = Some(path);
            }
        }
    }

    let is_empty_dir = !has_source_file && subdirs.is_empty();
    let is_empty_barrel_dir = if let Some(barrel) = &barrel_path {
        has_source_file
            && subdirs.is_empty()
            && !has_other_source_files(barrel, current)
            && is_empty_barrel(barrel)
    } else {
        false
    };

    if is_empty_dir || is_empty_barrel_dir {
        violations.push(directory_violation(current, Severity::Warn));
    }

    for subdir in subdirs {
        walk_directories(&subdir, ignored, exclude_dirs, violations);
    }
}

fn has_other_source_files(barrel_path: &Path, dir: &Path) -> bool {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.is_file() && is_source_file(&path) && path != barrel_path {
            return true;
        }
    }

    false
}

fn is_empty_barrel(path: &Path) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return false,
    };

    is_empty_barrel_source(&source)
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
        let statement = &bytes[statement_start..cursor];

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

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if SOURCE_EXTENSIONS.contains(&ext)
    )
}

fn is_barrel_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some(BARREL_FILE)
}

fn directory_violation(dir: &Path, severity: Severity) -> Violation {
    Violation {
        file: dir.to_path_buf(),
        line: None,
        column: None,
        rule: NO_EMPTY_DIRECTORIES_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
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
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    &bytes[i..]
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

        std::str::from_utf8(&self.source[start..self.index]).ok()
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
    use super::is_empty_barrel_source;

    #[test]
    fn detects_empty_barrel_file() {
        assert!(is_empty_barrel_source(""));
    }

    #[test]
    fn detects_barrel_with_only_comments() {
        let source = "// This is a barrel file\n/* re-exports below */\n";
        assert!(is_empty_barrel_source(source));
    }

    #[test]
    fn allows_barrel_with_named_re_exports() {
        let source = "export { Button } from \"./Button\";\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn allows_barrel_with_namespace_re_exports() {
        let source = "export * from \"./Button\";\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn allows_barrel_with_type_re_exports() {
        let source = "export type { ButtonProps } from \"./Button.type\";\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn rejects_barrel_with_logic() {
        let source = "export { Button } from \"./Button\";\nconst value = 1;\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn rejects_barrel_with_local_export() {
        let source = "export { Button };\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn rejects_barrel_with_import() {
        let source = "import { Button } from \"./Button\";\nexport { Button };\n";
        assert!(!is_empty_barrel_source(source));
    }

    #[test]
    fn allows_multiline_re_exports() {
        let source = "export {\n  Button,\n  type ButtonProps,\n} from \"./Button\";\n";
        assert!(!is_empty_barrel_source(source));
    }
}
