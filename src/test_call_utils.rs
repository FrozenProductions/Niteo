use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;
use oxc_span::Span;

const TEST_FUNCTION_NAMES: &[&str] = &["describe", "it", "test"];

pub struct TestPropertyCall {
    pub function_name: String,
    pub member_span: Span,
    pub object_span: Span,
    pub property_span: Span,
}

struct TestPropertyCollector<'a, 'b> {
    property_name: &'b str,
    calls: Vec<TestPropertyCall>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, 'b> Visit<'a> for TestPropertyCollector<'a, 'b> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(member) = &call.callee
            && let Expression::Identifier(object) = &member.object
            && TEST_FUNCTION_NAMES.contains(&object.name.as_str())
            && member.property.name == self.property_name
        {
            self.calls.push(TestPropertyCall {
                function_name: object.name.to_string(),
                member_span: member.span,
                object_span: object.span,
                property_span: member.property.span,
            });
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }
}

pub fn collect_test_property_calls(
    program: &oxc_ast::ast::Program,
    property_name: &str,
) -> Vec<TestPropertyCall> {
    let mut collector = TestPropertyCollector {
        property_name,
        calls: Vec::new(),
        _phantom: std::marker::PhantomData,
    };
    collector.visit_program(program);
    collector.calls
}
