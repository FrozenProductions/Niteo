use std::path::Path;

use oxc_ast::ast::CommentKind;

use crate::config::CommentsRuleConfig;
use crate::rules::{NO_COMMENTS_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Remove implementation comments or convert them to allowed documentation.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &CommentsRuleConfig,
) -> Vec<Violation> {
    let source = program.source_text;
    let mut violations = Vec::new();

    // Don't flag niteo's own suppression directives as code comments
    for comment in &program.comments {
        if is_niteo_directive(source, comment) {
            continue;
        }

        if config.allow_doc_comments && is_doc_comment(source, comment) {
            continue;
        }

        let pos = line_index.position_for(comment.span);
        violations.push(Violation {
            file: file.to_path_buf(),
            line: Some(pos.line),
            column: Some(pos.column),
            rule: NO_COMMENTS_RULE_ID,
            message: MESSAGE,
            severity: config.severity,
            detail: None,
            subject: None,
        });
    }

    violations
}

fn is_niteo_directive(source: &str, comment: &oxc_ast::ast::Comment) -> bool {
    let text = &source[comment.content_span().start as usize..comment.content_span().end as usize];
    let trimmed = text.trim_start();
    trimmed.starts_with("niteo-ignore-file")
        || trimmed.starts_with("niteo-ignore-next-line")
        || trimmed.starts_with("niteo-ignore-line")
}

fn is_doc_comment(source: &str, comment: &oxc_ast::ast::Comment) -> bool {
    match comment.kind {
        CommentKind::Line => {
            let text =
                &source[comment.content_span().start as usize..comment.content_span().end as usize];
            text.starts_with('/')
        }
        CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => {
            let text =
                &source[comment.content_span().start as usize..comment.content_span().end as usize];
            text.starts_with('*')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{CommentsRuleConfig, Severity};
    use crate::rules::Violation;
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, config: &CommentsRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let source_type = SourceType::tsx();
        let parser_return = Parser::new(&allocator, source, source_type).parse();
        let program = parser_return.program;
        check_file(Path::new("example.tsx"), &program, &line_index, config)
    }

    #[test]
    fn finds_line_comments() {
        let violations = run_check("const value = 1 // no\n", &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(17));
    }

    #[test]
    fn finds_block_comments() {
        let violations = run_check("const value = /* no */ 1\n", &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(1));
        assert_eq!(violations[0].column, Some(15));
    }

    #[test]
    fn finds_tsx_comments() {
        let source = "export function View() {\n  return <div>{/* no */}</div>\n}\n";
        let violations = run_check(source, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
        assert_eq!(violations[0].column, Some(16));
    }

    #[test]
    fn ignores_comment_markers_inside_strings() {
        let source = r#"const url = "https://example.com"
const block = "/* not a comment */"
"#;
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_doc_comments_when_allowed() {
        let source = "/// User model\n/** Component docs */\nconst value = 1\n";
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn reports_doc_comments_when_disallowed() {
        let config = CommentsRuleConfig {
            severity: Severity::Warn,
            allow_doc_comments: false,
        };
        let source = "/// User model\n/** Component docs */\nconst value = 1\n";
        let violations = run_check(source, &config);

        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn ignores_niteo_ignore_file_directive() {
        let source = "// niteo-ignore-file\nconst x = 1;\n";
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_niteo_ignore_next_line_directive() {
        let source = "// niteo-ignore-next-line: no-console\nconsole.log('test');\n";
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_niteo_ignore_line_directive() {
        let source = "console.log('test'); // niteo-ignore-line\n";
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_niteo_directive_with_extra_whitespace() {
        let source = "//   niteo-ignore-file\nconst x = 1;\n";
        let violations = run_check(source, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn still_reports_non_directive_niteo_comments() {
        let source = "// niteo is a linter\nconst x = 1;\n";
        let violations = run_check(source, &test_config());

        assert_eq!(violations.len(), 1);
    }

    fn test_config() -> CommentsRuleConfig {
        CommentsRuleConfig {
            severity: Severity::Warn,
            allow_doc_comments: true,
        }
    }
}
