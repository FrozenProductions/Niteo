use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::allocator::with_reusable_allocator;
use crate::analysis::{self, AnalysisOptions};
use crate::baseline;
use crate::config::rule_metadata::FixCapability;
use crate::fix;

pub struct FixOptions {
    pub dry_run: bool,
    pub git_selection: Option<crate::git::GitSelection>,
    pub baseline_path: PathBuf,
    pub deny_child_configs: bool,
}

pub fn fix_workspace(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    options: FixOptions,
) -> Result<()> {
    let collected = analysis::collect(
        workspace,
        AnalysisOptions {
            root_override: root_override.clone(),
            scope_override,
            git_selection: options.git_selection,
            prompt_for_changed_files: false,
            deny_child_configs: options.deny_child_configs,
            cache_enabled: false,
            clear_cache: false,
        },
    )?;

    let resolved_baseline_path = crate::analysis::resolve_path(workspace, options.baseline_path);
    let violations = match baseline::read_baseline(&resolved_baseline_path)? {
        Some(ref existing) => {
            existing.filter_new_violations(&collected.project_root, collected.violations)
        }
        None => collected.violations,
    };

    let fixable_files: HashSet<PathBuf> = violations
        .iter()
        .filter(|violation| {
            crate::config::rule_metadata::rule_by_id(violation.rule)
                .is_some_and(|metadata| metadata.fix_capability != FixCapability::None)
        })
        .map(|violation| violation.file.clone())
        .collect();

    if fixable_files.is_empty() {
        println!("No fixable violations found.");
        return Ok(());
    }

    let config_set = crate::config::ConfigSet::resolve(
        workspace,
        crate::config::ConfigSetOptions {
            root_override: root_override.clone(),
            scan_scope: None,
            deny_child_configs: options.deny_child_configs,
        },
    )?;

    let mut all_fixes = Vec::new();

    for file in &fixable_files {
        let config_for_file = config_set.config_for_file(file);
        let rules = crate::rules_runner::build_file_rules(
            &config_for_file.rules,
            &config_for_file.structure,
            &config_for_file.architecture,
            collected.import_graph.clone(),
            collected.workspace.clone(),
        );

        let any_fixable = rules
            .iter()
            .any(|rule| rule.severity().is_enabled() && rule.supports_fix());

        if !any_fixable {
            continue;
        }

        let source = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;

        let needs_ast = rules
            .iter()
            .any(|rule| rule.severity().is_enabled() && rule.needs_ast() && rule.supports_fix());

        let single_file = [file.clone()];
        let type_location_style =
            crate::rules::TypeLocationStyle::detect(&single_file, &config_for_file.structure.types);

        let file_fixes = with_reusable_allocator(|allocator| {
            let line_index = crate::syntax::LineIndex::new(&source);
            let parse_result = if needs_ast {
                match crate::syntax::source_type_from_path(file) {
                    Some(source_type) => {
                        let parser_return =
                            oxc_parser::Parser::new(allocator, &source, source_type).parse();
                        if parser_return.panicked {
                            None
                        } else {
                            Some(parser_return.program)
                        }
                    }
                    None => None,
                }
            } else {
                None
            };

            let ctx = crate::rules::FileContext {
                file,
                source: &source,
                program: parse_result.as_ref(),
                line_index: &line_index,
                import_graph: collected.import_graph.clone(),
                workspace: collected.workspace.clone(),
                type_location_style,
            };
            crate::fix::collect_fixes(&ctx, &rules)
        });

        all_fixes.extend(file_fixes);
    }

    if all_fixes.is_empty() {
        println!("No fixes to apply.");
        return Ok(());
    }

    if options.dry_run {
        fix::report_dry_run(&all_fixes);
        return Ok(());
    }

    let outcome = fix::apply_fixes(
        all_fixes,
        fix::ApplyFixOptions {
            dry_run: false,
            validate_parse: true,
        },
    )?;

    for file in &outcome.fixed_files {
        println!("Fixed {}", file.display());
    }

    if outcome.rejected_overlapping > 0 {
        eprintln!(
            "warning: rejected {} overlapping edits",
            outcome.rejected_overlapping
        );
    }
    if outcome.rejected_stale > 0 {
        eprintln!(
            "warning: rejected {} edits due to stale source",
            outcome.rejected_stale
        );
    }
    if outcome.rejected_invalid > 0 {
        eprintln!(
            "warning: rejected {} invalid edits",
            outcome.rejected_invalid
        );
    }
    if outcome.rejected_parse > 0 {
        eprintln!(
            "warning: rejected {} edits because fixed source would not parse",
            outcome.rejected_parse
        );
    }

    println!("Fixed {} file(s).", outcome.fixed_files.len());

    if let Some(existing_baseline) = baseline::read_baseline(&resolved_baseline_path)? {
        let result = existing_baseline.prune(&collected.project_root, &[]);
        if result.removed_count > 0 {
            baseline::write_baseline(&resolved_baseline_path, &result.baseline)?;
            println!("Pruned {} stale baseline entries", result.removed_count);
        }
    }

    Ok(())
}
