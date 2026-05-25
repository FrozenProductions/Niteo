use std::path::Path;

use crate::config::RuleConfig;
use crate::jsx::{first_jsx_location, is_hook_file};
use crate::rules::{HOOK_NO_JSX_RULE_ID, Violation};
const MESSAGE: &str = "Hook files should not contain JSX. Extract UI into a separate component.";

pub fn check_file(file: &Path, source: &str, config: &RuleConfig) -> Vec<Violation> {
    if !is_hook_file(file) {
        return Vec::new();
    }

    if let Some(cursor) = first_jsx_location(source) {
        return vec![Violation {
            file: file.to_path_buf(),
            line: Some(cursor.line),
            column: Some(cursor.column),
            rule: HOOK_NO_JSX_RULE_ID,
            message: MESSAGE,
            severity: config.severity,
            detail: None,
            subject: None,
        }];
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::check_file;
    use crate::config::{RuleConfig, Severity};
    use std::path::Path;

    #[test]
    fn reports_jsx_in_hook_file() {
        let violations = check_file(
            Path::new("src/hooks/useAuth.ts"),
            "export function useAuth() {\n  return <div>Loading</div>;\n}\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(2));
    }

    #[test]
    fn reports_jsx_in_dot_hook_file() {
        let violations = check_file(
            Path::new("useAuth.hook.ts"),
            "export function useAuth() {\n  return <p>Hello</p>;\n}\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn reports_jsx_in_dot_hooks_file() {
        let violations = check_file(
            Path::new("useAuth.hooks.ts"),
            "export function useAuth() {\n  return <span>Hi</span>;\n}\n",
            &test_config(),
        );

        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn allows_hook_without_jsx() {
        let violations = check_file(
            Path::new("src/hooks/useAuth.ts"),
            "export function useAuth() {\n  return { user: null };\n}\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_jsx_in_non_hook_file() {
        let violations = check_file(
            Path::new("src/components/Auth.tsx"),
            "export function Auth() {\n  return <div>Login</div>;\n}\n",
            &test_config(),
        );

        assert!(violations.is_empty());
    }

    #[test]
    fn ignores_jsx_in_comments_and_strings() {
        let source = r#"// return <div>Loading</div>;
const text = "<p>Hello</p>";
"#;
        let violations = check_file(Path::new("src/hooks/useAuth.ts"), source, &test_config());

        assert!(violations.is_empty());
    }

    fn test_config() -> RuleConfig {
        RuleConfig {
            severity: Severity::Warn,
        }
    }
}
