use anyhow::{Result, bail};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use crate::config::GitignoreConfig;
use crate::syntax;
use crate::tsconfig::TsConfig;

pub fn discover_files(
    root: &Path,
    scope: Option<&Path>,
    gitignore_config: &GitignoreConfig,
    tsconfig: Option<&TsConfig>,
) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        bail!("root path does not exist: {}", root.display());
    }

    let mut builder = WalkBuilder::new(root);
    builder.git_ignore(gitignore_config.enabled);
    builder.hidden(false);
    builder.follow_links(false);

    if let Some(scope) = scope {
        let scope = scope.to_path_buf();
        builder.filter_entry(move |entry| entry.path().starts_with(&scope));
    }

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

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let tsconfig = crate::tsconfig::discover_and_parse(root)?.expect("tsconfig should parse");
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

        let tsconfig = crate::tsconfig::discover_and_parse(root)?.expect("tsconfig should parse");
        let files = discover_files(root, None, &gitignore_disabled(), Some(&tsconfig))?;
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/app.ts"));
        Ok(())
    }
}
