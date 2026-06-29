pub mod json;
pub mod model;
pub mod ndjson;
pub mod sarif;
pub mod summary;
pub mod suppressions;
pub mod text;

pub use model::Report;
pub use summary::{BOLD, DIM, GREEN, RED, RESET, YELLOW};
pub use suppressions::render_suppression_report_text;

#[cfg(test)]
mod tests {

    use crate::config::{FailurePolicy, FailureThreshold, Severity};
    use crate::report::model::Report;
    use crate::rules::Violation;
    use anyhow::Result;
    use std::path::PathBuf;

    fn make_violation(severity: Severity) -> Violation {
        Violation {
            file: PathBuf::from("test.ts"),
            span: None,
            line: Some(1),
            column: Some(1),
            rule: "test-rule",
            message: "test message",
            severity,
            detail: None,
            subject: None,
        }
    }

    fn policy_with_default(threshold: FailureThreshold) -> FailurePolicy {
        FailurePolicy {
            default: threshold,
            rules: std::collections::HashMap::new(),
            categories: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_failure_threshold_error() -> Result<()> {
        let report = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report.has_findings_matching(&policy_with_default(FailureThreshold::Error)));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(!report_warn.has_findings_matching(&policy_with_default(FailureThreshold::Error)));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(!report_info.has_findings_matching(&policy_with_default(FailureThreshold::Error)));

        Ok(())
    }

    #[test]
    fn test_failure_threshold_warn() -> Result<()> {
        let report_error = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report_error.has_findings_matching(&policy_with_default(FailureThreshold::Warn)));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(report_warn.has_findings_matching(&policy_with_default(FailureThreshold::Warn)));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(!report_info.has_findings_matching(&policy_with_default(FailureThreshold::Warn)));

        Ok(())
    }

    #[test]
    fn test_failure_threshold_any() -> Result<()> {
        let report_error = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report_error.has_findings_matching(&policy_with_default(FailureThreshold::Any)));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(report_warn.has_findings_matching(&policy_with_default(FailureThreshold::Any)));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(report_info.has_findings_matching(&policy_with_default(FailureThreshold::Any)));

        let report_off = Report::new(vec![], vec![make_violation(Severity::Off)]);
        assert!(!report_off.has_findings_matching(&policy_with_default(FailureThreshold::Any)));

        Ok(())
    }
}
