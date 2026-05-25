use std::path::{Component, Path};

use crate::config::{NoLogicInDomainRuleConfig, Severity};
use crate::rules::Violation;

const RULE_NAME: &str = "no-logic-in-domain";
const MESSAGE: &str =
    "Keep domain files free of logic. Move implementation to feature or service files.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DomainKind {
    Types,
    Constants,
}

pub fn check_file(file: &Path, source: &str, config: &NoLogicInDomainRuleConfig) -> Vec<Violation> {
    let Some(kind) = classify_domain_file(file, config) else {
        return Vec::new();
    };

    find_logic_in_file(file, source, config.severity, kind)
}

fn classify_domain_file(file: &Path, config: &NoLogicInDomainRuleConfig) -> Option<DomainKind> {
    if is_types_file(file, config) {
        return Some(DomainKind::Types);
    }

    if is_constants_file(file, config) {
        return Some(DomainKind::Constants);
    }

    None
}

fn is_types_file(file: &Path, config: &NoLogicInDomainRuleConfig) -> bool {
    let mut folders = vec!["types".to_string()];
    folders.extend(config.extra_folders.clone());

    let in_types_folder = file.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if folders.iter().any(|f| name.to_str() == Some(f))
        )
    });

    let has_types_suffix = {
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let mut suffixes = vec![".type.ts".to_string(), ".types.ts".to_string()];
        suffixes.extend(config.extra_file_suffixes.clone());
        suffixes.iter().any(|suffix| file_name.ends_with(suffix))
    };

    in_types_folder || has_types_suffix
}

fn is_constants_file(file: &Path, config: &NoLogicInDomainRuleConfig) -> bool {
    let mut folders = vec!["constants".to_string()];
    folders.extend(config.extra_folders.clone());

    let in_constants_folder = file.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if folders.iter().any(|f| name.to_str() == Some(f))
        )
    });

    let has_constants_suffix = {
        let file_name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let mut suffixes = vec![".constant.ts".to_string(), ".constants.ts".to_string()];
        suffixes.extend(config.extra_file_suffixes.clone());
        suffixes.iter().any(|suffix| file_name.ends_with(suffix))
    };

    in_constants_folder || has_constants_suffix
}

fn find_logic_in_file(
    file: &Path,
    source: &str,
    severity: Severity,
    kind: DomainKind,
) -> Vec<Violation> {
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
            (b'/', _) if is_regex_start(bytes, cursor.index) => {
                skip_regex_literal(bytes, &mut cursor);
            }
            _ if starts_logic_pattern(bytes, cursor.index, kind) => {
                violations.push(logic_violation(file, &cursor, severity));
                skip_statement(bytes, &mut cursor);
            }
            _ => cursor.advance(bytes),
        }
    }

    violations
}

fn skip_statement(bytes: &[u8], cursor: &mut Cursor) {
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
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
            (b'/', Some(b'/')) => skip_line_comment(bytes, cursor),
            (b'/', Some(b'*')) => skip_block_comment(bytes, cursor),
            (b'/', _) if is_regex_start(bytes, cursor.index) => {
                skip_regex_literal(bytes, cursor);
            }
            (b'{', _) => {
                brace_depth += 1;
                cursor.advance(bytes);
            }
            (b'}', _) => {
                brace_depth = brace_depth.saturating_sub(1);
                cursor.advance(bytes);
            }
            (b'(', _) => {
                paren_depth += 1;
                cursor.advance(bytes);
            }
            (b')', _) => {
                paren_depth = paren_depth.saturating_sub(1);
                cursor.advance(bytes);
            }
            (b';', _) if brace_depth == 0 && paren_depth == 0 => {
                cursor.advance(bytes);
                break;
            }
            (b'\n', _) if brace_depth == 0 && paren_depth == 0 => {
                cursor.advance(bytes);
                break;
            }
            _ => cursor.advance(bytes),
        }
    }
}

fn starts_logic_pattern(bytes: &[u8], index: usize, kind: DomainKind) -> bool {
    match kind {
        DomainKind::Types => {
            starts_function_declaration(bytes, index)
                || starts_class_declaration(bytes, index)
                || starts_const_declaration(bytes, index)
                || starts_let_declaration(bytes, index)
                || starts_var_declaration(bytes, index)
        }
        DomainKind::Constants => {
            starts_function_declaration(bytes, index)
                || starts_class_declaration(bytes, index)
                || starts_let_declaration(bytes, index)
                || starts_var_declaration(bytes, index)
                || starts_const_with_logic(bytes, index)
        }
    }
}

fn starts_const_with_logic(bytes: &[u8], index: usize) -> bool {
    let Some(const_pos) = find_const_position(bytes, index) else {
        return false;
    };

    let after_const = const_pos + b"const".len();
    let after_const_bytes = &bytes[after_const..];
    if !after_const_bytes
        .first()
        .is_some_and(|&b| b.is_ascii_whitespace())
    {
        return false;
    }

    let after_name = skip_past_identifier(bytes, after_const);
    let after_eq = skip_whitespace(&bytes[after_name..]);
    if !after_eq.starts_with(b"=") {
        return false;
    }

    let after_eq_pos = after_eq.as_ptr() as usize - bytes.as_ptr() as usize + b"=".len();
    let after_eq_whitespace = skip_whitespace(&bytes[after_eq_pos..]);

    starts_arrow_function(after_eq_whitespace) || starts_function_expression(after_eq_whitespace)
}

fn find_const_position(bytes: &[u8], index: usize) -> Option<usize> {
    if starts_keyword(bytes, index, b"const") {
        return Some(index);
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        if rest.starts_with(b"const") {
            let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;
            return Some(rest_offset);
        }
    }

    None
}

fn starts_arrow_function(slice: &[u8]) -> bool {
    if slice.starts_with(b"(") {
        let after_paren = skip_past_paren_group(slice);
        let after_paren_whitespace = skip_whitespace(after_paren);
        return after_paren_whitespace.starts_with(b"=>");
    }

    if slice
        .first()
        .is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_')
    {
        let after_param = skip_past_identifier(slice, 0);
        let after_param_whitespace = skip_whitespace(&slice[after_param..]);
        return after_param_whitespace.starts_with(b"=>");
    }

    false
}

fn starts_function_expression(slice: &[u8]) -> bool {
    if slice.starts_with(b"function") {
        let after_function = slice.as_ptr() as usize - slice.as_ptr() as usize + b"function".len();
        return slice
            .get(after_function)
            .is_some_and(|&b| b.is_ascii_whitespace() || b == b'(');
    }

    if slice.starts_with(b"async") {
        let after_async = skip_whitespace(&slice[b"async".len()..]);
        return after_async.starts_with(b"function");
    }

    false
}

fn skip_past_paren_group(bytes: &[u8]) -> &[u8] {
    if !bytes.starts_with(b"(") {
        return bytes;
    }

    let mut depth = 0;
    let mut i = 0;
    let mut string_quote: Option<u8> = None;

    while i < bytes.len() {
        let current = bytes[i];

        if let Some(quote) = string_quote {
            if current == b'\\' {
                i += 2;
                continue;
            }
            if current == quote {
                string_quote = None;
            }
            i += 1;
            continue;
        }

        match current {
            b'\'' | b'"' | b'`' => string_quote = Some(current),
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &bytes[i + 1..];
                }
            }
            _ => {}
        }

        i += 1;
    }

    &bytes[bytes.len()..]
}

fn skip_past_identifier(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }

    while i < bytes.len() && is_identifier_byte(Some(bytes[i])) {
        i += 1;
    }

    i
}

fn starts_function_declaration(bytes: &[u8], index: usize) -> bool {
    if starts_keyword(bytes, index, b"function") {
        let after_function = index + b"function".len();
        let next = bytes.get(after_function).copied();
        return matches!(next, Some(b' ' | b'\t' | b'\n' | b'\r' | b'(' | b'<'));
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;

        if rest.starts_with(b"function") {
            let after_function = rest_offset + b"function".len();
            if bytes
                .get(after_function)
                .is_some_and(|&b| b.is_ascii_whitespace() || b == b'(' || b == b'<')
            {
                return true;
            }
        }

        if rest.starts_with(b"async") {
            let after_async = skip_whitespace(&bytes[rest_offset + b"async".len()..]);
            if after_async.starts_with(b"function") {
                return true;
            }
        }
    }

    if starts_keyword(bytes, index, b"async") {
        let after_async = index + b"async".len();
        let rest = skip_whitespace(&bytes[after_async..]);
        let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;

        if rest.starts_with(b"function") {
            return true;
        }

        if rest.starts_with(b"export") {
            let after_export = skip_whitespace(&bytes[rest_offset + b"export".len()..]);
            if after_export.starts_with(b"async function") || after_export.starts_with(b"function")
            {
                return true;
            }
        }
    }

    false
}

fn starts_const_declaration(bytes: &[u8], index: usize) -> bool {
    if starts_keyword(bytes, index, b"const") {
        let after_const = index + b"const".len();
        if bytes
            .get(after_const)
            .is_some_and(|&b| b.is_ascii_whitespace())
        {
            return true;
        }
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        if rest.starts_with(b"const") {
            let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;
            let after_const = rest_offset + b"const".len();
            if bytes
                .get(after_const)
                .is_some_and(|&b| b.is_ascii_whitespace())
            {
                return true;
            }
        }
    }

    false
}

fn starts_let_declaration(bytes: &[u8], index: usize) -> bool {
    if starts_keyword(bytes, index, b"let") {
        let after_let = index + b"let".len();
        if bytes
            .get(after_let)
            .is_some_and(|&b| b.is_ascii_whitespace())
        {
            return true;
        }
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        if rest.starts_with(b"let") {
            let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;
            let after_let = rest_offset + b"let".len();
            if bytes
                .get(after_let)
                .is_some_and(|&b| b.is_ascii_whitespace())
            {
                return true;
            }
        }
    }

    false
}

fn starts_var_declaration(bytes: &[u8], index: usize) -> bool {
    if starts_keyword(bytes, index, b"var") {
        let after_var = index + b"var".len();
        if bytes
            .get(after_var)
            .is_some_and(|&b| b.is_ascii_whitespace())
        {
            return true;
        }
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        if rest.starts_with(b"var") {
            let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;
            let after_var = rest_offset + b"var".len();
            if bytes
                .get(after_var)
                .is_some_and(|&b| b.is_ascii_whitespace())
            {
                return true;
            }
        }
    }

    false
}

fn starts_class_declaration(bytes: &[u8], index: usize) -> bool {
    if starts_keyword(bytes, index, b"class") {
        let after_class = index + b"class".len();
        if bytes
            .get(after_class)
            .is_some_and(|&b| b.is_ascii_whitespace())
        {
            return true;
        }
    }

    if starts_keyword(bytes, index, b"export") {
        let after_export = index + b"export".len();
        let rest = skip_whitespace(&bytes[after_export..]);
        if rest.starts_with(b"class") {
            let rest_offset = rest.as_ptr() as usize - bytes.as_ptr() as usize;
            let after_class = rest_offset + b"class".len();
            if bytes
                .get(after_class)
                .is_some_and(|&b| b.is_ascii_whitespace())
            {
                return true;
            }
        }
    }

    false
}

fn skip_whitespace(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }
    &bytes[i..]
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

fn logic_violation(file: &Path, cursor: &Cursor, severity: Severity) -> Violation {
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

fn is_regex_start(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    let prev = bytes[index - 1];
    matches!(
        prev,
        b'(' | b'['
            | b'{'
            | b','
            | b';'
            | b':'
            | b'='
            | b'+'
            | b'-'
            | b'!'
            | b'~'
            | b'&'
            | b'|'
            | b'^'
            | b'?'
            | b'>'
            | b'<'
            | b'%'
            | b'*'
            | b'/'
            | b'\n'
            | b'\r'
            | b'\t'
            | b' '
    )
}

fn skip_regex_literal(bytes: &[u8], cursor: &mut Cursor) {
    cursor.advance(bytes);

    let mut in_char_class = false;

    while cursor.index < bytes.len() {
        let current = bytes[cursor.index];

        if current == b'\\' {
            cursor.advance(bytes);
            if cursor.index < bytes.len() {
                cursor.advance(bytes);
            }
            continue;
        }

        if current == b'[' {
            in_char_class = true;
            cursor.advance(bytes);
            continue;
        }

        if current == b']' && in_char_class {
            in_char_class = false;
            cursor.advance(bytes);
            continue;
        }

        if current == b'/' && !in_char_class {
            cursor.advance(bytes);
            while cursor.index < bytes.len() && bytes[cursor.index].is_ascii_alphabetic() {
                cursor.advance(bytes);
            }
            return;
        }

        cursor.advance(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{NoLogicInDomainRuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_function_in_types_folder() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("types/Button.ts"),
            "function handleClick() {}\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
    }

    #[test]
    fn allows_const_in_constants_folder() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("constants/routes.ts"),
            "export const ROUTES = { HOME: '/' };\n",
            &config,
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_function_in_constants_folder() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("constants/routes.ts"),
            "function getRoute() { return '/'; }\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_const_arrow_function_in_constants_folder() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("constants/routes.ts"),
            "const getRoute = () => '/';\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_function_in_type_file() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("Button.type.ts"),
            "export function handleClick() {}\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_class_in_constant_file() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("api.constants.ts"),
            "export class ApiClient {}\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_type_declaration_in_type_file() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("Button.type.ts"),
            "export type ButtonProps = { label: string };\n",
            &config,
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_const_in_type_file() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(Path::new("Button.type.ts"), "const VALUE = 1;\n", &config);

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_let_declaration() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(Path::new("types/state.ts"), "let count = 0;\n", &config);

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_async_function() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("types/api.ts"),
            "async function fetchData() {}\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_logic_in_comments() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let source = "// function handleClick() {}\n/* const value = 1; */\n";
        let violations = check_file(Path::new("types/test.ts"), source, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_logic_in_strings() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let source = r#"const text = "function handleClick() {}";"#;
        let violations = check_file(Path::new("constants/test.ts"), source, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_keywords_in_regex() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let source = r#"export const PATTERN = /\b(function|const|local|export)\b/g;"#;
        let violations = check_file(Path::new("constants/test.ts"), source, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_keywords_in_object_literals() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let source = r#"export const DOCS = {
    function: "Define a function body.",
    local: "Declare a local variable.",
    typeof: "Capture the inferred type.",
};"#;
        let violations = check_file(Path::new("constants/test.ts"), source, &config);

        assert!(violations.is_empty());
    }

    #[test]
    fn respects_extra_folders_config() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec!["enums".to_string()],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("enums/status.ts"),
            "function getStatus() {}\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn respects_extra_file_suffixes_config() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![".model.ts".to_string()],
        };

        let violations = check_file(Path::new("User.model.ts"), "class User {}\n", &config);

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn ignores_non_domain_files() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("Button.tsx"),
            "function handleClick() {}\n",
            &config,
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_exported_arrow_function_in_constants() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("constants/helpers.ts"),
            "export const formatName = (name: string) => name.trim();\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_exported_function_expression_in_constants() {
        let config = NoLogicInDomainRuleConfig {
            severity: Severity::Warn,
            extra_folders: vec![],
            extra_file_suffixes: vec![],
        };

        let violations = check_file(
            Path::new("constants/helpers.ts"),
            "export const formatName = function(name: string) { return name.trim(); };\n",
            &config,
        );

        assert_eq!(violations.len(), 1);
    }
}
