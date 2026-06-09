pub mod build;
pub mod extract;
pub mod helpers;
pub mod model;
pub mod resolver;
pub(crate) mod serialization;

pub use build::{build_import_graph, build_import_graph_with_cache};
pub use model::{ImportEdge, ImportGraph, ImportKind};

#[cfg(test)]
pub use build::build_import_graph_from_sources;

impl ImportGraph {
    pub fn format_dot(&self) -> String {
        serialization::GraphFormatter::new(ImportGraph {
            files: self.files.clone(),
            edges: self.edges.clone(),
        })
        .to_dot()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::config::structure::DomainConfig;
    use crate::import_graph::helpers::{
        extensionless, is_barrel_file, is_relative_specifier, normalize_path,
    };
    use crate::import_graph::resolver::ImportResolverIndex;
    use crate::tsconfig::{PathTargetPattern, ResolvedPathAlias, TsConfig};

    use super::*;

    fn resolve_import_specifier(
        source_file: &Path,
        specifier: &str,
        all_files: &[PathBuf],
    ) -> Option<PathBuf> {
        ImportResolverIndex::new(all_files, None).resolve(source_file, specifier)
    }

    #[test]
    fn identifies_relative_specifiers() {
        assert!(is_relative_specifier("./foo"));
        assert!(is_relative_specifier("../bar"));
        assert!(is_relative_specifier("/absolute"));
        assert!(!is_relative_specifier("lodash"));
        assert!(!is_relative_specifier("@scope/package"));
    }

    #[test]
    fn normalizes_paths_correctly() {
        let path = Path::new("src/components/../utils/./helper");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("src/utils/helper"));
    }

    #[test]
    fn resolves_relative_import() {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));
    }

    #[test]
    fn resolves_import_with_extension() {
        let files = vec![PathBuf::from("src/a.ts"), PathBuf::from("src/b.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b.ts", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));
    }

    #[test]
    fn resolves_directory_import_to_barrel() {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components/index.ts"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./components", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/components/index.ts")));
    }

    #[test]
    fn returns_none_for_external_import() {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "lodash", &files);
        assert_eq!(resolved, None);
    }

    #[test]
    fn returns_none_for_unresolved_import() {
        let files = vec![PathBuf::from("src/a.ts")];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./nonexistent", &files);
        assert_eq!(resolved, None);
    }

    #[test]
    fn resolves_extensionless_duplicate_deterministically() {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/b.ts"),
            PathBuf::from("src/b.tsx"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./b", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/b.ts")));

        let files_reversed = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/b.tsx"),
            PathBuf::from("src/b.ts"),
        ];
        let resolved_reversed =
            resolve_import_specifier(Path::new("src/a.ts"), "./b", &files_reversed);
        assert_eq!(resolved_reversed, Some(PathBuf::from("src/b.tsx")));
    }

    #[test]
    fn resolves_directory_barrel_duplicate_deterministically() {
        let files = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components.ts"),
            PathBuf::from("src/components/index.ts"),
        ];
        let resolved = resolve_import_specifier(Path::new("src/a.ts"), "./components", &files);
        assert_eq!(resolved, Some(PathBuf::from("src/components.ts")));

        let files_reversed = vec![
            PathBuf::from("src/a.ts"),
            PathBuf::from("src/components/index.ts"),
            PathBuf::from("src/components.ts"),
        ];
        let resolved_reversed =
            resolve_import_specifier(Path::new("src/a.ts"), "./components", &files_reversed);
        assert_eq!(
            resolved_reversed,
            Some(PathBuf::from("src/components/index.ts"))
        );
    }

    #[test]
    fn resolves_aliased_import_through_graph() {
        let tsconfig = TsConfig {
            base_url: PathBuf::from("/repo"),
            aliases: vec![ResolvedPathAlias {
                pattern: "@/*".into(),
                prefix: "@/".into(),
                suffix: "".into(),
                targets: vec![PathTargetPattern {
                    prefix: "src/".into(),
                    suffix: "".into(),
                }],
            }],
        };

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
            .unwrap();
        assert_eq!(edge.specifier, "@/shared/helper");
        assert_eq!(
            edge.resolved_target,
            Some(PathBuf::from("/repo/src/shared/helper.ts"))
        );
    }

    #[test]
    fn resolves_aliased_import_when_first_alias_does_not_match() {
        let tsconfig = TsConfig {
            base_url: PathBuf::from("/repo"),
            aliases: vec![
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
        };

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
            .unwrap();
        assert_eq!(edge.specifier, "@/shared/helper");
        assert_eq!(
            edge.resolved_target,
            Some(PathBuf::from("/repo/src/shared/helper.ts"))
        );
    }
}
