use crate::harness;

#[test]
fn cache_flag_creates_cache_file() {
    let project = harness::copy_fixture("reports/clean").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let cache_path = project.path().join(".niteo").join("cache.json");
    assert!(cache_path.exists(), "cache file should be created");

    let contents = std::fs::read_to_string(&cache_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert_eq!(parsed["version"], 1);
    assert!(parsed["niteo_version"].is_string());
    assert!(parsed["files"].is_object());
}

#[test]
fn clear_cache_removes_cache_file() {
    let project = harness::copy_fixture("reports/clean").unwrap();
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
}

#[test]
fn cache_invalidates_when_file_changes() {
    let project = harness::copy_fixture("reports/clean").unwrap();
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let first_cache = std::fs::read_to_string(&cache_path).unwrap();
    let first_parsed: serde_json::Value = serde_json::from_str(&first_cache).unwrap();
    let first_files = first_parsed["files"].as_object().unwrap();
    let first_file_count = first_files.len();

    let ts_path = project.path().join("src/utils.ts");
    std::fs::write(&ts_path, "export const newValue = 1;\n").unwrap();

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let second_cache = std::fs::read_to_string(&cache_path).unwrap();
    let second_parsed: serde_json::Value = serde_json::from_str(&second_cache).unwrap();
    let second_files = second_parsed["files"].as_object().unwrap();
    let second_file_count = second_files.len();

    assert_eq!(
        second_file_count,
        first_file_count + 1,
        "cache should include the new file"
    );
}

#[test]
fn cache_reuses_entries_for_unchanged_files() {
    let project = harness::copy_fixture("reports/clean").unwrap();
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let first_cache = std::fs::read_to_string(&cache_path).unwrap();
    let first_parsed: serde_json::Value = serde_json::from_str(&first_cache).unwrap();

    let first_entry = first_parsed["files"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    let first_hash = first_entry["content_hash"].as_str().unwrap().to_string();

    harness::niteo_in_project(project.path())
        .args(["lint", "--cache"])
        .assert()
        .success();

    let second_cache = std::fs::read_to_string(&cache_path).unwrap();
    let second_parsed: serde_json::Value = serde_json::from_str(&second_cache).unwrap();

    let second_entry = second_parsed["files"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    let second_hash = second_entry["content_hash"].as_str().unwrap().to_string();

    assert_eq!(
        first_hash, second_hash,
        "unchanged files should keep same hash"
    );
}

#[test]
fn no_cache_does_not_create_cache_file() {
    let project = harness::copy_fixture("reports/clean").unwrap();
    let cache_path = project.path().join(".niteo").join("cache.json");

    harness::niteo_in_project(project.path())
        .args(["lint"])
        .assert()
        .success();

    assert!(
        !cache_path.exists(),
        "cache should not be created without --cache"
    );
}
