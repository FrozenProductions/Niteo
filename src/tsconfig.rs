use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TsConfig {
    pub base_url: PathBuf,
    pub aliases: Vec<ResolvedPathAlias>,
}

impl Default for TsConfig {
    fn default() -> Self {
        Self {
            base_url: PathBuf::from("."),
            aliases: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPathAlias {
    pub pattern: String,
    pub prefix: String,
    pub suffix: String,
    pub targets: Vec<PathTargetPattern>,
}

#[derive(Debug, Clone)]
pub struct PathTargetPattern {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Deserialize)]
struct RawTsConfig {
    #[serde(rename = "compilerOptions")]
    compiler_options: Option<RawCompilerOptions>,
}

#[derive(Deserialize)]
struct RawCompilerOptions {
    #[serde(rename = "baseUrl")]
    base_url: Option<String>,
    paths: Option<HashMap<String, Vec<String>>>,
}

pub fn discover_and_parse(workspace: &Path) -> Result<Option<TsConfig>> {
    let path = workspace.join("tsconfig.json");
    if !path.exists() {
        return Ok(None);
    }
    parse_file(&path).map(Some)
}

fn parse_file(path: &Path) -> Result<TsConfig> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let raw: RawTsConfig = serde_json::from_str(&source)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let config_dir = path.parent().unwrap_or(Path::new("."));

    let base_url = match raw
        .compiler_options
        .as_ref()
        .and_then(|co| co.base_url.as_deref())
    {
        Some(relative) => config_dir.join(relative),
        None => config_dir.to_path_buf(),
    };

    let paths = raw
        .compiler_options
        .and_then(|co| co.paths)
        .unwrap_or_default();

    let mut aliases = Vec::new();
    for (pattern, targets) in paths {
        let (prefix, suffix) = split_pattern(&pattern);
        let resolved_targets = targets
            .iter()
            .map(|target| {
                let (target_prefix, target_suffix) = split_pattern(target);
                PathTargetPattern {
                    prefix: target_prefix,
                    suffix: target_suffix,
                }
            })
            .collect();

        aliases.push(ResolvedPathAlias {
            pattern,
            prefix,
            suffix,
            targets: resolved_targets,
        });
    }

    Ok(TsConfig { base_url, aliases })
}

fn split_pattern(pattern: &str) -> (String, String) {
    if let Some(pos) = pattern.find('*') {
        let prefix = pattern.get(..pos).unwrap_or("").to_string();
        let suffix = pattern.get(pos + 1..).unwrap_or("").to_string();
        (prefix, suffix)
    } else {
        (pattern.to_string(), String::new())
    }
}

pub fn match_alias<'a>(alias: &ResolvedPathAlias, specifier: &'a str) -> Option<&'a str> {
    if alias.pattern.contains('*') {
        if specifier.starts_with(&alias.prefix) && specifier.ends_with(&alias.suffix) {
            let wildcard_start = alias.prefix.len();
            let wildcard_end = specifier.len().saturating_sub(alias.suffix.len());
            if wildcard_end >= wildcard_start {
                return Some(specifier.get(wildcard_start..wildcard_end).unwrap_or(""));
            }
        }
    } else if specifier == alias.pattern {
        return Some("");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple_pattern() -> Result<()> {
        let (prefix, suffix) = split_pattern("@/*");
        assert_eq!(prefix, "@/");
        assert_eq!(suffix, "");
        Ok(())
    }

    #[test]
    fn split_pattern_with_suffix() -> Result<()> {
        let (prefix, suffix) = split_pattern("@features/*/utils");
        assert_eq!(prefix, "@features/");
        assert_eq!(suffix, "/utils");
        Ok(())
    }

    #[test]
    fn split_star_only() -> Result<()> {
        let (prefix, suffix) = split_pattern("*");
        assert_eq!(prefix, "");
        assert_eq!(suffix, "");
        Ok(())
    }

    #[test]
    fn split_no_wildcard() -> Result<()> {
        let (prefix, suffix) = split_pattern("react");
        assert_eq!(prefix, "react");
        assert_eq!(suffix, "");
        Ok(())
    }

    #[test]
    fn parses_tsconfig_with_paths() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("tsconfig.json");
        std::fs::write(
            &config_path,
            r#"{
                "compilerOptions": {
                    "baseUrl": ".",
                    "paths": {
                        "@/*": ["src/*"]
                    }
                }
            }"#,
        )?;

        let tsconfig = parse_file(&config_path)?;
        assert_eq!(tsconfig.aliases.len(), 1);
        assert_eq!(tsconfig.aliases[0].prefix, "@/");
        assert_eq!(tsconfig.aliases[0].suffix, "");
        assert_eq!(tsconfig.aliases[0].targets.len(), 1);
        assert_eq!(tsconfig.aliases[0].targets[0].prefix, "src/");
        assert_eq!(tsconfig.aliases[0].targets[0].suffix, "");
        Ok(())
    }

    #[test]
    fn parses_minimal_tsconfig() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("tsconfig.json");
        std::fs::write(
            &config_path,
            r#"{
                "compilerOptions": {
                    "baseUrl": "src"
                }
            }"#,
        )?;

        let tsconfig = parse_file(&config_path)?;
        assert!(tsconfig.base_url.ends_with("src"));
        assert!(tsconfig.aliases.is_empty());
        Ok(())
    }

    #[test]
    fn parses_tsconfig_without_compiler_options() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let config_path = dir.path().join("tsconfig.json");
        std::fs::write(&config_path, r#"{}"#)?;

        let tsconfig = parse_file(&config_path)?;
        assert_eq!(tsconfig.aliases.len(), 0);
        Ok(())
    }

    #[test]
    fn discover_returns_none_when_no_tsconfig() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let result = discover_and_parse(dir.path())?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn discover_finds_tsconfig() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("tsconfig.json"),
            r#"{"compilerOptions": {"baseUrl": "."}}"#,
        )?;

        let result = discover_and_parse(dir.path())?;
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn match_alias_wildcard_captures_middle() -> Result<()> {
        let alias = ResolvedPathAlias {
            pattern: "@/*".into(),
            prefix: "@/".into(),
            suffix: "".into(),
            targets: vec![],
        };
        assert_eq!(match_alias(&alias, "@/shared/date"), Some("shared/date"));
        assert_eq!(match_alias(&alias, "lodash"), None);
        Ok(())
    }

    #[test]
    fn match_alias_exact_does_not_match_prefix() -> Result<()> {
        let alias = ResolvedPathAlias {
            pattern: "react".into(),
            prefix: "react".into(),
            suffix: "".into(),
            targets: vec![],
        };
        assert_eq!(match_alias(&alias, "react"), Some(""));
        assert_eq!(match_alias(&alias, "react-dom"), None);
        Ok(())
    }

    #[test]
    fn match_alias_with_suffix_pattern() -> Result<()> {
        let alias = ResolvedPathAlias {
            pattern: "@features/*/utils".into(),
            prefix: "@features/".into(),
            suffix: "/utils".into(),
            targets: vec![],
        };
        assert_eq!(match_alias(&alias, "@features/auth/utils"), Some("auth"));
        assert_eq!(
            match_alias(&alias, "@features/nested/dir/utils"),
            Some("nested/dir")
        );
        assert_eq!(match_alias(&alias, "@features/auth/other"), None);
        Ok(())
    }
}
