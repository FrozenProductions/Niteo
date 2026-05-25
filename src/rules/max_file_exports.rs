use std::path::Path;

use crate::config::FileExportsRuleConfig;
use crate::rules::Violation;

const RULE_NAME: &str = "max-file-exports";
const MESSAGE: &str = "Split this file or reduce its public surface area.";

pub fn check_file(file: &Path, source: &str, config: &FileExportsRuleConfig) -> Vec<Violation> {
    let export_count = count_exports(source);
    if export_count <= config.max_exports {
        return Vec::new();
    }

    vec![Violation {
        file: file.to_path_buf(),
        line: Some(1),
        column: Some(1),
        rule: RULE_NAME,
        message: MESSAGE,
        severity: config.severity,
        detail: None,
        subject: None,
    }]
}

fn count_exports(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0usize;
    let mut cursor = Cursor::default();
    let mut string_quote: Option<u8> = None;

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];
        let next = bytes.get(cursor.index + 1).copied();

        if let Some(quote) = string_quote {
            if current == b'\\' {
                cursor.advance();
                if cursor.index < bytes.len() {
                    cursor.advance();
                }
                continue;
            }

            if current == quote {
                string_quote = None;
            }

            cursor.advance();
            continue;
        }

        match (current, next) {
            (b'\'', _) | (b'"', _) | (b'`', _) => {
                string_quote = Some(current);
                cursor.advance();
            }
            (b'/', Some(b'/')) => skip_line_comment(bytes, &mut cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, &mut cursor),
            _ if starts_keyword(bytes, cursor.index, b"export") => {
                count += count_export_statement(bytes, cursor.index);
                cursor.index += b"export".len();
            }
            _ => cursor.advance(),
        }
    }

    count
}

#[derive(Debug, Clone, Copy, Default)]
struct Cursor {
    index: usize,
}

impl Cursor {
    fn advance(&mut self) {
        self.index += 1;
    }
}

fn count_export_statement(bytes: &[u8], index: usize) -> usize {
    let mut scanner = ExportScanner::new(bytes, index + b"export".len());
    scanner.skip_whitespace();

    if scanner.consume_keyword("default") {
        return 1;
    }

    if scanner.consume_byte(b'*') {
        return 1;
    }

    if scanner.peek_byte() == Some(b'{') {
        return scanner.count_named_exports();
    }

    scanner.skip_export_modifiers();

    if scanner.peek_byte() == Some(b'{') {
        return scanner.count_named_exports();
    }

    if scanner.consume_keyword("type") {
        scanner.skip_whitespace();
        if scanner.peek_byte() == Some(b'{') {
            return scanner.count_named_exports();
        }

        return usize::from(scanner.has_identifier_before_byte(b'='));
    }

    if scanner.consume_keyword("interface")
        || scanner.consume_keyword("function")
        || scanner.consume_keyword("class")
        || scanner.consume_keyword("enum")
    {
        return usize::from(scanner.next_identifier().is_some());
    }

    if scanner.consume_keyword("const")
        || scanner.consume_keyword("let")
        || scanner.consume_keyword("var")
    {
        return scanner.count_variable_exports();
    }

    0
}

fn skip_line_comment(bytes: &[u8], cursor: &mut Cursor) {
    while cursor.index < bytes.len() && bytes[cursor.index] != b'\n' {
        cursor.advance();
    }
}

fn skip_block_comment(bytes: &[u8], cursor: &mut Cursor) {
    cursor.advance();
    cursor.advance();

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];
        let next = bytes.get(cursor.index + 1).copied();

        cursor.advance();

        if current == b'*' && next == Some(b'/') {
            cursor.advance();
            break;
        }
    }
}

fn starts_keyword(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes.get(index..index + keyword.len()) == Some(keyword)
        && !is_identifier_byte(bytes.get(index.wrapping_sub(1)).copied())
        && !is_identifier_byte(bytes.get(index + keyword.len()).copied())
}

fn is_identifier_byte(byte: Option<u8>) -> bool {
    byte.is_some_and(is_identifier_part)
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

fn is_identifier_part(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

#[derive(Debug)]
struct ExportScanner<'a> {
    source: &'a [u8],
    index: usize,
}

impl<'a> ExportScanner<'a> {
    fn new(source: &'a [u8], index: usize) -> Self {
        Self { source, index }
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.source.get(self.index),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            self.index += 1;
        }
    }

    fn skip_export_modifiers(&mut self) {
        loop {
            self.skip_whitespace();
            if self.consume_keyword("declare")
                || self.consume_keyword("abstract")
                || self.consume_keyword("async")
            {
                continue;
            }

            break;
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        if !starts_keyword(self.source, self.index, keyword.as_bytes()) {
            return false;
        }

        self.index += keyword.len();
        true
    }

    fn consume_byte(&mut self, byte: u8) -> bool {
        if self.source.get(self.index) != Some(&byte) {
            return false;
        }

        self.index += 1;
        true
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.get(self.index).copied()
    }

    fn next_identifier(&mut self) -> Option<&'a [u8]> {
        self.skip_whitespace();
        let start = self.index;
        let first = self.source.get(start).copied()?;
        if !is_identifier_start(first) {
            return None;
        }

        self.index += 1;
        while self.index < self.source.len() && is_identifier_part(self.source[self.index]) {
            self.index += 1;
        }

        Some(&self.source[start..self.index])
    }

    fn has_identifier_before_byte(&mut self, byte: u8) -> bool {
        self.next_identifier().is_some() && {
            self.skip_whitespace();
            self.source.get(self.index) == Some(&byte)
        }
    }

    fn count_named_exports(&mut self) -> usize {
        if !self.consume_byte(b'{') {
            return 0;
        }

        let mut count = 0usize;
        let mut has_specifier = false;
        let mut brace_depth = 1usize;
        let mut string_quote: Option<u8> = None;

        while self.index < self.source.len() && brace_depth > 0 {
            let current = self.source[self.index];
            let next = self.source.get(self.index + 1).copied();

            if let Some(quote) = string_quote {
                self.index += escaped_step(current);
                if current == quote {
                    string_quote = None;
                }
                continue;
            }

            match (current, next) {
                (b'\'', _) | (b'"', _) | (b'`', _) => {
                    string_quote = Some(current);
                    self.index += 1;
                }
                (b'{', _) => {
                    brace_depth += 1;
                    self.index += 1;
                }
                (b'}', _) => {
                    if brace_depth == 1 && has_specifier {
                        count += 1;
                    }
                    brace_depth = brace_depth.saturating_sub(1);
                    self.index += 1;
                }
                (b',', _) if brace_depth == 1 => {
                    if has_specifier {
                        count += 1;
                    }
                    has_specifier = false;
                    self.index += 1;
                }
                _ if brace_depth == 1 && is_identifier_start(current) => {
                    let Some(identifier) = self.next_identifier() else {
                        continue;
                    };
                    if identifier != b"type" {
                        has_specifier = true;
                    }
                }
                _ => self.index += 1,
            }
        }

        count
    }

    fn count_variable_exports(&mut self) -> usize {
        let mut count = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut paren_depth = 0usize;
        let mut expects_binding = true;
        let mut string_quote: Option<u8> = None;

        while self.index < self.source.len() {
            let current = self.source[self.index];
            let next = self.source.get(self.index + 1).copied();

            if let Some(quote) = string_quote {
                self.index += escaped_step(current);
                if current == quote {
                    string_quote = None;
                }
                continue;
            }

            match (current, next) {
                (b'\'', _) | (b'"', _) | (b'`', _) => {
                    string_quote = Some(current);
                    self.index += 1;
                }
                (b'{', _) => {
                    brace_depth += 1;
                    self.index += 1;
                }
                (b'}', _) => {
                    brace_depth = brace_depth.saturating_sub(1);
                    self.index += 1;
                }
                (b'[', _) => {
                    bracket_depth += 1;
                    self.index += 1;
                }
                (b']', _) => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    self.index += 1;
                }
                (b'(', _) => {
                    paren_depth += 1;
                    self.index += 1;
                }
                (b')', _) => {
                    paren_depth = paren_depth.saturating_sub(1);
                    self.index += 1;
                }
                (b';' | b'\n', _) if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 => {
                    break;
                }
                (b',', _) if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 => {
                    expects_binding = true;
                    self.index += 1;
                }
                _ if expects_binding && is_identifier_start(current) => {
                    count += 1;
                    expects_binding = false;
                    self.index += 1;
                    while self.index < self.source.len()
                        && is_identifier_part(self.source[self.index])
                    {
                        self.index += 1;
                    }
                }
                _ => self.index += 1,
            }
        }

        count
    }
}

fn escaped_step(current: u8) -> usize {
    if current == b'\\' { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{FileExportsRuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_files_with_too_many_named_export_declarations() {
        let source = r#"export const one = 1;
export function two() {}
export class Three {}
export interface Four {}
"#;
        let violations = check_file(Path::new("dump.ts"), source, &test_config(3));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn counts_named_export_lists() {
        let source = r#"const one = 1;
const two = 2;
const three = 3;
export { one, two as renamedTwo, type three };
"#;
        let violations = check_file(Path::new("dump.ts"), source, &test_config(2));

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn counts_multiple_variable_exports() {
        let violations = check_file(
            Path::new("values.ts"),
            "export const one = 1, two = 2, three = 3;\n",
            &test_config(2),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_files_within_limit() {
        let source = r#"export const one = 1;
export { two, three };
"#;
        let violations = check_file(Path::new("small.ts"), source, &test_config(3));

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_exports_in_comments_and_strings() {
        let source = r#"const text = "export const one = 1";
// export const two = 2;
/* export const three = 3; */
export const four = 4;
"#;
        let violations = check_file(Path::new("small.ts"), source, &test_config(1));

        assert!(violations.is_empty());
    }

    #[test]
    fn counts_default_and_namespace_exports() {
        let source = r#"export default value;
export * from "./other";
export * as names from "./names";
"#;
        let violations = check_file(Path::new("barrel.ts"), source, &test_config(2));

        assert_eq!(violations.len(), 1);
    }

    fn test_config(max_exports: usize) -> FileExportsRuleConfig {
        FileExportsRuleConfig {
            severity: Severity::Warn,
            max_exports,
        }
    }
}
