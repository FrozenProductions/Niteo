use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-mutable-exports";
const MESSAGE: &str = "Only export const, never export let.";

pub fn check_file(file: &Path, source: &str, config: &RuleConfig) -> Vec<Violation> {
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
            _ if starts_export_let(bytes, cursor.index) => {
                violations.push(mutable_export_violation(file, &cursor, config.severity));
                advance_past_export_let(bytes, &mut cursor);
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

fn mutable_export_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
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

fn starts_export_let(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"export") {
        return false;
    }

    let mut next_index = index + b"export".len();
    next_index = skip_inline_whitespace(bytes, next_index);

    if !starts_keyword(bytes, next_index, b"let") {
        return false;
    }

    let after_let = next_index + b"let".len();
    let next_byte = bytes.get(after_let).copied();
    !is_identifier_byte(next_byte)
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

fn skip_inline_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        index += 1;
    }

    index
}

fn advance_past_export_let(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"export".len() + b"let".len();
    while cursor.index < bytes.len() && cursor.index < target_index {
        cursor.advance(bytes);
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

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_export_let() {
        let violations = check_file(
            Path::new("value.ts"),
            "export let count = 0;\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_multiline_export_let() {
        let violations = check_file(
            Path::new("value.ts"),
            "export\n  let count = 0;\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_export_const() {
        let violations = check_file(
            Path::new("value.ts"),
            "export const count = 0;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_named_function_export() {
        let violations = check_file(
            Path::new("value.ts"),
            "export function foo() {}\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_export_let_in_comments_and_strings() {
        let source = r#"// export let count = 0;
const text = "export let count = 0";
/* export let count = 0; */
"#;
        let violations = check_file(Path::new("value.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_let_inside_expression() {
        let source = r#"const exportLet = true;
const value = "before export let after";
"#;
        let violations = check_file(Path::new("value.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_export_letting() {
        let source = "export letting foo = 1;\n";
        let violations = check_file(Path::new("value.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
