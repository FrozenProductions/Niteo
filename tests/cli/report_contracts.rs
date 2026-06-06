use crate::harness;
use serde_json::Value;

#[test]
fn json_report_has_summary_fields() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let summary = &parsed["summary"];

    assert!(summary["filesScanned"].is_number());
    assert!(summary["violations"].is_number());
    assert!(summary["errors"].is_number());
    assert!(summary["warnings"].is_number());
    assert!(summary["info"].is_number());
    assert!(summary["score"].is_number());
    assert!(summary["status"].is_string());
}

#[test]
fn json_violation_has_required_fields() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    assert!(!violations.is_empty());

    for violation in violations {
        assert!(violation["file"].is_string());
        assert!(violation["rule"].is_string());
        assert!(violation["message"].is_string());
        assert!(violation["severity"].is_string());
    }
}

#[test]
fn json_violation_severity_values_are_valid() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let violations = parsed["violations"].as_array().unwrap();

    let valid_severities = ["error", "warning", "info", "off"];
    for violation in violations {
        let severity = violation["severity"].as_str().unwrap();
        assert!(
            valid_severities.contains(&severity),
            "unexpected severity: {severity}"
        );
    }
}

#[test]
fn json_files_list_matches_scanned_files() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let files = parsed["files"].as_array().unwrap();
    let files_scanned = parsed["summary"]["filesScanned"].as_u64().unwrap();

    assert_eq!(files.len() as u64, files_scanned);
}

#[test]
fn sarif_has_correct_schema_and_version() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(
        parsed["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(parsed["version"], "2.1.0");
}

#[test]
fn sarif_tool_driver_is_niteo() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let driver = &parsed["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "Niteo");
    assert!(driver["rules"].is_array());
}

#[test]
fn sarif_results_match_violations() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let results = parsed["runs"][0]["results"].as_array().unwrap();
    assert!(!results.is_empty());

    for result in results {
        assert!(result["ruleId"].is_string());
        assert!(result["message"]["text"].is_string());
        assert!(result["locations"].is_array());
    }
}

#[test]
fn sarif_rules_contain_emitted_rule_ids() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    let rule_ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();

    assert!(rule_ids.contains(&"no-console"));
}

#[test]
fn sarif_summary_exists_under_properties() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let summary = &parsed["runs"][0]["properties"]["summary"];
    assert!(summary["filesScanned"].is_number());
    assert!(summary["violations"].is_number());
}

#[test]
fn sarif_locations_have_physical_location() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let results = parsed["runs"][0]["results"].as_array().unwrap();
    for result in results {
        let locations = result["locations"].as_array().unwrap();
        assert!(!locations.is_empty());

        let physical = &locations[0]["physicalLocation"];
        assert!(physical["artifactLocation"]["uri"].is_string());
        assert!(physical["region"]["startLine"].is_number());
    }
}

#[test]
fn clean_project_json_has_zero_violations() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["summary"]["violations"].as_u64().unwrap(), 0);
    assert_eq!(parsed["summary"]["errors"].as_u64().unwrap(), 0);
    assert_eq!(parsed["summary"]["warnings"].as_u64().unwrap(), 0);
    assert_eq!(parsed["summary"]["score"].as_u64().unwrap(), 100);
}

#[test]
fn ndjson_first_record_is_summary() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let first_line = stdout.lines().next().unwrap();
    let parsed: Value = serde_json::from_str(first_line).unwrap();

    assert_eq!(parsed["type"], "summary");
    assert!(parsed["filesScanned"].is_number());
    assert!(parsed["violations"].is_number());
    assert!(parsed["errors"].is_number());
    assert!(parsed["warnings"].is_number());
    assert!(parsed["info"].is_number());
    assert!(parsed["score"].is_number());
    assert!(parsed["status"].is_string());
}

#[test]
fn ndjson_violation_records_have_required_fields() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let violations: Vec<Value> = stdout
        .lines()
        .filter_map(|line| {
            let parsed: Value = serde_json::from_str(line).ok()?;
            if parsed["type"] == "violation" {
                Some(parsed)
            } else {
                None
            }
        })
        .collect();

    assert!(!violations.is_empty());

    for violation in &violations {
        assert!(violation["file"].is_string());
        assert!(violation["rule"].is_string());
        assert!(violation["message"].is_string());
        assert!(violation["severity"].is_string());
    }

    let valid_severities = ["error", "warning", "info", "off"];
    for violation in &violations {
        let severity = violation["severity"].as_str().unwrap();
        assert!(
            valid_severities.contains(&severity),
            "unexpected severity: {severity}"
        );
    }
}

#[test]
fn ndjson_file_records_match_files_scanned() {
    let project = harness::copy_fixture("reports/basic").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();

    let file_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            let parsed: Value = serde_json::from_str(line).unwrap();
            parsed["type"] == "file"
        })
        .collect();

    let summary_line = stdout.lines().next().unwrap();
    let summary: Value = serde_json::from_str(summary_line).unwrap();
    let files_scanned = summary["filesScanned"].as_u64().unwrap();

    assert_eq!(file_lines.len() as u64, files_scanned);
}

#[test]
fn clean_project_ndjson_has_summary_and_no_violations() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty());

    let summary: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["violations"].as_u64().unwrap(), 0);

    let violation_lines = lines
        .iter()
        .filter(|line| {
            let parsed: Value = serde_json::from_str(line).unwrap();
            parsed["type"] == "violation"
        })
        .count();
    assert_eq!(violation_lines, 0);
}
