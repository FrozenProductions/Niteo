pub mod build;
pub mod extract;
pub mod helpers;
pub mod model;
pub(crate) mod serialization;
pub mod topology;

pub use build::{build_import_graph, build_import_graph_with_cache};
pub use model::{ImportEdge, ImportGraph, ImportKind};

#[cfg(test)]
pub use build::{build_import_graph_from_sources, build_import_graph_from_sources_with_workspace};

impl ImportGraph {
    pub fn format_dot(&self) -> String {
        serialization::GraphFormatter::new(self).to_dot()
    }
}

#[cfg(test)]
mod tests {

    use anyhow::{Context, Result};
    use std::path::{Path, PathBuf};

    use crate::config::structure::DomainConfig;
    use crate::tsconfig::{PathTargetPattern, ResolvedPathAlias, TsConfig};

    use super::*;

    #[test]
    fn resolves_aliased_import_through_graph() -> Result<()> {
        let tsconfig = TsConfig::new(
            PathBuf::from("/repo"),
            vec![ResolvedPathAlias {
                pattern: "@/*".into(),
                prefix: "@/".into(),
                suffix: "".into(),
                targets: vec![PathTargetPattern {
                    prefix: "src/".into(),
                    suffix: "".into(),
                }],
            }],
        );

        let files_with_sources = vec![
            (
                "/repo/src/app.ts",
                r#"import { helper } from "@/shared/helper";"#,
            ),
            ("/repo/src/shared/helper.ts", r#"export const helper = 42;"#),
        ];

        let domain = DomainConfig {
            folders: Vec::new(),
            file_suffixes: Vec::new(),
        };
        let graph = build_import_graph_from_sources(&files_with_sources, &domain, Some(&tsconfig));

        let edge = graph
            .edges_from(Path::new("/repo/src/app.ts"))
            .next()
            .context("expected at least one edge")?;
        assert_eq!(edge.specifier, "@/shared/helper");
        assert_eq!(
            edge.resolved_target,
            Some(PathBuf::from("/repo/src/shared/helper.ts"))
        );
        Ok(())
    }

    #[test]
    fn resolves_aliased_import_when_first_alias_does_not_match() -> Result<()> {
        let tsconfig = TsConfig::new(
            PathBuf::from("/repo"),
            vec![
                ResolvedPathAlias {
                    pattern: "@components/*".into(),
                    prefix: "@components/".into(),
                    suffix: "".into(),
                    targets: vec![PathTargetPattern {
                        prefix: "src/components/".into(),
                        suffix: "".into(),
                    }],
                },
                ResolvedPathAlias {
                    pattern: "@/*".into(),
                    prefix: "@/".into(),
                    suffix: "".into(),
                    targets: vec![PathTargetPattern {
                        prefix: "src/".into(),
                        suffix: "".into(),
                    }],
                },
            ],
        );

        let files_with_sources = vec![
            (
                "/repo/src/app.ts",
                r#"import { helper } from "@/shared/helper";"#,
            ),
            ("/repo/src/shared/helper.ts", r#"export const helper = 42;"#),
        ];

        let domain = DomainConfig {
            folders: Vec::new(),
            file_suffixes: Vec::new(),
        };
        let graph = build_import_graph_from_sources(&files_with_sources, &domain, Some(&tsconfig));

        let edge = graph
            .edges_from(Path::new("/repo/src/app.ts"))
            .next()
            .context("expected at least one edge")?;
        assert_eq!(edge.specifier, "@/shared/helper");
        assert_eq!(
            edge.resolved_target,
            Some(PathBuf::from("/repo/src/shared/helper.ts"))
        );
        Ok(())
    }
}
