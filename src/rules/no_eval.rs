use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::{NO_EVAL_RULE_ID, Violation};
const MESSAGE: &str = "Disallow eval() and new Function() as they execute arbitrary code.";

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
            _ if starts_eval_call(bytes, cursor.index) => {
                violations.push(eval_violation(file, &cursor, config.severity));
                advance_past_eval(bytes, &mut cursor);
            }
            _ if starts_new_function(bytes, cursor.index) => {
                violations.push(eval_violation(file, &cursor, config.severity));
                advance_past_new_function(bytes, &mut cursor);
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

fn eval_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: NO_EVAL_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn starts_eval_call(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"eval") {
        return false;
    }

    let after_eval = index + b"eval".len();
    let next = bytes.get(after_eval);

    matches!(next, Some(b'('))
}

fn starts_new_function(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"new") {
        return false;
    }

    let after_new = index + b"new".len();
    let next_index = skip_inline_whitespace(bytes, after_new);

    if !starts_keyword(bytes, next_index, b"Function") {
        return false;
    }

    let after_function = next_index + b"Function".len();
    let next = bytes.get(after_function);

    matches!(next, Some(b'('))
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

fn advance_past_eval(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"eval".len();
    while cursor.index < bytes.len() && cursor.index < target_index {
        cursor.advance(bytes);
    }
}

fn advance_past_new_function(bytes: &[u8], cursor: &mut Cursor) {
    let after_new = cursor.index + b"new".len();
    let next_index = skip_inline_whitespace_in_cursor(bytes, cursor, after_new);

    let target_index = next_index + b"Function".len();
    while cursor.index < bytes.len() && cursor.index < target_index {
        cursor.advance(bytes);
    }
}

fn skip_inline_whitespace_in_cursor(bytes: &[u8], cursor: &mut Cursor, target: usize) -> usize {
    while cursor.index < bytes.len() && cursor.index < target {
        cursor.advance(bytes);
    }
    cursor.index
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
    fn reports_eval_call() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "eval('code');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn reports_new_function() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "new Function('return 1');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_new_function_with_space() {
        let violations = check_file(
            Path::new("Component.tsx"),
            "new  Function('return 1');\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_eval_in_comments() {
        let source = "// eval('code');\n/* new Function('test'); */\n";
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_eval_in_strings() {
        let source = r#"const text = "eval('hello')";"#;
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_identifier_fragment() {
        let source = "const evaluate = true;\nconst FunctionBuilder = class {};\n";
        let violations = check_file(Path::new("Component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
