use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::{PREFER_SATISFIES_RULE_ID, Violation};
const MESSAGE: &str =
    "Prefer 'satisfies' over 'as' for type validation without changing the inferred type.";

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
            _ if starts_as_cast(bytes, cursor.index) => {
                if should_prefer_satisfies(bytes, cursor.index) {
                    violations.push(as_cast_violation(file, &cursor, config.severity));
                }
                advance_past_as(bytes, &mut cursor);
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

fn as_cast_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: PREFER_SATISFIES_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn starts_as_cast(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"as") {
        return false;
    }

    let after_as = index + b"as".len();
    let next_byte = bytes.get(after_as).copied();
    !is_identifier_byte(next_byte)
}

fn should_prefer_satisfies(bytes: &[u8], as_index: usize) -> bool {
    let after_as = as_index + b"as".len();
    let next_index = skip_inline_whitespace(bytes, after_as);

    if starts_keyword(bytes, next_index, b"const")
        || starts_keyword(bytes, next_index, b"any")
        || starts_keyword(bytes, next_index, b"unknown")
    {
        return false;
    }

    let before_as = as_index.wrapping_sub(1);
    let prev_index = skip_trailing_whitespace(bytes, before_as);

    matches!(
        bytes.get(prev_index).copied(),
        Some(b'}') | Some(b']') | Some(b'"') | Some(b'\'') | Some(b'`') | Some(b'0'..=b'9')
    )
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

fn skip_trailing_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index > 0 && matches!(bytes.get(index), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        index -= 1;
    }

    index
}

fn advance_past_as(bytes: &[u8], cursor: &mut Cursor) {
    let target_index = cursor.index + b"as".len();
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
    fn reports_object_literal_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const config = { port: 3000 } as Config;\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn reports_array_literal_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const items = [1, 2, 3] as number[];\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_string_literal_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const event = \"click\" as EventName;\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_numeric_literal_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const code = 404 as StatusCode;\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_as_const() {
        let violations = check_file(
            Path::new("value.ts"),
            "const config = { port: 3000 } as const;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_as_any() {
        let violations = check_file(
            Path::new("value.ts"),
            "const value = something as any;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_as_unknown() {
        let violations = check_file(
            Path::new("value.ts"),
            "const value = something as unknown as Target;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_variable_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const value = someVar as Target;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_function_call_as_cast() {
        let violations = check_file(
            Path::new("value.ts"),
            "const result = getData() as Response;\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_as_in_comments_and_strings() {
        let source = r#"// const x = {} as Config;
const text = "as Config";
/* const x = {} as Config; */
"#;
        let violations = check_file(Path::new("value.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn does_not_match_as_in_identifier() {
        let source = "const task = 'hello';\n";
        let violations = check_file(Path::new("value.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
