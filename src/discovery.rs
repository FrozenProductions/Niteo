use anyhow::{Result, bail};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use crate::config::GitignoreConfig;
use crate::syntax;
use crate::tsconfig::TsConfig;

pub fn discover_files(
    project_root: &Path,
    scope: Option<&Path>,
    gitignore_config: &GitignoreConfig,
    tsconfig: Option<&TsConfig>,
) -> Result<Vec<PathBuf>> {
    if !project_root.exists() {
        bail!("root path does not exist: {}", project_root.display());
    }

    let scan_root = scope.unwrap_or(project_root);
    validate_scope(project_root, scan_root)?;

    let mut builder = WalkBuilder::new(scan_root);
    builder.git_ignore(gitignore_config.enabled);
    builder.hidden(false);
    builder.follow_links(false);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && syntax::is_typescript_file(path)
            && tsconfig.is_none_or(|config| config.is_included(path))
        {
            files.push(path.to_path_buf());
        }
    }

    // Deterministic order for stable graph and cache inputs.
    files.sort();
    Ok(files)
}

fn validate_scope(project_root: &Path, scan_root: &Path) -> Result<()> {
    if !scan_root.starts_with(project_root) {
        bail!("scope is outside the project root: {}", scan_root.display());
    }
    if !scan_root.exists() {
        bail!("scope path does not exist: {}", scan_root.display());
    }
    if !scan_root.is_file() && !scan_root.is_dir() {
        bail!(
            "scope is neither a file nor a directory: {}",
            scan_root.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    fn gitignore_disabled() -> GitignoreConfig {
        GitignoreConfig { enabled: false }
    }

    #[test]
    fn discovers_all_typescript_files_without_tsconfig() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("dist"))?;
        std::fs::write(root.join("src/app.ts"), "")?;
        std::fs::write(root.join("dist/bundle.ts"), "")?;

        let files = discover_files(root, None, &gitignore_disabled(), None)?;
        assert_eq!(files.len(), 2);
        Ok(())
    }

    #[test]
    fn respects_tsconfig_include() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("dist"))?;
        std::fs::write(root.join("src/app.ts"), "")?;
        std::fs::write(root.join("dist/bundle.ts"), "")?;
        std::fs::write(root.join("tsconfig.json"), r#"{"include": ["src"]}"#)?;

        let tsconfig =
            crate::tsconfig::discover_and_parse(root)?.context("tsconfig should parse")?;
        let files = discover_files(root, None, &gitignore_disabled(), Some(&tsconfig))?;
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/app.ts"));
        Ok(())
    }

    #[test]
    fn respects_tsconfig_exclude() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("dist"))?;
        std::fs::write(root.join("src/app.ts"), "")?;
        std::fs::write(root.join("dist/bundle.ts"), "")?;
        std::fs::write(root.join("tsconfig.json"), r#"{"exclude": ["dist"]}"#)?;

        let tsconfig =
            crate::tsconfig::discover_and_parse(root)?.context("tsconfig should parse")?;
        let files = discover_files(root, None, &gitignore_disabled(), Some(&tsconfig))?;
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/app.ts"));
        Ok(())
    }

    #[test]
    fn nested_directory_scope_returns_only_its_typescript_files() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/components"))?;
        std::fs::create_dir_all(root.join("src/utils"))?;
        std::fs::write(root.join("src/components/button.ts"), "")?;
        std::fs::write(root.join("src/components/panel.tsx"), "")?;
        std::fs::write(root.join("src/utils/format.ts"), "")?;
        std::fs::write(root.join("src/main.ts"), "")?;

        let scope = root.join("src/components");
        let files = discover_files(root, Some(&scope), &gitignore_disabled(), None)?;
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.starts_with(&scope)));
        Ok(())
    }

    #[test]
    fn file_scope_returns_only_that_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(root.join("src/one.ts"), "")?;
        std::fs::write(root.join("src/two.ts"), "")?;

        let scope = root.join("src/one.ts");
        let files = discover_files(root, Some(&scope), &gitignore_disabled(), None)?;
        assert_eq!(files, vec![scope]);
        Ok(())
    }

    #[test]
    fn file_scope_respects_tsconfig_include() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("dist"))?;
        std::fs::write(root.join("src/app.ts"), "")?;
        std::fs::write(root.join("dist/bundle.ts"), "")?;
        std::fs::write(root.join("tsconfig.json"), r#"{"include": ["src"]}"#)?;

        let tsconfig =
            crate::tsconfig::discover_and_parse(root)?.context("tsconfig should parse")?;

        let included = root.join("src/app.ts");
        let files = discover_files(
            root,
            Some(&included),
            &gitignore_disabled(),
            Some(&tsconfig),
        )?;
        assert_eq!(files, vec![included]);

        let excluded = root.join("dist/bundle.ts");
        let files = discover_files(
            root,
            Some(&excluded),
            &gitignore_disabled(),
            Some(&tsconfig),
        )?;
        assert!(files.is_empty());
        Ok(())
    }

    #[test]
    fn scope_outside_project_root_fails() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        let outside = tempfile::tempdir()?;
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(root.join("src/app.ts"), "")?;

        let error = discover_files(root, Some(outside.path()), &gitignore_disabled(), None)
            .err()
            .context("expected scope validation to fail")?
            .to_string();
        assert!(error.contains("outside the project root"));
        Ok(())
    }

    #[test]
    fn nonexistent_scope_fails() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        let scope = root.join("missing");

        let error = discover_files(root, Some(&scope), &gitignore_disabled(), None)
            .err()
            .context("expected scope validation to fail")?
            .to_string();
        assert!(error.contains("does not exist"));
        Ok(())
    }

    #[test]
    fn file_order_is_stable_across_runs() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("src/deep"))?;
        std::fs::write(root.join("src/zeta.ts"), "")?;
        std::fs::write(root.join("src/alpha.ts"), "")?;
        std::fs::write(root.join("src/deep/mid.ts"), "")?;

        let first = discover_files(root, None, &gitignore_disabled(), None)?;
        let second = discover_files(root, None, &gitignore_disabled(), None)?;
        assert_eq!(first, second);

        let mut sorted = first.clone();
        sorted.sort();
        assert_eq!(first, sorted);
        Ok(())
    }
}
