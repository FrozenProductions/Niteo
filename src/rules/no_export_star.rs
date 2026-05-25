use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::{NO_EXPORT_STAR_RULE_ID, Violation};
const MESSAGE: &str =
    "Avoid export * because it hides the public API shape. Use explicit named re-exports.";

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
            _ if starts_export_star(bytes, cursor.index) => {
                violations.push(export_star_violation(file, &cursor, config.severity));
                advance_past_export_star(bytes, &mut cursor);
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

fn export_star_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: NO_EXPORT_STAR_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn starts_export_star(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"export") {
        return false;
    }

    let mut next_index = index + b"export".len();
    next_index = skip_inline_whitespace(bytes, next_index);

    if !starts_keyword(bytes, next_index, b"*") {
        return false;
    }

    let after_star = next_index + 1;
    let after_ws = skip_inline_whitespace(bytes, after_star);

    if starts_keyword(bytes, after_ws, b"as") {
        return false;
    }

    let next_byte = bytes.get(after_star).copied();
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

fn advance_past_export_star(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"export".len() + 1;
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
    fn reports_export_star() {
        let violations = check_file(
            Path::new("index.ts"),
            "export * from './module';\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_multiline_export_star() {
        let violations = check_file(
            Path::new("index.ts"),
            "export\n  * from './module';\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_named_re_exports() {
        let violations = check_file(
            Path::new("index.ts"),
            "export { foo, bar } from './module';\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_namespace_re_exports() {
        let violations = check_file(
            Path::new("index.ts"),
            "export * as utils from './utils';\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_export_star_in_comments_and_strings() {
        let source = r#"// export * from './module';
const text = "export * from './module'";
/* export * from './module'; */
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_star_in_expression() {
        let source = r#"const exportStar = true;
const value = "before export * after";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
