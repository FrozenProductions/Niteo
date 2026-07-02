use crate::harness;
use anyhow::{Context, Result};

#[test]
fn cache_flag_creates_cache_file() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let cache_path = project.path().join(".niteo").join("cache.json");
    assert!(cache_path.exists(), "cache file should be created");

    let contents = std::fs::read_to_string(&cache_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&contents)?;
    assert_eq!(parsed["version"], 3);
    assert!(parsed["niteo_version"].is_string());
    assert!(parsed["files"].is_object());
    Ok(())
}

#[test]
fn clear_cache_removes_cache_file() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();
    assert!(cache_path.exists());

    harness::niteo_in_project(project.path())
        .args(["lint", "--clear-cache"])
        .assert()
        .success();

    assert!(
        !cache_path.exists(),
        "cache file should be removed after --clear-cache"
    );
    Ok(())
}

#[test]
fn cache_invalidates_when_file_changes() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let first_cache = std::fs::read_to_string(&cache_path)?;
    let first_parsed: serde_json::Value = serde_json::from_str(&first_cache)?;
    let first_files = first_parsed["files"]
        .as_object()
        .context("expected files object")?;
    let first_file_count = first_files.len();

    let ts_path = project.path().join("src/utils.ts");
    std::fs::write(&ts_path, "export const newValue = 1;\n")?;

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let second_cache = std::fs::read_to_string(&cache_path)?;
    let second_parsed: serde_json::Value = serde_json::from_str(&second_cache)?;
    let second_files = second_parsed["files"]
        .as_object()
        .context("expected files object")?;
    let second_file_count = second_files.len();

    assert_eq!(
        second_file_count,
        first_file_count + 1,
        "cache should include the new file"
    );
    Ok(())
}

#[test]
fn cache_reuses_entries_for_unchanged_files() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let first_cache = std::fs::read_to_string(&cache_path)?;
    let first_parsed: serde_json::Value = serde_json::from_str(&first_cache)?;

    let first_entry = first_parsed["files"]
        .as_object()
        .context("expected files object")?
        .values()
        .next()
        .context("expected at least one file entry")?
        .clone();
    let first_hash = first_entry["content_hash"]
        .as_str()
        .context("expected content hash string")?
        .to_string();

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let second_cache = std::fs::read_to_string(&cache_path)?;
    let second_parsed: serde_json::Value = serde_json::from_str(&second_cache)?;

    let second_entry = second_parsed["files"]
        .as_object()
        .context("expected files object")?
        .values()
        .next()
        .context("expected at least one file entry")?
        .clone();
    let second_hash = second_entry["content_hash"]
        .as_str()
        .context("expected content hash string")?
        .to_string();

    assert_eq!(
        first_hash, second_hash,
        "unchanged files should keep same hash"
    );
    Ok(())
}

#[test]
fn cache_writes_and_reuses_violations() -> Result<()> {
    let project = harness::copy_fixture("reports/basic")?;
    let cache_path = project.path().join(".niteo").join("cache.json");

    let _ = harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert();

    let first_cache = std::fs::read_to_string(&cache_path)?;
    let first_parsed: serde_json::Value = serde_json::from_str(&first_cache)?;
    let first_files = first_parsed["files"]
        .as_object()
        .context("expected files object")?;

    let console_entry = first_files
        .values()
        .find(|entry| {
            entry["violations"]
                .as_array()
                .map(|violations| {
                    violations
                        .iter()
                        .any(|v| v["rule"].as_str() == Some("no-console"))
                })
                .unwrap_or(false)
        })
        .context("expected a cache entry with no-console violations")?;
    let violations_array = console_entry["violations"]
        .as_array()
        .context("expected violations to be an array")?;
    assert!(!violations_array.is_empty());

    let _ = harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert();

    let second_cache = std::fs::read_to_string(&cache_path)?;
    let second_parsed: serde_json::Value = serde_json::from_str(&second_cache)?;
    let second_files = second_parsed["files"]
        .as_object()
        .context("expected files object")?;
    let second_console_entry = second_files
        .values()
        .find(|entry| {
            entry["violations"]
                .as_array()
                .map(|violations| {
                    violations
                        .iter()
                        .any(|v| v["rule"].as_str() == Some("no-console"))
                })
                .unwrap_or(false)
        })
        .context("expected a cache entry with no-console violations on second run")?;

    assert_eq!(
        console_entry["violations"], second_console_entry["violations"],
        "unchanged files should keep same violations"
    );
    Ok(())
}

#[test]
fn no_cache_does_not_create_cache_file() -> Result<()> {
    let project = harness::copy_fixture("reports/clean")?;
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();

    assert!(
        !cache_path.exists(),
        "cache should not be created without --cache"
    );
    Ok(())
}
