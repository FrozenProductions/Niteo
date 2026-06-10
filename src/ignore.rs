use std::path::PathBuf;

const FILE_DIRECTIVE: &str = "niteo-ignore-file";
const NEXT_LINE_DIRECTIVE: &str = "niteo-ignore-next-line";
const LINE_DIRECTIVE: &str = "niteo-ignore-line";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoreKind {
    File,
    Line,
    NextLine,
}

impl std::fmt::Display for IgnoreKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IgnoreKind::File => write!(f, "niteo-ignore-file"),
            IgnoreKind::Line => write!(f, "niteo-ignore-line"),
            IgnoreKind::NextLine => write!(f, "niteo-ignore-next-line"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IgnoreDirective {
    pub kind: IgnoreKind,
    pub line: usize,
    pub rules: Vec<String>,
}

impl IgnoreDirective {
    pub fn should_suppress(&self, violation_line: Option<usize>, rule_name: &str) -> bool {
        if !self.rules.is_empty() && !self.rules.iter().any(|rule| rule == rule_name) {
            return false;
        }

        match self.kind {
            IgnoreKind::File => true,
            IgnoreKind::Line => violation_line == Some(self.line),
            IgnoreKind::NextLine => violation_line == Some(self.line + 1),
        }
    }
}

impl std::fmt::Display for IgnoreDirective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        if !self.rules.is_empty() {
            write!(f, ": {}", self.rules.join(", "))?;
        }
        Ok(())
    }
}

pub fn parse_ignore_directives(source: &str) -> Vec<IgnoreDirective> {
    let mut directives = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;

        if let Some(directive) = find_directive(line, line_number) {
            directives.push(directive);
        }
    }

    directives
}

// Byte-level scanning detects directives only in actual comments, not inside string literals
fn find_directive(line: &str, line_number: usize) -> Option<IgnoreDirective> {
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let current = bytes[index];
        let next = bytes.get(index + 1).copied();

        match (current, next) {
            (b'\'', _) | (b'"', _) | (b'`', _) => {
                index = skip_string(bytes, index, current);
            }
            (b'/', Some(b'/')) => {
                let comment_text = &line[index + 2..];
                return parse_comment_directive(comment_text, line_number);
            }
            (b'/', Some(b'*')) => {
                index = skip_block_comment_inline(bytes, index);
            }
            _ => index += 1,
        }
    }

    None
}

fn parse_comment_directive(comment: &str, line_number: usize) -> Option<IgnoreDirective> {
    let trimmed = comment.trim();

    let (prefix, kind) = if trimmed.starts_with(FILE_DIRECTIVE) {
        (FILE_DIRECTIVE, IgnoreKind::File)
    } else if trimmed.starts_with(NEXT_LINE_DIRECTIVE) {
        (NEXT_LINE_DIRECTIVE, IgnoreKind::NextLine)
    } else if trimmed.starts_with(LINE_DIRECTIVE) {
        (LINE_DIRECTIVE, IgnoreKind::Line)
    } else {
        return None;
    };

    let after_prefix = &trimmed[prefix.len()..];
    let rules = parse_rules(after_prefix);

    Some(IgnoreDirective {
        kind,
        line: line_number,
        rules,
    })
}

fn parse_rules(after_prefix: &str) -> Vec<String> {
    let trimmed = after_prefix.trim();
    if let Some(rules_part) = trimmed.strip_prefix(':') {
        rules_part
            .split(',')
            .map(|segment| segment.trim().to_string())
            .filter(|segment| !segment.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

fn skip_string(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes.get(index).copied() {
            Some(b'\\') => {
                index += 2;
                continue;
            }
            Some(q) if q == quote => return index + 1,
            _ => index += 1,
        }
    }
    index
}

fn skip_block_comment_inline(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

pub fn should_suppress_violation(
    directives: &[IgnoreDirective],
    violation_line: Option<usize>,
    rule_name: &str,
) -> bool {
    directives
        .iter()
        .any(|directive| directive.should_suppress(violation_line, rule_name))
}

#[derive(Debug, Clone)]
pub struct SuppressionReport {
    pub files: Vec<FileSuppressionInfo>,
}

#[derive(Debug, Clone)]
pub struct FileSuppressionInfo {
    pub file: PathBuf,
    pub suppressed_count: usize,
    pub stale_directives: Vec<IgnoreDirective>,
}

impl SuppressionReport {
    pub fn total_suppressed(&self) -> usize {
        self.files
            .iter()
            .map(|file_info| file_info.suppressed_count)
            .sum()
    }

    pub fn total_stale(&self) -> usize {
        self.files
            .iter()
            .map(|file_info| file_info.stale_directives.len())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_ignore_all_rules() {
        let source = "// niteo-ignore-file\nconst x = 1;\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::File);
        assert!(directives[0].rules.is_empty());
    }

    #[test]
    fn parse_file_ignore_specific_rule() {
        let source = "// niteo-ignore-file: no-console\nconst x = 1;\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::File);
        assert_eq!(directives[0].rules, vec!["no-console"]);
    }

    #[test]
    fn parse_file_ignore_multiple_rules() {
        let source = "// niteo-ignore-file: no-console, no-debugger\nconst x = 1;\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].rules, vec!["no-console", "no-debugger"]);
    }

    #[test]
    fn parse_next_line_ignore_all_rules() {
        let source = "// niteo-ignore-next-line\nconsole.log('test');\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::NextLine);
        assert_eq!(directives[0].line, 1);
    }

    #[test]
    fn parse_next_line_ignore_specific_rule() {
        let source = "// niteo-ignore-next-line: no-console\nconsole.log('test');\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::NextLine);
        assert_eq!(directives[0].rules, vec!["no-console"]);
    }

    #[test]
    fn parse_inline_line_ignore() {
        let source = "console.log('test'); // niteo-ignore-line\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::Line);
        assert_eq!(directives[0].line, 1);
        assert!(directives[0].rules.is_empty());
    }

    #[test]
    fn parse_inline_line_ignore_specific_rule() {
        let source = "console.log('test'); // niteo-ignore-line: no-console\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::Line);
        assert_eq!(directives[0].rules, vec!["no-console"]);
    }

    #[test]
    fn parse_standalone_line_ignore() {
        let source = "// niteo-ignore-line\nconsole.log('test');\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, IgnoreKind::Line);
        assert_eq!(directives[0].line, 1);
    }

    #[test]
    fn ignores_directives_inside_strings() {
        let source = "const x = \"// niteo-ignore-file\";\n";
        let directives = parse_ignore_directives(source);

        assert!(directives.is_empty());
    }

    #[test]
    fn suppress_file_violation() {
        let directives = parse_ignore_directives("// niteo-ignore-file\nconst x = 1;");

        assert!(should_suppress_violation(
            &directives,
            Some(2),
            "no-console"
        ));
        assert!(should_suppress_violation(
            &directives,
            Some(2),
            "no-debugger"
        ));
    }

    #[test]
    fn suppress_file_violation_specific_rule() {
        let directives =
            parse_ignore_directives("// niteo-ignore-file: no-console\nconsole.log('x');");

        assert!(should_suppress_violation(
            &directives,
            Some(2),
            "no-console"
        ));
        assert!(!should_suppress_violation(
            &directives,
            Some(2),
            "no-debugger"
        ));
    }

    #[test]
    fn suppress_next_line_violation() {
        let directives = parse_ignore_directives("// niteo-ignore-next-line\nconsole.log('test');");

        assert!(should_suppress_violation(
            &directives,
            Some(2),
            "no-console"
        ));
        assert!(!should_suppress_violation(
            &directives,
            Some(3),
            "no-console"
        ));
    }

    #[test]
    fn suppress_inline_line_violation() {
        let directives = parse_ignore_directives("console.log('test'); // niteo-ignore-line");

        assert!(should_suppress_violation(
            &directives,
            Some(1),
            "no-console"
        ));
        assert!(!should_suppress_violation(
            &directives,
            Some(2),
            "no-console"
        ));
    }

    #[test]
    fn multiple_directives_in_file() {
        let source =
            "// niteo-ignore-file: no-enums\n\n// niteo-ignore-next-line\nconsole.log('x');\n";
        let directives = parse_ignore_directives(source);

        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].kind, IgnoreKind::File);
        assert_eq!(directives[1].kind, IgnoreKind::NextLine);
        assert_eq!(directives[1].line, 3);
    }
}
