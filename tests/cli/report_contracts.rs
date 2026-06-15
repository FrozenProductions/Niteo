use crate::harness;
use anyhow::{Context, Result};
use serde_json::Value;

#[test]
fn json_report_has_summary_fields() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let summary = &parsed["summary"];

    assert!(summary["filesScanned"].is_number());
    assert!(summary["violations"].is_number());
    assert!(summary["errors"].is_number());
    assert!(summary["warnings"].is_number());
    assert!(summary["info"].is_number());
    assert!(summary["score"].is_number());
    assert!(summary["status"].is_string());
    Ok(())
}

#[test]
fn json_violation_has_required_fields() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    assert!(!violations.is_empty());

    for violation in violations {
        assert!(violation["file"].is_string());
        assert!(violation["rule"].is_string());
        assert!(violation["message"].is_string());
        assert!(violation["severity"].is_string());
    }
    Ok(())
}

#[test]
fn json_violation_severity_values_are_valid() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    let violations = parsed["violations"]
        .as_array()
        .context("expected violations array")?;

    let valid_severities = ["error", "warning", "info", "off"];
    for violation in violations {
        let severity = violation["severity"]
            .as_str()
            .context("expected severity string")?;
        assert!(
            valid_severities.contains(&severity),
            "unexpected severity: {severity}"
        );
    }
    Ok(())
}

#[test]
fn json_files_list_matches_scanned_files() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let files = parsed["files"].as_array().context("expected files array")?;
    let files_scanned = parsed["summary"]["filesScanned"]
        .as_u64()
        .context("expected files scanned count")?;

    assert_eq!(files.len() as u64, files_scanned);
    Ok(())
}

#[test]
fn sarif_has_correct_schema_and_version() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    assert_eq!(
        parsed["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    assert_eq!(parsed["version"], "2.1.0");
    Ok(())
}

#[test]
fn sarif_tool_driver_is_niteo() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let driver = &parsed["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "Niteo");
    assert!(driver["rules"].is_array());
    Ok(())
}

#[test]
fn sarif_results_match_violations() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let results = parsed["runs"][0]["results"]
        .as_array()
        .context("expected results array")?;
    assert!(!results.is_empty());

    for result in results {
        assert!(result["ruleId"].is_string());
        assert!(result["message"]["text"].is_string());
        assert!(result["locations"].is_array());
    }
    Ok(())
}

#[test]
fn sarif_rules_contain_emitted_rule_ids() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .context("expected rules array")?;
    let rule_ids: Vec<&str> = rules
        .iter()
        .map(|r| r["id"].as_str().context("expected rule id string"))
        .collect::<Result<Vec<_>, _>>()?;

    assert!(rule_ids.contains(&"no-console"));
    Ok(())
}

#[test]
fn sarif_summary_exists_under_properties() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let summary = &parsed["runs"][0]["properties"]["summary"];
    assert!(summary["filesScanned"].is_number());
    assert!(summary["violations"].is_number());
    Ok(())
}

#[test]
fn sarif_locations_have_physical_location() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "sarif"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    let results = parsed["runs"][0]["results"]
        .as_array()
        .context("expected results array")?;
    for result in results {
        let locations = result["locations"]
            .as_array()
            .context("expected locations array")?;
        assert!(!locations.is_empty());

        let physical = &locations[0]["physicalLocation"];
        assert!(physical["artifactLocation"]["uri"].is_string());
        assert!(physical["region"]["startLine"].is_number());
    }
    Ok(())
}

#[test]
fn clean_project_json_has_zero_violations() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;

    assert_eq!(
        parsed["summary"]["violations"]
            .as_u64()
            .context("expected violations count")?,
        0
    );
    assert_eq!(
        parsed["summary"]["errors"]
            .as_u64()
            .context("expected errors count")?,
        0
    );
    assert_eq!(
        parsed["summary"]["warnings"]
            .as_u64()
            .context("expected warnings count")?,
        0
    );
    assert_eq!(
        parsed["summary"]["score"]
            .as_u64()
            .context("expected score")?,
        100
    );
    Ok(())
}

#[test]
fn ndjson_first_record_is_summary() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let first_line = stdout
        .lines()
        .next()
        .context("expected at least one line")?;
    let parsed: Value = serde_json::from_str(first_line)?;

    assert_eq!(parsed["type"], "summary");
    assert!(parsed["filesScanned"].is_number());
    assert!(parsed["violations"].is_number());
    assert!(parsed["errors"].is_number());
    assert!(parsed["warnings"].is_number());
    assert!(parsed["info"].is_number());
    assert!(parsed["score"].is_number());
    assert!(parsed["status"].is_string());
    Ok(())
}

#[test]
fn ndjson_violation_records_have_required_fields() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
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
        let severity = violation["severity"]
            .as_str()
            .context("expected severity string")?;
        assert!(
            valid_severities.contains(&severity),
            "unexpected severity: {severity}"
        );
    }
    Ok(())
}

#[test]
fn ndjson_file_records_match_files_scanned() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;

    let mut file_lines = Vec::new();
    for line in stdout.lines() {
        let parsed: Value = serde_json::from_str(line)?;
        if parsed["type"] == "file" {
            file_lines.push(line);
        }
    }

    let summary_line = stdout
        .lines()
        .next()
        .context("expected at least one line")?;
    let summary: Value = serde_json::from_str(summary_line)?;
    let files_scanned = summary["filesScanned"]
        .as_u64()
        .context("expected files scanned count")?;

    assert_eq!(file_lines.len() as u64, files_scanned);
    Ok(())
}

#[test]
fn clean_project_ndjson_has_summary_and_no_violations() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    let output = harness::niteo_in_project(project.path())
        .args(["lint", "--format", "ndjson"])
        .output()?;

    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty());

    let summary: Value = serde_json::from_str(lines[0])?;
    assert_eq!(summary["type"], "summary");
    assert_eq!(
        summary["violations"]
            .as_u64()
            .context("expected violations count")?,
        0
    );

    let mut violation_lines = 0;
    for line in &lines {
        let parsed: Value = serde_json::from_str(line)?;
        if parsed["type"] == "violation" {
            violation_lines += 1;
        }
    }
    assert_eq!(violation_lines, 0);
    Ok(())
}
