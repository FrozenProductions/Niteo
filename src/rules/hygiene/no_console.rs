use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use oxc_ast::ast::{BindingPattern, CallExpression, Expression, VariableDeclaration};
use oxc_ast_visit::Visit;

use crate::config::NoConsoleRuleConfig;
use crate::rules::{NO_CONSOLE_RULE_ID, Violation};
use crate::syntax::LineIndex;
const MESSAGE: &str = "Disallow console statements outside allowed file patterns.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoConsoleRuleConfig,
) -> Vec<Violation> {
    let allow_set = match build_allow_set(config) {
        Ok(allow_set) => allow_set,
        Err(error) => {
            eprintln!("warning: {error}");
            return Vec::new();
        }
    };
    let file_str = file.to_string_lossy();
    if allow_set
        .as_ref()
        .is_some_and(|set| set.is_match(file_str.as_ref()))
    {
        return Vec::new();
    }

    let mut visitor = ConsoleVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        console_aliases: HashSet::new(),
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct ConsoleVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    console_aliases: HashSet<String>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> Visit<'a> for ConsoleVisitor<'a, 'f> {
    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            let is_console_init = declarator.init.as_ref().is_some_and(|init| {
                matches!(init, Expression::Identifier(id) if id.name == "console")
            });
            if !is_console_init {
                continue;
            }

            if let BindingPattern::ObjectPattern(object_pattern) = &declarator.id {
                for property in object_pattern.properties.iter() {
                    if let BindingPattern::BindingIdentifier(binding_id) = &property.value {
                        self.console_aliases
                            .insert(binding_id.name.to_string());
                    }
                }
                if let Some(rest) = &object_pattern.rest
                    && let BindingPattern::BindingIdentifier(binding_id) = &rest.argument
                {
                    self.console_aliases
                        .insert(binding_id.name.to_string());
                }
            }
        }
        oxc_ast_visit::walk::walk_variable_declaration(self, decl);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(id) = &call.callee
            && self.console_aliases.contains(id.name.as_str())
        {
            self.violations.push(Violation::from_span(
                    self.file,
                    self.line_index,
                    id.span,
                    NO_CONSOLE_RULE_ID,
                    MESSAGE,
                    self.severity,
                ));
        }
        if let Expression::StaticMemberExpression(static_member) = &call.callee
            && matches!(&static_member.object, Expression::Identifier(id) if id.name == "console")
        {
            self.violations.push(Violation::from_span(
                self.file,
                self.line_index,
                static_member.span,
                NO_CONSOLE_RULE_ID,
                MESSAGE,
                self.severity,
            ));
        }
        if let Expression::ComputedMemberExpression(computed) = &call.callee
            && matches!(&computed.object, Expression::Identifier(id) if id.name == "console")
        {
            self.violations.push(Violation::from_span(
                self.file,
                self.line_index,
                computed.span,
                NO_CONSOLE_RULE_ID,
                MESSAGE,
                self.severity,
            ));
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }
}

fn build_allow_set(config: &NoConsoleRuleConfig) -> anyhow::Result<Option<GlobSet>> {
    if config.allow_patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in &config.allow_patterns {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .with_context(|| format!("invalid allow-patterns glob: {pattern}"))?;
        builder.add(glob);
    }

    Ok(Some(
        builder
            .build()
            .with_context(|| "failed to build glob set")?,
    ))
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NoConsoleRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, config: &NoConsoleRuleConfig) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::tsx()).parse();
        let program = parser_return.program;
        check_file(Path::new("Component.tsx"), &program, &line_index, config)
    }

    #[test]
    fn reports_console_methods() -> Result<()> {
        for source in [
            "console.log('hello');\n",
            "console.warn('warning');\n",
            "console.error('error');\n",
        ] {
            let violations = run_check(source, &test_config());
            assert_eq!(violations.len(), 1, "expected 1 violation for: {source:?}");
        }
    
        Ok(())}

    #[test]
    fn reports_console_bracket_access() -> Result<()> {
        let violations = run_check("console['log']('hello');\n", &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_console_in_service_files() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec!["**/*.service.ts".to_string()],
        };
        let allocator = Allocator::default();
        let source = "console.log('hello');\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("api.service.ts"),
            &program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_console_in_comments() -> Result<()> {
        let source = "// console.log('hello');\n/* console.warn('test'); */\n";
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_console_in_strings() -> Result<()> {
        let source = r#"const text = "console.log('hello')";"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_log_property_on_non_console_object() -> Result<()> {
        let source = r#"const logger = { log: console.log }; logger.log('hello');"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_console_with_glob_non_match() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec!["**/*.service.ts".to_string()],
        };
        let violations = run_check("console.log('hello');\n", &config);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_console_with_directory_glob() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec!["scripts/**".to_string()],
        };
        let allocator = Allocator::default();
        let source = "console.log('hello');\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("scripts/deploy.ts"),
            &program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_console_outside_glob_directory() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec!["scripts/**".to_string()],
        };
        let violations = run_check("console.log('hello');\n", &config);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_destructured_console_log() -> Result<()> {
        let source = "const { log } = console;\nlog('hello');\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_destructured_console_warn() -> Result<()> {
        let source = "const { warn } = console;\nwarn('warning');\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_renamed_destructured_console() -> Result<()> {
        let source = "const { log: renamedLog } = console;\nrenamedLog('hello');\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_multiple_destructured_console_methods() -> Result<()> {
        let source =
            "const { log, warn, error } = console;\nlog('a');\nwarn('b');\nerror('c');\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 3);
    
        Ok(())}

    #[test]
    fn ignores_non_console_destructuring() -> Result<()> {
        let source = r#"const { log } = someLogger;\nlog('hello');\n"#;
        let violations = run_check(source, &test_config());
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn ignores_destructured_console_when_file_allowed() -> Result<()> {
        let config = NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec!["**/*.service.ts".to_string()],
        };
        let allocator = Allocator::default();
        let source = "const { log } = console;\nlog('hello');\n";
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        let violations = check_file(
            Path::new("api.service.ts"),
            &program,
            &line_index,
            &config,
        );
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_destructured_and_direct_console() -> Result<()> {
        let source = "const { log } = console;\nlog('hello');\nconsole.warn('warn');\n";
        let violations = run_check(source, &test_config());
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    fn test_config() -> NoConsoleRuleConfig {
        NoConsoleRuleConfig {
            severity: Severity::Warn,
            allow_patterns: vec![],
        }
    }
}
