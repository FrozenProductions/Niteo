use std::path::Path;

use crate::config::{CommentsRuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-comments";
const MESSAGE: &str = "Remove implementation comments or convert them to allowed documentation.";

pub fn check_file(file: &Path, source: &str, config: &CommentsRuleConfig) -> Vec<Violation> {
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
            (b'/', Some(b'/')) => {
                if should_report_line_comment(bytes, cursor.index, config) {
                    violations.push(comment_violation(file, &cursor, config.severity));
                }
                skip_line_comment(bytes, &mut cursor);
            }
            (b'/', Some(b'*')) => {
                if should_report_block_comment(bytes, cursor.index, config) {
                    violations.push(comment_violation(file, &cursor, config.severity));
                }
                skip_block_comment(bytes, &mut cursor);
            }
            _ => {
                cursor.advance(bytes);
            }
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

fn comment_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: cursor.line,
        column: cursor.column,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
    }
}

fn should_report_line_comment(bytes: &[u8], index: usize, config: &CommentsRuleConfig) -> bool {
    !config.allow_doc_comments || !is_doc_line_comment(bytes, index)
}

fn should_report_block_comment(bytes: &[u8], index: usize, config: &CommentsRuleConfig) -> bool {
    !config.allow_doc_comments || !is_doc_block_comment(bytes, index)
}

fn is_doc_line_comment(bytes: &[u8], index: usize) -> bool {
    bytes.get(index + 2) == Some(&b'/')
}

fn is_doc_block_comment(bytes: &[u8], index: usize) -> bool {
    bytes.get(index + 2) == Some(&b'*')
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
    use crate::config::{CommentsRuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn finds_line_comments() {
        let violations = check_file(
            Path::new("example.ts"),
            "const value = 1 // no\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].column, 17);
    }

    #[test]
    fn finds_block_comments() {
        let violations = check_file(
            Path::new("example.ts"),
            "const value = /* no */ 1\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].column, 15);
    }

    #[test]
    fn finds_tsx_comments() {
        let source = "export function View() {\n  return <div>{/* no */}</div>\n}\n";
        let violations = check_file(Path::new("example.tsx"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 2);
        assert_eq!(violations[0].column, 16);
    }

    #[test]
    fn ignores_comment_markers_inside_strings() {
        let source = r#"const url = "https://example.com"
const block = "/* not a comment */"
"#;
        let violations = check_file(Path::new("example.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_doc_comments_when_allowed() {
        let source = "/// User model\n/** Component docs */\nconst value = 1\n";
        let violations = check_file(Path::new("example.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_doc_comments_when_disallowed() {
        let config = CommentsRuleConfig {
            severity: Severity::Warn,
            allow_doc_comments: false,
        };
        let source = "/// User model\n/** Component docs */\nconst value = 1\n";
        let violations = check_file(Path::new("example.ts"), source, &config);

        assert_eq!(violations.len(), 2);
    }

    fn test_config() -> CommentsRuleConfig {
        CommentsRuleConfig {
            severity: Severity::Warn,
            allow_doc_comments: true,
        }
    }
}
