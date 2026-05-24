use std::path::Path;

use crate::config::{Severity, UpwardImportRuleConfig};
use crate::rules::Violation;

const RULE_NAME: &str = "no-upward-import";
const MESSAGE: &str = "Replace upward relative imports with local or project-root imports.";

pub fn check_file(file: &Path, source: &str, config: &UpwardImportRuleConfig) -> Vec<Violation> {
    let bytes = source.as_bytes();
    let mut violations = Vec::new();
    let mut cursor = Cursor::default();
    let mut string_quote: Option<u8> = None;

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
            (b'/', Some(b'/')) => skip_line_comment(bytes, &mut cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, &mut cursor),
            _ if starts_keyword(bytes, cursor.index, b"import") => {
                if let Some(import_cursor) = parse_import_statement(bytes, cursor.index) {
                    if should_report(import_cursor.specifier, config) {
                        violations.push(violation(file, &cursor, config.severity));
                    }
                    advance_to(bytes, &mut cursor, import_cursor.next_index);
                    continue;
                }

                cursor.advance(bytes);
            }
            _ if starts_keyword(bytes, cursor.index, b"export") => {
                if let Some(export_cursor) = parse_export_statement(bytes, cursor.index) {
                    if should_report(export_cursor.specifier, config) {
                        violations.push(violation(file, &cursor, config.severity));
                    }
                    advance_to(bytes, &mut cursor, export_cursor.next_index);
                    continue;
                }

                cursor.advance(bytes);
            }
            _ => cursor.advance(bytes),
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

struct ParsedModuleSpecifier<'a> {
    specifier: &'a [u8],
    next_index: usize,
}

fn parse_import_statement(bytes: &[u8], index: usize) -> Option<ParsedModuleSpecifier<'_>> {
    let mut scanner = StatementScanner::new(bytes, index + b"import".len());
    scanner.skip_whitespace();

    if scanner.peek_byte() == Some(b'(') {
        scanner.advance();
        scanner.skip_whitespace();
        let specifier = scanner.read_string_literal()?;
        scanner.skip_whitespace();
        if scanner.peek_byte() == Some(b')') {
            scanner.advance();
        }

        return Some(ParsedModuleSpecifier {
            specifier,
            next_index: scanner.index,
        });
    }

    if scanner.peek_byte() == Some(b'"')
        || scanner.peek_byte() == Some(b'\'')
        || scanner.peek_byte() == Some(b'`')
    {
        let specifier = scanner.read_string_literal()?;
        return Some(ParsedModuleSpecifier {
            specifier,
            next_index: scanner.index,
        });
    }

    if scanner.consume_keyword("type") {
        scanner.skip_whitespace();
    }

    if scanner.skip_until_keyword("from") {
        scanner.skip_whitespace();
        let specifier = scanner.read_string_literal()?;
        return Some(ParsedModuleSpecifier {
            specifier,
            next_index: scanner.index,
        });
    }

    None
}

fn parse_export_statement(bytes: &[u8], index: usize) -> Option<ParsedModuleSpecifier<'_>> {
    let mut scanner = StatementScanner::new(bytes, index + b"export".len());
    scanner.skip_whitespace();

    if scanner.consume_keyword("type") {
        scanner.skip_whitespace();
    }

    if !(scanner.peek_byte() == Some(b'*') || scanner.peek_byte() == Some(b'{')) {
        return None;
    }

    if scanner.skip_until_keyword("from") {
        scanner.skip_whitespace();
        let specifier = scanner.read_string_literal()?;
        return Some(ParsedModuleSpecifier {
            specifier,
            next_index: scanner.index,
        });
    }

    None
}

fn should_report(specifier: &[u8], config: &UpwardImportRuleConfig) -> bool {
    upward_depth(specifier) > config.max_depth
}

fn upward_depth(specifier: &[u8]) -> usize {
    specifier
        .split(|byte| *byte == b'/')
        .take_while(|segment| *segment == b"..")
        .count()
}

fn violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
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

fn advance_to(bytes: &[u8], cursor: &mut Cursor, next_index: usize) {
    while cursor.index < bytes.len() && cursor.index < next_index {
        cursor.advance(bytes);
    }
}

fn starts_keyword(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes.get(index..index + keyword.len()) == Some(keyword)
        && !is_identifier_byte(bytes.get(index.wrapping_sub(1)).copied())
        && !is_identifier_byte(bytes.get(index + keyword.len()).copied())
}

fn is_identifier_byte(byte: Option<u8>) -> bool {
    matches!(
        byte,
        Some(b'a'..=b'z') | Some(b'A'..=b'Z') | Some(b'0'..=b'9') | Some(b'_') | Some(b'$')
    )
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

#[derive(Debug)]
struct StatementScanner<'a> {
    source: &'a [u8],
    index: usize,
}

impl<'a> StatementScanner<'a> {
    fn new(source: &'a [u8], index: usize) -> Self {
        Self { source, index }
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.source.get(self.index),
            Some(b' ' | b'\t' | b'\r' | b'\n')
        ) {
            self.advance();
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.get(self.index).copied()
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        if !starts_keyword(self.source, self.index, keyword.as_bytes()) {
            return false;
        }

        self.index += keyword.len();
        true
    }

    fn skip_until_keyword(&mut self, keyword: &str) -> bool {
        while self.index < self.source.len() {
            if starts_keyword(self.source, self.index, keyword.as_bytes()) {
                self.index += keyword.len();
                return true;
            }

            self.index += 1;
        }

        false
    }

    fn read_string_literal(&mut self) -> Option<&'a [u8]> {
        let quote = self.peek_byte()?;
        if !matches!(quote, b'\'' | b'"' | b'`') {
            return None;
        }

        self.advance();
        let start = self.index;
        while self.index < self.source.len() {
            let current = self.source[self.index];
            if current == b'\\' {
                self.index += 2;
                continue;
            }

            if current == quote {
                let end = self.index;
                self.advance();
                return Some(&self.source[start..end]);
            }

            self.index += 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{Severity, UpwardImportRuleConfig};
    use std::path::Path;

    #[test]
    fn reports_upward_relative_imports() {
        let source = r#"import { shared } from "../../../shared";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_upward_relative_export_from() {
        let source = r#"export { shared } from "../shared";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_dynamic_upward_imports() {
        let source = r#"const shared = await import("../../shared");
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn keeps_line_positions_after_multiline_imports() {
        let source = r#"import {
  local,
} from "./local";
import { shared } from "../shared";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(4));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_same_folder_and_downward_imports() {
        let source = r#"import { value } from "./value";
export { other } from "./other";
const shared = import("shared");
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_treat_export_default_as_export_from() {
        let source = r#"export default function Component() {}
import { shared } from "../shared";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn allows_configured_upward_depth() {
        let source = r#"import { shared } from "../shared";
import { other } from "../../other";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config_with_depth(1));

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn ignores_comments_and_strings() {
        let source = r#"// import { shared } from "../shared";
const text = "export { shared } from '../shared'";
/* import x from "../shared" */
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            severity: Severity::Warn,
            max_depth: 0,
        }
    }

    fn test_config_with_depth(max_depth: usize) -> UpwardImportRuleConfig {
        UpwardImportRuleConfig {
            max_depth,
            ..test_config()
        }
    }
}
