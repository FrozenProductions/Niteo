use std::collections::HashMap;
use std::path::Path;

use crate::config::{NoInterfaceRuleConfig, Severity};
use crate::jsx::{Cursor, is_jsx_tag_start};
use crate::rules::{NO_INTERFACE_RULE_ID, Violation};
const MESSAGE: &str = "Use a type alias instead of an interface.";

pub fn check_file(file: &Path, source: &str, config: &NoInterfaceRuleConfig) -> Vec<Violation> {
    let scanner = Scanner::new(source);
    let interfaces = scanner.find_interfaces();

    if config.allow_declaration_merging {
        let name_counts = count_interface_names(&interfaces);
        interfaces
            .into_iter()
            .filter(|(name, _)| name_counts.get(name).copied().unwrap_or(0) <= 1)
            .map(|(_, cursor)| interface_violation(file, &cursor, config.severity))
            .collect()
    } else {
        interfaces
            .into_iter()
            .map(|(_, cursor)| interface_violation(file, &cursor, config.severity))
            .collect()
    }
}

#[derive(Debug)]
struct Scanner<'source> {
    bytes: &'source [u8],
}

impl<'source> Scanner<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            bytes: source.as_bytes(),
        }
    }

    fn find_interfaces(&self) -> Vec<(String, Cursor)> {
        let mut interfaces = Vec::new();
        let mut cursor = Cursor::default();
        self.scan_code(&mut cursor, StopAt::End, &mut interfaces);
        interfaces
    }

    fn scan_code(
        &self,
        cursor: &mut Cursor,
        stop_at: StopAt,
        interfaces: &mut Vec<(String, Cursor)>,
    ) {
        while cursor.index < self.bytes.len() {
            if stop_at.should_stop(self.bytes, cursor.index) {
                return;
            }

            let current = self.bytes[cursor.index];
            let next = self.bytes.get(cursor.index + 1).copied();

            match (current, next) {
                (b'\'', _) | (b'"', _) => self.skip_quoted_string(cursor, current),
                (b'`', _) => self.skip_template_literal(cursor),
                (b'/', Some(b'/')) => self.skip_line_comment(cursor),
                (b'/', Some(b'*')) => self.skip_block_comment(cursor),
                (b'<', _) if is_jsx_tag_start(self.bytes, cursor.index) => {
                    self.skip_jsx_element(cursor, interfaces);
                }
                _ if self.starts_interface(cursor.index) => {
                    self.collect_interface(cursor, interfaces);
                }
                _ => cursor.advance(self.bytes),
            }
        }
    }

    fn skip_jsx_element(&self, cursor: &mut Cursor, interfaces: &mut Vec<(String, Cursor)>) {
        let opening_tag = self.skip_jsx_tag(cursor, interfaces);
        if opening_tag.is_self_closing || opening_tag.is_closing {
            return;
        }

        self.skip_jsx_children(cursor, interfaces);
    }

    fn skip_jsx_children(&self, cursor: &mut Cursor, interfaces: &mut Vec<(String, Cursor)>) {
        while cursor.index < self.bytes.len() {
            let current = self.bytes[cursor.index];

            match current {
                b'{' => {
                    cursor.advance(self.bytes);
                    self.scan_code(cursor, StopAt::JsxExpressionEnd, interfaces);
                    if self.bytes.get(cursor.index) == Some(&b'}') {
                        cursor.advance(self.bytes);
                    }
                }
                b'<' if self.starts_jsx_closing_tag(cursor.index) => {
                    self.skip_jsx_tag(cursor, interfaces);
                    return;
                }
                b'<' if is_jsx_tag_start(self.bytes, cursor.index) => {
                    self.skip_jsx_element(cursor, interfaces);
                }
                _ => cursor.advance(self.bytes),
            }
        }
    }

    fn skip_jsx_tag(&self, cursor: &mut Cursor, interfaces: &mut Vec<(String, Cursor)>) -> JsxTag {
        let is_closing = self.starts_jsx_closing_tag(cursor.index);
        cursor.advance(self.bytes);

        while cursor.index < self.bytes.len() {
            let current = self.bytes[cursor.index];
            let next = self.bytes.get(cursor.index + 1).copied();

            match (current, next) {
                (b'\'', _) | (b'"', _) => self.skip_quoted_string(cursor, current),
                (b'`', _) => self.skip_template_literal(cursor),
                (b'/', Some(b'>')) => {
                    cursor.advance(self.bytes);
                    cursor.advance(self.bytes);
                    return JsxTag {
                        is_closing,
                        is_self_closing: true,
                    };
                }
                (b'{', _) => {
                    cursor.advance(self.bytes);
                    self.scan_code(cursor, StopAt::JsxExpressionEnd, interfaces);
                    if self.bytes.get(cursor.index) == Some(&b'}') {
                        cursor.advance(self.bytes);
                    }
                }
                (b'>', _) => {
                    cursor.advance(self.bytes);
                    return JsxTag {
                        is_closing,
                        is_self_closing: false,
                    };
                }
                _ => cursor.advance(self.bytes),
            }
        }

        JsxTag {
            is_closing,
            is_self_closing: false,
        }
    }

    fn collect_interface(&self, cursor: &mut Cursor, interfaces: &mut Vec<(String, Cursor)>) {
        let saved_cursor = *cursor;
        cursor.index += b"interface".len();
        cursor.column += b"interface".len();

        self.skip_whitespace_and_comments(cursor);

        let name_start = cursor.index;
        if !self
            .bytes
            .get(name_start)
            .copied()
            .is_some_and(is_identifier_start)
        {
            return;
        }

        cursor.advance(self.bytes);
        while cursor.index < self.bytes.len()
            && self
                .bytes
                .get(cursor.index)
                .copied()
                .is_some_and(is_identifier_part)
        {
            cursor.advance(self.bytes);
        }

        let name = String::from_utf8_lossy(&self.bytes[name_start..cursor.index]).to_string();
        interfaces.push((name, saved_cursor));
    }

    fn skip_template_literal(&self, cursor: &mut Cursor) {
        cursor.advance(self.bytes);

        while cursor.index < self.bytes.len() {
            let current = self.bytes[cursor.index];

            if current == b'\\' {
                cursor.advance(self.bytes);
                if cursor.index < self.bytes.len() {
                    cursor.advance(self.bytes);
                }
                continue;
            }

            if current == b'`' {
                cursor.advance(self.bytes);
                return;
            }

            cursor.advance(self.bytes);
        }
    }

    fn skip_quoted_string(&self, cursor: &mut Cursor, quote: u8) {
        cursor.advance(self.bytes);

        while cursor.index < self.bytes.len() {
            let current = self.bytes[cursor.index];

            if current == b'\\' {
                cursor.advance(self.bytes);
                if cursor.index < self.bytes.len() {
                    cursor.advance(self.bytes);
                }
                continue;
            }

            cursor.advance(self.bytes);

            if current == quote {
                return;
            }
        }
    }

    fn skip_line_comment(&self, cursor: &mut Cursor) {
        while cursor.index < self.bytes.len() && self.bytes[cursor.index] != b'\n' {
            cursor.advance(self.bytes);
        }
    }

    fn skip_block_comment(&self, cursor: &mut Cursor) {
        cursor.advance(self.bytes);
        cursor.advance(self.bytes);

        while cursor.index < self.bytes.len() {
            let current = self.bytes[cursor.index];
            let next = self.bytes.get(cursor.index + 1).copied();

            cursor.advance(self.bytes);

            if current == b'*' && next == Some(b'/') {
                cursor.advance(self.bytes);
                break;
            }
        }
    }

    fn skip_whitespace_and_comments(&self, cursor: &mut Cursor) {
        loop {
            while cursor.index < self.bytes.len() && self.bytes[cursor.index].is_ascii_whitespace()
            {
                cursor.advance(self.bytes);
            }

            if cursor.index >= self.bytes.len() {
                break;
            }

            match (
                self.bytes.get(cursor.index),
                self.bytes.get(cursor.index + 1),
            ) {
                (Some(b'/'), Some(b'/')) => self.skip_line_comment(cursor),
                (Some(b'/'), Some(b'*')) => self.skip_block_comment(cursor),
                _ => break,
            }
        }
    }

    fn starts_interface(&self, index: usize) -> bool {
        starts_keyword(self.bytes, index, b"interface")
    }

    fn starts_jsx_closing_tag(&self, index: usize) -> bool {
        self.bytes.get(index) == Some(&b'<') && self.bytes.get(index + 1) == Some(&b'/')
    }
}

#[derive(Debug, Clone, Copy)]
struct JsxTag {
    is_closing: bool,
    is_self_closing: bool,
}

#[derive(Debug, Clone, Copy)]
enum StopAt {
    End,
    JsxExpressionEnd,
}

impl StopAt {
    fn should_stop(self, bytes: &[u8], index: usize) -> bool {
        matches!(self, Self::JsxExpressionEnd) && bytes.get(index) == Some(&b'}')
    }
}

fn count_interface_names(interfaces: &[(String, Cursor)]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (name, _) in interfaces {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    counts
}

fn interface_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
    Violation {
        file: file.to_path_buf(),
        line: Some(cursor.line),
        column: Some(cursor.column),
        rule: NO_INTERFACE_RULE_ID,
        message: MESSAGE,
        severity,
        detail: None,
        subject: None,
    }
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

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{NoInterfaceRuleConfig, Severity};
    use std::path::Path;

    fn test_config() -> NoInterfaceRuleConfig {
        NoInterfaceRuleConfig {
            severity: Severity::Warn,
            allow_declaration_merging: true,
        }
    }

    fn strict_config() -> NoInterfaceRuleConfig {
        NoInterfaceRuleConfig {
            severity: Severity::Warn,
            allow_declaration_merging: false,
        }
    }

    #[test]
    fn reports_single_interface() {
        let violations = check_file(
            Path::new("types.ts"),
            "interface User { name: string }\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn allows_declaration_merging() {
        let source = r#"interface User { name: string }
interface User { age: number }
"#;
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_all_interfaces_when_merging_disabled() {
        let source = r#"interface User { name: string }
interface User { age: number }
"#;
        let violations = check_file(Path::new("types.ts"), source, &strict_config());

        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[1].line, Some(2));
    }

    #[test]
    fn reports_mixed_interfaces() {
        let source = r#"interface User { name: string }
interface User { age: number }
interface Post { title: string }
"#;
        let violations = check_file(Path::new("types.ts"), source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    }

    #[test]
    fn ignores_interface_in_comments_and_strings() {
        let source = r#"// interface User { name: string }
const text = "interface User";
/* interface Post { title: string } */
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

    #[test]
    fn ignores_interface_in_jsx_text() {
        let source = r#"<p className="mt-1">
    Scale the full app interface for the current window.
</p>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_interface_in_jsx_text_with_bracketed_tailwind_class() {
        let source = r#"<p className="mt-1 text-xs leading-[1.55] text-fumi-400">
    Scale the full app interface for the current window.
</p>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_interface_in_nested_jsx_text() {
        let source = r#"<div>
    <p>The user interface is ready.</p>
    <span>interface keyword in text</span>
</div>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_interface_in_jsx_expression() {
        let source = r#"<div>
    {interface User { name: string }}
</div>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &strict_config());

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_interface_in_jsx_attribute_values() {
        let source = r#"<Component tooltip="This interface is deprecated" />
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn handles_jsx_fragments() {
        let source = r#"<><p>interface text</p></>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn handles_jsx_with_expressions() {
        let source = r#"<div>
    {user.name}
    <p>interface description</p>
    {count}
</div>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn handles_jsx_attribute_expression_with_nested_object() {
        let source = r#"<Component
    options={{ label: "interface label", value: count }}
>
    interface text
</Component>
"#;
        let violations = check_file(Path::new("component.tsx"), source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_interface_after_jsx() {
        let source = r#"const element = <p>interface text</p>;

interface User { name: string }
"#;
        let violations = check_file(Path::new("component.tsx"), source, &strict_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert_eq!(violations[0].column, Some(1));
    }
}
