pub mod json;
pub mod model;
pub mod ndjson;
pub mod sarif;
pub mod summary;
pub mod suppressions;
pub mod text;

pub use model::{FailureThreshold, Report};
pub use summary::{BOLD, DIM, GREEN, RED, RESET, YELLOW};
pub use suppressions::render_suppression_report_text;

#[cfg(test)]
mod tests {
    use crate::config::Severity;
    use crate::report::model::FailureThreshold;
    use crate::report::model::Report;
    use crate::rules::Violation;
    use std::path::PathBuf;

    fn make_violation(severity: Severity) -> Violation {
        Violation {
            file: PathBuf::from("test.ts"),
            line: Some(1),
            column: Some(1),
            rule: "test-rule",
            message: "test message",
            severity,
            detail: None,
            subject: None,
        }
    }

    #[test]
    fn test_failure_threshold_error() {
        let report = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report.has_findings_at_or_above(FailureThreshold::Error));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(!report_warn.has_findings_at_or_above(FailureThreshold::Error));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(!report_info.has_findings_at_or_above(FailureThreshold::Error));
    }

    #[test]
    fn test_failure_threshold_warn() {
        let report_error = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report_error.has_findings_at_or_above(FailureThreshold::Warn));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(report_warn.has_findings_at_or_above(FailureThreshold::Warn));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(!report_info.has_findings_at_or_above(FailureThreshold::Warn));
    }

    #[test]
    fn test_failure_threshold_any() {
        let report_error = Report::new(vec![], vec![make_violation(Severity::Error)]);
        assert!(report_error.has_findings_at_or_above(FailureThreshold::Any));

        let report_warn = Report::new(vec![], vec![make_violation(Severity::Warn)]);
        assert!(report_warn.has_findings_at_or_above(FailureThreshold::Any));

        let report_info = Report::new(vec![], vec![make_violation(Severity::Info)]);
        assert!(report_info.has_findings_at_or_above(FailureThreshold::Any));

        let report_off = Report::new(vec![], vec![make_violation(Severity::Off)]);
        assert!(!report_off.has_findings_at_or_above(FailureThreshold::Any));
    }
}
