#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub index: usize,
    pub line: usize,
    pub column: usize,
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
    pub fn advance(&mut self, bytes: &[u8]) {
        if bytes[self.index] == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.index += 1;
    }
}

#[allow(dead_code)]
pub fn contains_jsx(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut cursor = Cursor::default();
    scan_for_jsx(bytes, &mut cursor)
}

pub fn first_jsx_location(source: &str) -> Option<Cursor> {
    let bytes = source.as_bytes();
    let mut cursor = Cursor::default();
    if scan_for_jsx(bytes, &mut cursor) {
        Some(cursor)
    } else {
        None
    }
}

fn scan_for_jsx(bytes: &[u8], cursor: &mut Cursor) -> bool {
    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];
        let next = bytes.get(cursor.index + 1).copied();

        match (current, next) {
            (b'\'', _) | (b'"', _) => skip_quoted_string(bytes, cursor, current),
            (b'`', _) => skip_template_literal(bytes, cursor),
            (b'/', Some(b'/')) => skip_line_comment(bytes, cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, cursor),
            (b'<', _) if is_jsx_tag_start(bytes, cursor.index) => {
                return true;
            }
            _ => cursor.advance(bytes),
        }
    }

    false
}

fn skip_quoted_string(bytes: &[u8], cursor: &mut Cursor, quote: u8) {
    cursor.advance(bytes);

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];

        if current == b'\\' {
            cursor.advance(bytes);
            if cursor.index < bytes.len() {
                cursor.advance(bytes);
            }
            continue;
        }

        cursor.advance(bytes);

        if current == quote {
            return;
        }
    }
}

fn skip_template_literal(bytes: &[u8], cursor: &mut Cursor) {
    cursor.advance(bytes);

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];

        if current == b'\\' {
            cursor.advance(bytes);
            if cursor.index < bytes.len() {
                cursor.advance(bytes);
            }
            continue;
        }

        if current == b'`' {
            cursor.advance(bytes);
            return;
        }

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

pub fn is_jsx_tag_start(bytes: &[u8], index: usize) -> bool {
    let after = index + 1;
    match bytes.get(after) {
        Some(b'/') | Some(b'>') => true,
        Some(b'a'..=b'z') | Some(b'A'..=b'Z') => true,
        _ => false,
    }
}

pub fn is_hook_file(path: &std::path::Path) -> bool {
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    if file_stem.ends_with(".hook") || file_stem.ends_with(".hooks") {
        return true;
    }

    if let Some(parent) = path.parent() {
        if parent.file_name().map(|n| n.to_string_lossy()) == Some("hooks".into()) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_simple_jsx() {
        assert!(contains_jsx("<div>Hello</div>"));
    }

    #[test]
    fn detects_self_closing_jsx() {
        assert!(contains_jsx("<Component />"));
    }

    #[test]
    fn detects_jsx_fragment() {
        assert!(contains_jsx("<><p>text</p></>"));
    }

    #[test]
    fn ignores_jsx_in_string() {
        assert!(!contains_jsx("const text = \"<div>Hello</div>\";"));
    }

    #[test]
    fn ignores_jsx_in_comment() {
        assert!(!contains_jsx("// <div>Hello</div>"));
        assert!(!contains_jsx("/* <div>Hello</div> */"));
    }

    #[test]
    fn ignores_jsx_in_template_literal() {
        assert!(!contains_jsx("const text = `<div>Hello</div>`;"));
    }

    #[test]
    fn returns_false_for_plain_typescript() {
        assert!(!contains_jsx("const x: number = 1;"));
    }

    #[test]
    fn hook_file_by_suffix() {
        assert!(is_hook_file(Path::new("useAuth.hook.ts")));
        assert!(is_hook_file(Path::new("useAuth.hooks.ts")));
    }

    #[test]
    fn hook_file_in_hooks_folder() {
        assert!(is_hook_file(Path::new("src/hooks/useAuth.ts")));
        assert!(is_hook_file(Path::new("hooks/useAuth.tsx")));
    }

    #[test]
    fn non_hook_file() {
        assert!(!is_hook_file(Path::new("src/components/Button.tsx")));
        assert!(!is_hook_file(Path::new("src/utils/format.ts")));
    }
}
