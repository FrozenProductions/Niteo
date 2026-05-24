use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-logic-in-barrel";
const MESSAGE: &str = "Keep barrel files limited to re-exports.";
const BARREL_FILE_NAME: &str = "index.ts";

pub fn check_file(file: &Path, source: &str, config: &RuleConfig) -> Vec<Violation> {
    if file.file_name().and_then(|name| name.to_str()) != Some(BARREL_FILE_NAME) {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut cursor = Cursor::default();

    while cursor.index < bytes.len() {
        skip_trivia(bytes, &mut cursor);

        if cursor.index >= bytes.len() {
            break;
        }

        let statement_start = cursor;
        let statement = read_statement(bytes, &mut cursor);
        if !is_re_export(statement) {
            return vec![barrel_violation(file, &statement_start, config.severity)];
        }
    }

    Vec::new()
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

fn barrel_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
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

fn is_re_export(statement: &[u8]) -> bool {
    let mut scanner = TokenScanner::new(statement);

    if scanner.next_token() != Some("export") {
        return false;
    }

    if scanner.peek_token() == Some("type") {
        scanner.next_token();
    }

    match scanner.next_token() {
        Some("*") => scanner.contains_token("from"),
        Some("{") => scanner.contains_token("from"),
        _ => false,
    }
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

    fn contains_token(&mut self, expected: &str) -> bool {
        while let Some(token) = self.next_token() {
            if token == expected {
                return true;
            }
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
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn allows_named_re_exports() {
        let source = r#"export { Button } from "./Button";
export type { ButtonProps } from "./Button.type";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_namespace_re_exports() {
        let source = r#"export * from "./Button";
export * as ButtonParts from "./Button.parts";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_multiline_re_exports() {
        let source = r#"export {
  Button,
  type ButtonProps,
} from "./Button";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_non_barrel_files() {
        let source = r#"const value = 1;"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_imports_in_barrels() {
        let source = r#"import { Button } from "./Button";
export { Button };
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_local_exports_in_barrels() {
        let source = r#"export { Button };
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_logic_after_re_exports() {
        let source = r#"export { Button } from "./Button";

const value = 1;
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
