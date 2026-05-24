use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-empty-interface";
const MESSAGE: &str = "Use a type alias instead of an empty interface.";

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
            _ if starts_interface(bytes, cursor.index) => {
                let saved_cursor = cursor;
                cursor.index += b"interface".len();
                cursor.column += b"interface".len();

                if is_empty_interface(bytes, &mut cursor) {
                    violations.push(interface_violation(file, &saved_cursor, config.severity));
                }
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

fn is_empty_interface(bytes: &[u8], cursor: &mut Cursor) -> bool {
    skip_whitespace_and_comments(bytes, cursor);

    if !is_identifier_byte(bytes.get(cursor.index).copied()) {
        return false;
    }

    while cursor.index < bytes.len() && is_identifier_byte(bytes.get(cursor.index).copied()) {
        cursor.advance(bytes);
    }

    skip_whitespace_and_comments(bytes, cursor);

    if bytes.get(cursor.index) != Some(&b'{') {
        return false;
    }
    cursor.advance(bytes);

    skip_whitespace_and_comments(bytes, cursor);

    if bytes.get(cursor.index) != Some(&b'}') {
        return false;
    }

    true
}

fn skip_whitespace_and_comments(bytes: &[u8], cursor: &mut Cursor) {
    loop {
        while cursor.index < bytes.len() && bytes[cursor.index].is_ascii_whitespace() {
            cursor.advance(bytes);
        }

        if cursor.index >= bytes.len() {
            break;
        }

        if bytes.get(cursor.index) == Some(&b'/') {
            match bytes.get(cursor.index + 1) {
                Some(b'/') => skip_line_comment(bytes, cursor),
                Some(b'*') => skip_block_comment(bytes, cursor),
                _ => break,
            }
        } else {
            break;
        }
    }
}

fn interface_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
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

fn starts_interface(bytes: &[u8], index: usize) -> bool {
    starts_keyword(bytes, index, b"interface")
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

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_empty_interface() {
        let violations = check_file(
            Path::new("types.ts"),
            "interface Empty {}\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_empty_interface_with_newline() {
        let source = "interface Empty {\n}\n";
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_empty_interface_with_whitespace() {
        let source = "interface Empty {   }\n";
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_multiple_empty_interfaces() {
        let source = r#"interface A {}
interface B {}
"#;
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    }

    #[test]
    fn allows_interface_with_members() {
        let source = "interface User { name: string }\n";
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_interface_with_comment_in_body() {
        let source = "interface User { /* todo */ name: string }\n";
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_interface_in_comments_and_strings() {
        let source = r#"// interface Empty {}
const text = "interface Empty {}";
/* interface Empty {} */
"#;
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragments() {
        let source = r#"const interfacex = true;
"#;
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
