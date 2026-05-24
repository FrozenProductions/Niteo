use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-debugger";
const MESSAGE: &str = "Remove debugger statements before committing code.";

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
            _ if starts_debugger(bytes, cursor.index) => {
                violations.push(debugger_violation(file, &cursor, config.severity));
                advance_past_debugger(bytes, &mut cursor);
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

fn debugger_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: cursor.line,
        column: cursor.column,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn starts_debugger(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"debugger") {
        return false;
    }

    let after_debugger = index + b"debugger".len();
    let next = bytes.get(after_debugger);

    matches!(next, Some(b';' | b'\n' | b'\r' | b' ' | b'\t'))
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

fn advance_past_debugger(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"debugger".len();
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
    fn reports_debugger_statement() {
        let violations = check_file(Path::new("Component.tsx"), "debugger;\n", &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].column, 1);
    }

    #[test]
    fn reports_debugger_without_semicolon() {
        let violations = check_file(Path::new("Component.tsx"), "debugger\n", &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_debugger_with_space() {
        let violations = check_file(Path::new("Component.tsx"), "debugger \n", &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_debugger_in_comments() {
        let source = "// debugger;\n/* debugger; */\n";
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_debugger_in_strings() {
        let source = r#"const text = "debugger";"#;
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragment() {
        let source = "const debuggerHelper = true;\n";
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
