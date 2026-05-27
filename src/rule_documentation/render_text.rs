use super::model::RuleExplanation;

pub fn render_explanation_text(explanation: &RuleExplanation) -> String {
    let mut output = String::new();
    output.push_str(explanation.name);
    output.push('\n');
    output.push_str(&format!("severity: {}\n\n", explanation.severity.as_str()));
    output.push_str("Intent\n");
    output.push_str(explanation.intent);
    output.push_str("\n\nExamples\n");
    for example in &explanation.examples {
        output.push_str(&format!("- {}: {}\n", example.label, example.code));
    }
    output.push_str("\nConfig options\n");
    for option in &explanation.options {
        output.push_str(&format!("- {}: {}\n", option.name, option.description));
    }
    output.push_str("\nCurrent config\n");
    output.push_str(&format!(
        "- severity: {}\n",
        explanation.current_severity.as_str()
    ));
    for option in &explanation.current_options {
        output.push_str(&format!("- {option}\n"));
    }
    output
}
