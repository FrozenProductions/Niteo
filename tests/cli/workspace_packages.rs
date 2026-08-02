use crate::harness;
use anyhow::Result;
use serde_json::Value;
use std::fs;

fn build_workspace() -> Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let root = temp_dir.path();

    fs::write(
        root.join("package.json"),
        r#"{"name":"root","private":true,"workspaces":["packages/*"]}"#,
    )?;
    fs::write(
        root.join("niteo.toml"),
        r#"[rules.no-package-cycle]
severity = "warn"

[rules.no-private-package-import]
severity = "warn"
"#,
    )?;

    fs::create_dir_all(root.join("packages/app/src"))?;
    fs::create_dir_all(root.join("packages/ui/src"))?;
    fs::create_dir_all(root.join("packages/ui/internal"))?;
    fs::write(
        root.join("packages/app/package.json"),
        r#"{"name":"app","private":true}"#,
    )?;
    fs::write(
        root.join("packages/ui/package.json"),
        r#"{"name":"@scope/ui","exports":{".":"./src/index.ts"}}"#,
    )?;
    fs::write(
        root.join("packages/app/src/main.ts"),
        "import { Button } from '@scope/ui';\nimport { helper } from '@scope/ui/internal/helper';\n",
    )?;
    fs::write(
        root.join("packages/app/src/index.ts"),
        "export const App = 1;\n",
    )?;
    fs::write(
        root.join("packages/ui/src/index.ts"),
        "import { App } from 'app';\nexport const Button = 1;\n",
    )?;
    fs::write(
        root.join("packages/ui/internal/helper.ts"),
        "export const helper = 1;\n",
    )?;

    Ok(temp_dir)
}

fn run_lint_json(project: &std::path::Path) -> Result<Value> {
    let output = harness::niteo_in_project(project)
        .args(["lint", "--format", "json"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Value = serde_json::from_str(&stdout)?;
    Ok(parsed)
}

fn violations_for_file(parsed: &Value, file_suffix: &str, rule: &str) -> Vec<Value> {
    parsed
        .get("violations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|violation| {
            violation
                .get("file")
                .and_then(Value::as_str)
                .is_some_and(|file| file.ends_with(file_suffix))
                && violation.get("rule").and_then(Value::as_str) == Some(rule)
        })
        .collect()
}

#[test]
fn package_name_imports_resolve_to_package_entrypoints() -> Result<()> {
    let project = build_workspace()?;
    let parsed = run_lint_json(project.path())?;

    let main_cycle = violations_for_file(&parsed, "packages/app/src/main.ts", "no-package-cycle");
    assert_eq!(
        main_cycle.len(),
        1,
        "package-name import must be part of the detected cycle"
    );
    assert_eq!(
        main_cycle[0].get("subject").and_then(Value::as_str),
        Some("@scope/ui")
    );
    assert!(
        main_cycle[0]
            .get("detail")
            .and_then(Value::as_str)
            .is_some_and(|detail| detail.contains("@scope/ui") && detail.contains("app")),
        "cycle detail must name both packages"
    );

    let ui_cycle = violations_for_file(&parsed, "packages/ui/src/index.ts", "no-package-cycle");
    assert_eq!(ui_cycle.len(), 1);
    assert_eq!(
        ui_cycle[0].get("subject").and_then(Value::as_str),
        Some("app")
    );

    Ok(())
}

#[test]
fn non_exported_subpath_import_is_rejected() -> Result<()> {
    let project = build_workspace()?;
    let parsed = run_lint_json(project.path())?;

    let private = violations_for_file(
        &parsed,
        "packages/app/src/main.ts",
        "no-private-package-import",
    );
    assert_eq!(private.len(), 1, "non-exported subpath must be reported");
    assert_eq!(
        private[0].get("subject").and_then(Value::as_str),
        Some("@scope/ui/internal/helper")
    );

    let exported = violations_for_file(
        &parsed,
        "packages/ui/src/index.ts",
        "no-private-package-import",
    );
    assert!(
        exported.is_empty(),
        "exported package import must not be reported"
    );

    Ok(())
}
