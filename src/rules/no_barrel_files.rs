use std::path::Path;

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-barrel-files";
const MESSAGE: &str = "Avoid barrel files; import directly from the source module.";
const BARREL_FILE_NAME: &str = "index.ts";

pub fn check_file(file: &Path, source: &str, config: &RuleConfig) -> Vec<Violation> {
    if file.file_name().and_then(|name| name.to_str()) != Some(BARREL_FILE_NAME) {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut index = 0;
    let mut has_re_export = false;

    while index < bytes.len() {
        skip_trivia(bytes, &mut index);

        if index >= bytes.len() {
            break;
        }

        let statement = read_statement(bytes, &mut index);
        if is_re_export(statement) {
            has_re_export = true;
        }
    }

    if has_re_export {
        vec![barrel_violation(file, config.severity)]
    } else {
        Vec::new()
    }
}

fn barrel_violation(file: &Path, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: None,
        column: None,
        rule: RULE_NAME,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
}

fn skip_trivia(bytes: &[u8], index: &mut usize) {
    loop {
        while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
            *index += 1;
        }

        if starts_with(bytes, *index, b"//") {
            skip_line_comment(bytes, index);
            continue;
        }

        if starts_with(bytes, *index, b"/*") {
            skip_block_comment(bytes, index);
            continue;
        }

        break;
    }
}

fn read_statement<'a>(bytes: &'a [u8], index: &mut usize) -> &'a [u8] {
    let start = *index;
    let mut string_quote: Option<u8> = None;
    let mut brace_depth = 0usize;

    while *index < bytes.len() {
        let current = bytes[*index];
        let next = bytes.get(*index + 1).copied();

        if let Some(quote) = string_quote {
            if current == b'\\' {
                *index += 1;
                if *index < bytes.len() {
                    *index += 1;
                }
                continue;
            }

            if current == quote {
                string_quote = None;
            }

            *index += 1;
            continue;
        }

        match (current, next) {
            (b'\'', _) | (b'"', _) | (b'`', _) => {
                string_quote = Some(current);
                *index += 1;
            }
            (b'/', Some(b'/')) => skip_line_comment(bytes, index),
            (b'/', Some(b'*')) => skip_block_comment(bytes, index),
            (b'{', _) => {
                brace_depth += 1;
                *index += 1;
            }
            (b'}', _) => {
                brace_depth = brace_depth.saturating_sub(1);
                *index += 1;
            }
            (b';', _) => {
                *index += 1;
                break;
            }
            (b'\n', _) if brace_depth == 0 => {
                *index += 1;
                break;
            }
            _ => *index += 1,
        }
    }

    &bytes[start..*index]
}

fn is_re_export(statement: &[u8]) -> bool {
    let mut scanner = TokenScanner::new(statement);

    if scanner.next_token() != Some("export") {
        return false;
    }

    if scanner.peek_token() == Some("type") {
        scanner.next_token();
    }

    match scanner.next_token() {
        Some("*") => scanner.contains_token("from"),
        Some("{") => scanner.contains_token("from"),
        _ => false,
    }
}

fn skip_line_comment(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && bytes[*index] != b'\n' {
        *index += 1;
    }
}

fn skip_block_comment(bytes: &[u8], index: &mut usize) {
    *index += 1;
    *index += 1;

    while *index < bytes.len() {
        let current = bytes[*index];
        let next = bytes.get(*index + 1).copied();

        *index += 1;

        if current == b'*' && next == Some(b'/') {
            *index += 1;
            break;
        }
    }
}

fn starts_with(bytes: &[u8], index: usize, pattern: &[u8]) -> bool {
    bytes.get(index..index + pattern.len()) == Some(pattern)
}

#[derive(Debug)]
struct TokenScanner<'a> {
    source: &'a [u8],
    index: usize,
}

impl<'a> TokenScanner<'a> {
    fn new(source: &'a [u8]) -> Self {
        Self { source, index: 0 }
    }

    fn next_token(&mut self) -> Option<&'a str> {
        self.skip_non_tokens();

        if self.index >= self.source.len() {
            return None;
        }

        let start = self.index;
        if self.source[self.index].is_ascii_alphabetic() {
            while self.index < self.source.len() && self.source[self.index].is_ascii_alphabetic() {
                self.index += 1;
            }
        } else {
            self.index += 1;
        }

        std::str::from_utf8(&self.source[start..self.index]).ok()
    }

    fn peek_token(&mut self) -> Option<&'a str> {
        let index = self.index;
        let token = self.next_token();
        self.index = index;
        token
    }

    fn contains_token(&mut self, expected: &str) -> bool {
        while let Some(token) = self.next_token() {
            if token == expected {
                return true;
            }
        }

        false
    }

    fn skip_non_tokens(&mut self) {
        while self.index < self.source.len() {
            if self.source[self.index].is_ascii_alphanumeric()
                || matches!(self.source[self.index], b'{' | b'}' | b'*')
            {
                break;
            }

            self.index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_barrel_file_with_re_exports() {
        let source = r#"export { Button } from "./Button";
export type { ButtonProps } from "./Button.type";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert!(violations[0].line.is_none());
        assert!(violations[0].column.is_none());
    }

    #[test]
    fn reports_barrel_file_with_namespace_re_exports() {
        let source = r#"export * from "./Button";
export * as ButtonParts from "./Button.parts";
"#;
        let violations = check_file(Path::new("index.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_non_barrel_files() {
        let source = r#"export { Button } from "./Button";
"#;
        let violations = check_file(Path::new("Button.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_index_file_without_re_exports() {
        let source = r#"const value = 1;
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
