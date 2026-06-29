use std::path::{Path, PathBuf};

use crate::config::{FailurePolicy, Severity};
use crate::diagnostics::Diagnostic;
use crate::ignore::SuppressionReport;
use crate::rules::Violation;

pub fn count_by_severity(violations: &[Violation], severity: Severity) -> usize {
    violations
        .iter()
        .filter(|violation| violation.severity == severity)
        .count()
}

pub fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
        Severity::Info => "info",
        Severity::Off => "off",
    }
}

pub fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warning",
        Severity::Info => "note",
        Severity::Off => "none",
    }
}

pub fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

pub fn with_record_type(mut value: serde_json::Value, record_type: &str) -> serde_json::Value {
    if let serde_json::Value::Object(ref mut object) = value {
        object.insert("type".to_string(), serde_json::json!(record_type));
    }
    value
}

#[derive(Debug, Clone)]
pub struct Report {
    pub files: Vec<PathBuf>,
    pub violations: Vec<Violation>,
    pub suppression_report: Option<SuppressionReport>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn new(files: Vec<PathBuf>, violations: Vec<Violation>) -> Self {
        Self {
            files,
            violations,
            suppression_report: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_suppression_report(mut self, report: SuppressionReport) -> Self {
        self.suppression_report = Some(report);
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn has_findings_matching(&self, policy: &FailurePolicy) -> bool {
        self.violations.iter().any(|violation| {
            policy
                .threshold_for(violation.rule)
                .includes(violation.severity)
        })
    }

    pub fn error_count(&self) -> usize {
        count_by_severity(&self.violations, Severity::Error)
    }

    pub fn warning_count(&self) -> usize {
        count_by_severity(&self.violations, Severity::Warn)
    }

    pub fn info_count(&self) -> usize {
        count_by_severity(&self.violations, Severity::Info)
    }
}
