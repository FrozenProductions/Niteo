use std::path::Path;

use crate::config::NoConsoleRuleConfig;
use crate::rules::Violation;

const RULE_NAME: &str = "no-console";
const MESSAGE: &str = "Disallow console statements outside allowed file patterns.";

pub fn check_file(file: &Path, source: &str, config: &NoConsoleRuleConfig) -> Vec<Violation> {
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
            _ if starts_console_call(bytes, cursor.index) => {
                if !is_file_allowed(file, config) {
                    violations.push(console_violation(file, &cursor, config.severity));
                }
                advance_past_console(bytes, &mut cursor);
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

fn console_violation(file: &Path, cursor: &Cursor, severity: crate::config::Severity) -> Violation {
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

fn starts_console_call(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"console") {
        return false;
    }

    let after_console = index + b"console".len();
    let next = bytes.get(after_console);

    matches!(next, Some(b'.' | b'['))
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

fn advance_past_console(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"console".len();
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

fn is_file_allowed(file: &Path, config: &NoConsoleRuleConfig) -> bool {
    let file_str = file.to_string_lossy();
    for pattern in &config.allow_patterns {
        if file_str.contains(pattern) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{NoConsoleRuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_console_log() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "console.log('hello');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_console_warn() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "console.warn('warning');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_console_error() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "console.error('error');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_console_in_service_files() {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec![".service.ts".to_string()],
        };

        let violations = check_file(
            Path::new("api.service.ts"),
            "console.log('hello');\n",
            &config,
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_console_in_comments() {
        let source = "// console.log('hello');\n/* console.warn('test'); */\n";
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_console_in_strings() {
        let source = r#"const text = "console.log('hello')";"#;
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> NoConsoleRuleConfig {
        NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec![],
        }
    }
}
