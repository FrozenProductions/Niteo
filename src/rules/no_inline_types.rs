use std::path::{Component, Path, PathBuf};

use crate::config::{RuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-inline-types";
const TYPES_DIRECTORY_NAME: &str = "types";
const TYPE_FILE_SUFFIX: &str = ".type.ts";
const DECLARATION_FILE_SUFFIX: &str = ".d.ts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeLocationStyle {
    allows_type_files: bool,
    allows_types_directories: bool,
}

impl TypeLocationStyle {
    pub fn detect(files: &[PathBuf]) -> Self {
        let allows_type_files = files.iter().any(|file| is_type_file(file));
        let allows_types_directories = files.iter().any(|file| is_in_types_directory(file));

        Self {
            allows_type_files: allows_type_files || !allows_types_directories,
            allows_types_directories,
        }
    }

    fn allows_file(self, file: &Path) -> bool {
        is_declaration_file(file)
            || (self.allows_type_files && is_type_file(file))
            || (self.allows_types_directories && is_in_types_directory(file))
    }
}

pub fn check_file(
    file: &Path,
    source: &str,
    config: &RuleConfig,
    location_style: TypeLocationStyle,
) -> Vec<Violation> {
    if location_style.allows_file(file) {
        return Vec::new();
    }

    find_inline_type_declarations(file, source, config.severity)
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

fn find_inline_type_declarations(file: &Path, source: &str, severity: Severity) -> Vec<Violation> {
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
            _ if starts_type_alias_declaration(bytes, cursor.index)
                || starts_interface_declaration(bytes, cursor.index) =>
            {
                violations.push(inline_type_violation(file, &cursor, severity));
                cursor.advance(bytes);
            }
            _ => cursor.advance(bytes),
        }
    }

    violations
}

fn inline_type_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: cursor.line,
        column: cursor.column,
        rule: RULE_NAME,
        severity,
    }
}

fn starts_type_alias_declaration(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"type") {
        return false;
    }

    let Some(after_name) = index_after_declaration_name(bytes, index + b"type".len()) else {
        return false;
    };

    let next_index = skip_inline_whitespace(bytes, after_name);
    bytes.get(next_index) == Some(&b'=')
}

fn starts_interface_declaration(bytes: &[u8], index: usize) -> bool {
    if !starts_keyword(bytes, index, b"interface") {
        return false;
    }

    index_after_declaration_name(bytes, index + b"interface".len()).is_some()
}

fn index_after_declaration_name(bytes: &[u8], index: usize) -> Option<usize> {
    let name_start = skip_inline_whitespace(bytes, index);
    let first = bytes.get(name_start).copied()?;

    if !is_identifier_start(first) {
        return None;
    }

    let mut cursor = name_start + 1;
    while cursor < bytes.len() && is_identifier_part(bytes[cursor]) {
        cursor += 1;
    }

    Some(cursor)
}

fn starts_keyword(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes.get(index..index + keyword.len()) == Some(keyword)
        && !is_identifier_byte(bytes.get(index.wrapping_sub(1)).copied())
        && !is_identifier_byte(bytes.get(index + keyword.len()).copied())
}

fn is_identifier_byte(byte: Option<u8>) -> bool {
    byte.is_some_and(is_identifier_part)
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

fn is_identifier_part(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn skip_inline_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        index += 1;
    }

    index
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

fn is_type_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(TYPE_FILE_SUFFIX))
}

fn is_declaration_file(file: &Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(DECLARATION_FILE_SUFFIX))
}

fn is_in_types_directory(file: &Path) -> bool {
    file.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if name == TYPES_DIRECTORY_NAME
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{TypeLocationStyle, check_file};
    use crate::config::{RuleConfig, Severity};
    use std::path::{Path, PathBuf};

    #[test]
    fn reports_type_aliases_outside_type_files() {
        let violations = check_file(
            Path::new("Button.tsx"),
            "type ButtonProps = { label: string };\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].column, 1);
    }

    #[test]
    fn reports_interfaces_outside_type_files() {
        let source = r#"export interface ButtonProps {
  label: string;
}
"#;
        let violations = check_file(
            Path::new("Button.tsx"),
            source,
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].column, 8);
    }

    #[test]
    fn allows_type_declarations_in_type_files() {
        let violations = check_file(
            Path::new("Button.type.ts"),
            "export type ButtonProps = { label: string };\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_type_declarations_in_detected_types_directories() {
        let violations = check_file(
            Path::new("types/Button.ts"),
            "export interface ButtonProps {}\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")]),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_outside_detected_types_directories() {
        let violations = check_file(
            Path::new("Button.ts"),
            "interface ButtonProps {}\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("types/Button.ts")]),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn defaults_to_type_file_style_when_no_structure_exists() {
        let violations = check_file(
            Path::new("Button.ts"),
            "type ButtonProps = { label: string };\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.ts")]),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_imports_re_exports_comments_and_strings() {
        let source = r#"import type { ButtonProps } from "./Button.type";
export type { ButtonProps } from "./Button.type";
const text = "type ButtonProps = {}";
// interface ButtonProps {}
/* type ButtonProps = {} */
"#;
        let violations = check_file(
            Path::new("Button.ts"),
            source,
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn allows_declaration_files() {
        let violations = check_file(
            Path::new("global.d.ts"),
            "interface Window { appVersion: string }\n",
            &test_config(),
            TypeLocationStyle::detect(&[PathBuf::from("Button.type.ts")]),
        );

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
