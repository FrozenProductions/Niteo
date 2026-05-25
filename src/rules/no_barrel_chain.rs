use std::path::{Component, Path, PathBuf};

use crate::config::{RuleConfig, Severity};
use crate::rules::{NO_BARREL_CHAIN_RULE_ID, Violation};
const MESSAGE: &str = "Barrel files cannot re-export from other barrel files.";
const BARREL_FILE_NAME: &str = "index.ts";
const TYPESCRIPT_EXTENSIONS: &[&str] = &["ts", "tsx"];

pub fn check_file(
    file: &Path,
    source: &str,
    files: &[PathBuf],
    config: &RuleConfig,
) -> Vec<Violation> {
    if !is_barrel_file(file) {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut cursor = Cursor::default();
    let mut violations = Vec::new();

    while cursor.index < bytes.len() {
        skip_trivia(bytes, &mut cursor);

        if cursor.index >= bytes.len() {
            break;
        }

        let statement_start = cursor;
        let statement = read_statement(bytes, &mut cursor);
        let Some(specifier) = re_export_specifier(statement) else {
            continue;
        };

        if resolves_to_barrel(file, specifier, files) {
            violations.push(barrel_chain_violation(
                file,
                &statement_start,
                config.severity,
                specifier,
            ));
        }
    }

    violations
}

#[derive(Debug, Clone, Copy)]
struct Cursor {
    index: usize,
    line: usize,
    column: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            index: 0,
            line: 1,
            column: 1,
        }
    }
}

impl Cursor {
    fn advance(&mut self, bytes: &[u8]) {
        if bytes[self.index] == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.index += 1;
    }
}

fn barrel_chain_violation(
    file: &Path,
    cursor: &Cursor,
    severity: Severity,
    specifier: &str,
) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: NO_BARREL_CHAIN_RULE_ID,
        message: MESSAGE,
        severity,
        detail: Some(format!(
            "Re-export target '{specifier}' resolves to another index.ts barrel."
        )),
        subject: Some(specifier.to_owned()),
    }
}

fn is_barrel_file(file: &Path) -> bool {
    file.file_name().and_then(|name| name.to_str()) == Some(BARREL_FILE_NAME)
}

fn resolves_to_barrel(file: &Path, specifier: &str, files: &[PathBuf]) -> bool {
    if !specifier.starts_with('.') {
        return false;
    }

    let Some(parent) = file.parent() else {
        return false;
    };

    let target = normalize_path(&parent.join(specifier));
    files
        .iter()
        .filter(|candidate| is_barrel_file(candidate))
        .any(|candidate| {
            let normalized_candidate = normalize_path(candidate);
            normalized_candidate == target
                || extensionless(&normalized_candidate) == target
                || normalized_candidate.parent() == Some(target.as_path())
        })
}

fn extensionless(path: &Path) -> PathBuf {
    if TYPESCRIPT_EXTENSIONS.contains(&path.extension().and_then(|ext| ext.to_str()).unwrap_or(""))
    {
        return path.with_extension("");
    }

    path.to_path_buf()
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

fn skip_trivia(bytes: &[u8], cursor: &mut Cursor) {
    loop {
        while cursor.index < bytes.len() && bytes[cursor.index].is_ascii_whitespace() {
            cursor.advance(bytes);
        }

        if starts_with(bytes, cursor.index, b"//") {
            skip_line_comment(bytes, cursor);
            continue;
        }

        if starts_with(bytes, cursor.index, b"/*") {
            skip_block_comment(bytes, cursor);
            continue;
        }

        break;
    }
}

fn read_statement<'a>(bytes: &'a [u8], cursor: &mut Cursor) -> &'a [u8] {
    let start = cursor.index;
    let mut string_quote: Option<u8> = None;
    let mut brace_depth = 0usize;

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];
        let next = bytes.get(cursor.index + 1).copied();

        if let Some(quote) = string_quote {
            if current == b'\\' {
                cursor.advance(bytes);
                if cursor.index < bytes.len() {
                    cursor.advance(bytes);
                }
                continue;
            }

            if current == quote {
                string_quote = None;
            }

            cursor.advance(bytes);
            continue;
        }

        match (current, next) {
            (b'\'', _) | (b'"', _) | (b'`', _) => {
                string_quote = Some(current);
                cursor.advance(bytes);
            }
            (b'/', Some(b'/')) => skip_line_comment(bytes, cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, cursor),
            (b'{', _) => {
                brace_depth += 1;
                cursor.advance(bytes);
            }
            (b'}', _) => {
                brace_depth = brace_depth.saturating_sub(1);
                cursor.advance(bytes);
            }
            (b';', _) => {
                cursor.advance(bytes);
                break;
            }
            (b'\n', _) if brace_depth == 0 => {
                cursor.advance(bytes);
                break;
            }
            _ => cursor.advance(bytes),
        }
    }

    &bytes[start..cursor.index]
}

fn re_export_specifier(statement: &[u8]) -> Option<&str> {
    let mut scanner = TokenScanner::new(statement);

    if scanner.next_token()? != "export" {
        return None;
    }

    if scanner.peek_token() == Some("type") {
        scanner.next_token();
    }

    match scanner.next_token()? {
        "*" | "{" => {}
        _ => return None,
    }

    while let Some(token) = scanner.next_token() {
        if token == "from" {
            return scanner.next_string_literal();
        }
    }

    None
}

fn skip_line_comment(bytes: &[u8], cursor: &mut Cursor) {
    while cursor.index < bytes.len() && bytes[cursor.index] != b'\n' {
        cursor.advance(bytes);
    }
}

fn skip_block_comment(bytes: &[u8], cursor: &mut Cursor) {
    cursor.advance(bytes);
    cursor.advance(bytes);

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];
        let next = bytes.get(cursor.index + 1).copied();

        cursor.advance(bytes);

        if current == b'*' && next == Some(b'/') {
            cursor.advance(bytes);
            break;
        }
    }
}

fn starts_with(bytes: &[u8], index: usize, pattern: &[u8]) -> bool {
    bytes.get(index..index + pattern.len()) == Some(pattern)
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

    fn peek_token(&mut self) -> Option<&'a str> {
        let index = self.index;
        let token = self.next_token();
        self.index = index;
        token
    }

    fn next_string_literal(&mut self) -> Option<&'a str> {
        while self.index < self.source.len() {
            let quote = self.source[self.index];
            if !matches!(quote, b'\'' | b'"') {
                self.index += 1;
                continue;
            }

            let start = self.index + 1;
            self.index += 1;

            while self.index < self.source.len() {
                let current = self.source[self.index];

                if current == b'\\' {
                    self.index += 1;
                    if self.index < self.source.len() {
                        self.index += 1;
                    }
                    continue;
                }

                if current == quote {
                    let literal = std::str::from_utf8(&self.source[start..self.index]).ok();
                    self.index += 1;
                    return literal;
                }

                self.index += 1;
            }
        }

        None
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
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::{Path, PathBuf};

    #[test]
    fn reports_re_export_from_folder_barrel() {
        let files = test_files(&["src/components/index.ts", "src/components/button/index.ts"]);
        let violations = check_file(
            Path::new("src/components/index.ts"),
            "export { Button } from './button';\n",
            &files,
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
        assert_eq!(violations[0].subject.as_deref(), Some("./button"));
    }

    #[test]
    fn reports_re_export_from_explicit_barrel_file() {
        let files = test_files(&["src/index.ts", "src/components/index.ts"]);
        let violations = check_file(
            Path::new("src/index.ts"),
            "export * from './components/index';\n",
            &files,
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_type_re_export_from_barrel() {
        let files = test_files(&["src/index.ts", "src/types/index.ts"]);
        let violations = check_file(
            Path::new("src/index.ts"),
            "export type { Props } from './types';\n",
            &files,
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_re_export_from_non_barrel_file() {
        let files = test_files(&["src/index.ts", "src/Button.ts"]);
        let violations = check_file(
            Path::new("src/index.ts"),
            "export { Button } from './Button';\n",
            &files,
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_non_barrel_files() {
        let files = test_files(&["src/Button.ts", "src/components/index.ts"]);
        let violations = check_file(
            Path::new("src/Button.ts"),
            "export { Button } from './components';\n",
            &files,
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_external_re_exports() {
        let files = test_files(&["src/index.ts"]);
        let violations = check_file(
            Path::new("src/index.ts"),
            "export { z } from 'zod';\n",
            &files,
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_comments_and_strings() {
        let files = test_files(&["src/index.ts", "src/components/index.ts"]);
        let source = r#"// export { Button } from './components';
const text = "export { Button } from './components'";
/* export { Button } from './components'; */
"#;
        let violations = check_file(Path::new("src/index.ts"), source, &files, &test_config());

        assert!(violations.is_empty());
    }

    fn test_files(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
