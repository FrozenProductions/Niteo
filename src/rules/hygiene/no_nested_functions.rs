use std::path::Path;

use oxc_ast::ast::{Function, MethodDefinition, ObjectProperty};
use oxc_ast_visit::Visit;

use crate::config::{NestingContext, NoNestedFunctionsRuleConfig};
use crate::rules::{NO_NESTED_FUNCTIONS_RULE_ID, Violation};
use crate::syntax::LineIndex;

const MESSAGE: &str = "Function is nested too deeply. Extract it to a top-level or module-scope declaration.";

pub fn check_file(
    file: &Path,
    program: &oxc_ast::ast::Program,
    line_index: &LineIndex,
    config: &NoNestedFunctionsRuleConfig,
) -> Vec<Violation> {
    let mut visitor = NestedFunctionVisitor {
        violations: Vec::new(),
        file,
        line_index,
        severity: config.severity,
        max_depth: config.max_depth,
        contexts: &config.contexts,
        current_depth: 0,
        in_class_method: false,
        in_object_method: false,
        _phantom: std::marker::PhantomData,
    };
    visitor.visit_program(program);
    visitor.violations
}

struct NestedFunctionVisitor<'a, 'f> {
    violations: Vec<Violation>,
    file: &'f Path,
    line_index: &'f LineIndex,
    severity: crate::config::Severity,
    max_depth: usize,
    contexts: &'a [NestingContext],
    current_depth: usize,
    in_class_method: bool,
    in_object_method: bool,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'f> NestedFunctionVisitor<'a, 'f> {
    fn should_count_context(&self, context: NestingContext) -> bool {
        self.contexts.contains(&context)
    }

    fn function_context(&self) -> NestingContext {
        if self.in_class_method {
            NestingContext::ClassMethod
        } else if self.in_object_method {
            NestingContext::ObjectMethod
        } else {
            NestingContext::Function
        }
    }

    fn check_and_visit_function(
        &mut self,
        func: &Function<'a>,
        context: NestingContext,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        let counts = self.should_count_context(context);
        if counts {
            self.current_depth += 1;
        }

        if self.current_depth > self.max_depth {
            let span = func.id.as_ref().map(|id| id.span).unwrap_or(func.span);
            let pos = self.line_index.position_for(span);
            let name = func
                .id
                .as_ref()
                .map(|id| id.name.as_str().to_string())
                .unwrap_or_else(|| "<anonymous>".to_string());
            let detail = Some(format!(
                "depth {} exceeds max-depth {}",
                self.current_depth, self.max_depth
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_NESTED_FUNCTIONS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some(name),
            });
        }

        oxc_ast_visit::walk::walk_function(self, func, flags);

        if counts {
            self.current_depth -= 1;
        }
    }
}

impl<'a, 'f> Visit<'a> for NestedFunctionVisitor<'a, 'f> {
    fn visit_function(
        &mut self,
        func: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        let context = self.function_context();
        self.check_and_visit_function(func, context, flags);
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        let counts = self.should_count_context(NestingContext::Arrow);
        if counts {
            self.current_depth += 1;
        }

        if self.current_depth > self.max_depth {
            let pos = self.line_index.position_for(arrow.span);
            let detail = Some(format!(
                "depth {} exceeds max-depth {}",
                self.current_depth, self.max_depth
            ));
            self.violations.push(Violation {
                file: self.file.to_path_buf(),
                span: Some(arrow.span),
                line: Some(pos.line),
                column: Some(pos.column),
                rule: NO_NESTED_FUNCTIONS_RULE_ID,
                message: MESSAGE,
                severity: self.severity,
                detail,
                subject: Some("<arrow>".to_string()),
            });
        }

        oxc_ast_visit::walk::walk_arrow_function_expression(self, arrow);

        if counts {
            self.current_depth -= 1;
        }
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        let saved = self.in_class_method;
        self.in_class_method = true;
        oxc_ast_visit::walk::walk_method_definition(self, method);
        self.in_class_method = saved;
    }

    fn visit_object_property(&mut self, prop: &ObjectProperty<'a>) {
        let saved = self.in_object_method;
        self.in_object_method = prop.method;
        oxc_ast_visit::walk::walk_object_property(self, prop);
        self.in_object_method = saved;
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use super::*;
    use crate::config::{NestingContext, NoNestedFunctionsRuleConfig, Severity};
    use crate::syntax::LineIndex;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::path::Path;

    fn run_check(source: &str, max_depth: usize) -> Vec<Violation> {
        run_check_with_contexts(source, max_depth, &[])
    }

    fn run_check_with_contexts(
        source: &str,
        max_depth: usize,
        contexts: &[NestingContext],
    ) -> Vec<Violation> {
        let allocator = Allocator::default();
        let line_index = LineIndex::new(source);
        let parser_return = Parser::new(&allocator, source, SourceType::ts()).parse();
        let program = parser_return.program;
        check_file(
            Path::new("test.ts"),
            &program,
            &line_index,
            &test_config(max_depth, contexts),
        )
    }

    fn test_config(max_depth: usize, contexts: &[NestingContext]) -> NoNestedFunctionsRuleConfig {
        let contexts = if contexts.is_empty() {
            vec![
                NestingContext::Function,
                NestingContext::Arrow,
                NestingContext::ClassMethod,
                NestingContext::ObjectMethod,
            ]
        } else {
            contexts.to_vec()
        };
        NoNestedFunctionsRuleConfig {
            severity: Severity::Warn,
            max_depth,
            contexts,
        }
    }

    #[test]
    fn allows_top_level_function() -> Result<()> {
        let source = "function foo() {}\n";
        let violations = run_check(source, 1);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn allows_one_level_of_nesting_with_max_depth_2() -> Result<()> {
        let source = "function outer() { function inner() {} }\n";
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_two_levels_of_nesting_with_max_depth_2() -> Result<()> {
        let source =
            "function outer() { function middle() { function inner() {} } }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("inner"));
    
        Ok(())}

    #[test]
    fn reports_arrow_functions_as_nesting() -> Result<()> {
        let source =
            "function outer() { const inner = () => { const deep = () => {} }; }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn reports_nested_arrows_in_arrow() -> Result<()> {
        let source = "const a = () => { const b = () => { const c = () => {} }; };\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
    
        Ok(())}

    #[test]
    fn allows_flat_callbacks() -> Result<()> {
        let source =
            "function outer() { [1, 2].map(x => x + 1); }\n";
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn reports_multiple_violations() -> Result<()> {
        let source = r#"function a() {
  function b() {
    function c() {}
    function d() {}
  }
}
"#;
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 2);
    
        Ok(())}

    #[test]
    fn depth_resets_after_function_ends() -> Result<()> {
        let source = r#"function first() {
  function nested() {}
}
function second() {
  function nested() {}
}
"#;
        let violations = run_check(source, 2);
        assert!(violations.is_empty());
    
        Ok(())}

    #[test]
    fn max_depth_1_reports_any_nesting() -> Result<()> {
        let source = "function outer() { function inner() {} }\n";
        let violations = run_check(source, 1);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("inner"));
    
        Ok(())}

    #[test]
    fn mixed_function_and_arrow_nesting() -> Result<()> {
        let source =
            "function outer() { const mid = () => { function deep() {} } }\n";
        let violations = run_check(source, 2);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject.as_deref(), Some("deep"));
    
        Ok(())}

    #[test]
    fn context_arrow_excluded_does_not_count_arrows() -> Result<()> {
        let source = r#"function outer() {
  [1, 2].map(x => {
    return function inner() {};
  });
}
"#;
        let violations = run_check_with_contexts(
            source,
            2,
            &[NestingContext::Function, NestingContext::ClassMethod, NestingContext::ObjectMethod],
        );
        assert!(violations.is_empty(), "arrow should not count as nesting level");

        Ok(())}

    #[test]
    fn context_arrow_excluded_arrow_inside_function_at_depth_2() -> Result<()> {
        let source = r#"function a() {
  function b() {
    const x = () => {};
  }
}
"#;
        let violations = run_check_with_contexts(
            source,
            2,
            &[NestingContext::Function],
        );
        assert!(violations.is_empty(), "arrow at depth 2 should not trigger when arrow is excluded");

        Ok(())}

    #[test]
    fn context_only_function_counts_class_methods() -> Result<()> {
        // class-method not in contexts, so method should not count as nesting level
        let source = r#"
function outer() {
  class Foo {
    bar() {
      function deeply() {}
    }
  }
}
"#;
        let violations = run_check_with_contexts(
            source,
            2,
            &[NestingContext::Function, NestingContext::Arrow, NestingContext::ObjectMethod],
        );
        assert!(violations.is_empty(), "class method should not count when class-method context is excluded");

        Ok(())}

    #[test]
    fn context_only_function_counts_object_methods() -> Result<()> {
        let source = r#"
function outer() {
  const obj = {
    method() {
      function deeply() {}
    },
  };
}
"#;
        let violations = run_check_with_contexts(
            source,
            2,
            &[NestingContext::Function, NestingContext::Arrow, NestingContext::ClassMethod],
        );
        assert!(violations.is_empty(), "object method should not count when object-method context is excluded");

        Ok(())}

    #[test]
    fn context_class_method_counts_as_nesting() -> Result<()> {
        let source = r#"
function outer() {
  class Foo {
    methodA() {
      class Bar {
        methodB() {}
      }
    }
  }
}
"#;
        let violations = run_check_with_contexts(source, 2, &[
            NestingContext::Function,
            NestingContext::ClassMethod,
            NestingContext::Arrow,
            NestingContext::ObjectMethod,
        ]);
        // function(outer) -> class-method(methodA) -> class-method(methodB) = depth 3, exceeds 2
        assert_eq!(violations.len(), 1);

        Ok(())}

    #[test]
    fn context_object_method_counts_as_nesting() -> Result<()> {
        let source = r#"
function outer() {
  const obj = {
    middle() {
      return function inner() {};
    },
  };
}
"#;
        let violations = run_check_with_contexts(source, 2, &[
            NestingContext::Function,
            NestingContext::ObjectMethod,
            NestingContext::Arrow,
            NestingContext::ClassMethod,
        ]);
        assert_eq!(violations.len(), 1);

        Ok(())}
}
