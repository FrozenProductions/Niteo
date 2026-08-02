use anyhow::{Result, bail};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::write_report;
use crate::config;
use crate::discovery;
use crate::history::{self, HistoryEntry};
use crate::import_graph;

pub fn show(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_selection: Option<crate::git::GitSelection>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
    show_history: bool,
) -> Result<()> {
    if show_history {
        let entries = history::read_entries(workspace)?;
        let rendered = match output_format {
            OutputFormat::Text => render_history_text(&entries),
            OutputFormat::Json => render_history_json(&entries)?,
            OutputFormat::Sarif => bail!("SARIF format is not supported for the 'stats' command"),
            OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'stats' command"),
            OutputFormat::Markdown => {
                bail!("Markdown format is not supported for the 'stats' command")
            }
        };

        write_report(workspace, output_path, &rendered)?;
        return Ok(());
    }

    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope = scope_override
        .map(|scope| crate::analysis::resolve_scope_path(&project_config.root, scope))
        .transpose()?;

    let tsconfig = crate::tsconfig::discover_and_parse(workspace)?;

    let files = if let Some(selection) = git_selection.as_ref() {
        crate::analysis::resolve_changed_files(
            workspace,
            selection,
            &project_config.root,
            scan_scope.as_deref(),
            &project_config.gitignore,
            tsconfig.as_ref(),
        )?
    } else {
        discovery::discover_files(
            &project_config.root,
            scan_scope.as_deref(),
            &project_config.gitignore,
            tsconfig.as_ref(),
        )?
    };

    let tests_config = project_config.structure.tests.clone();
    let graph = import_graph::build_import_graph(
        &files,
        |file| tests_config.matches_file(file),
        tsconfig.as_ref(),
    )?;

    let rendered = match output_format {
        OutputFormat::Text => render_text(&graph),
        OutputFormat::Json => render_json(&graph)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'stats' command"),
        OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'stats' command"),
        OutputFormat::Markdown => {
            bail!("Markdown format is not supported for the 'stats' command")
        }
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

fn render_history_text(entries: &[HistoryEntry]) -> String {
    let mut output = String::new();
    output.push_str("Health Score History\n");
    output.push_str("====================\n\n");

    if entries.is_empty() {
        output.push_str("No history entries found.\n");
        return output;
    }

    output.push_str(
        "Timestamp                    Score  Files  Violations  Errors  Warnings  Infos\n",
    );
    for entry in entries {
        output.push_str(&format!(
            "{:<28} {:>5} {:>6} {:>11} {:>7} {:>9} {:>6}\n",
            entry.timestamp,
            entry.health_score,
            entry.files,
            entry.violations,
            entry.errors,
            entry.warnings,
            entry.infos
        ));
    }

    output
}

fn render_history_json(entries: &[HistoryEntry]) -> Result<String> {
    Ok(serde_json::to_string_pretty(entries)?)
}

fn render_text(graph: &import_graph::ImportGraph) -> String {
    let mut output = String::new();
    output.push_str("Project Statistics\n");
    output.push_str("==================\n\n");
    output.push_str(&format!("Files: {}\n", graph.file_count()));
    output.push_str(&format!("Import edges: {}\n", graph.edge_count()));
    output.push_str(&format!(
        "Unresolved local imports: {}\n",
        graph.unresolved_count()
    ));
    let breakdown = graph.unresolved_by_kind();
    output.push_str(&format!("  relative: {}\n", breakdown.relative));
    output.push_str(&format!("  alias: {}\n", breakdown.alias));
    output.push_str(&format!("  package: {}\n", breakdown.package));
    output.push('\n');

    let most_imported = graph.most_imported_files(5);
    if !most_imported.is_empty() {
        output.push_str("Most imported files:\n");
        for (path, count) in &most_imported {
            output.push_str(&format!("  {} ({})\n", path.display(), count));
        }
        output.push('\n');
    }

    let highest_fanout = graph.highest_fanout_files(5);
    if !highest_fanout.is_empty() {
        output.push_str("Highest fan-out files:\n");
        for (path, count) in &highest_fanout {
            output.push_str(&format!("  {} ({})\n", path.display(), count));
        }
    }

    output
}

fn render_json(graph: &import_graph::ImportGraph) -> Result<String> {
    #[derive(Serialize)]
    struct Stats {
        files: usize,
        import_edges: usize,
        unresolved_local_imports: usize,
        unresolved_imports_by_kind: UnresolvedByKind,
        most_imported: Vec<FileCount>,
        highest_fanout: Vec<FileCount>,
    }

    #[derive(Serialize)]
    struct UnresolvedByKind {
        relative: usize,
        alias: usize,
        package: usize,
    }

    #[derive(Serialize)]
    struct FileCount {
        path: String,
        count: usize,
    }

    let breakdown = graph.unresolved_by_kind();
    let stats = Stats {
        files: graph.file_count(),
        import_edges: graph.edge_count(),
        unresolved_local_imports: graph.unresolved_count(),
        unresolved_imports_by_kind: UnresolvedByKind {
            relative: breakdown.relative,
            alias: breakdown.alias,
            package: breakdown.package,
        },
        most_imported: graph
            .most_imported_files(5)
            .into_iter()
            .map(|(path, count)| FileCount {
                path: path.display().to_string(),
                count,
            })
            .collect(),
        highest_fanout: graph
            .highest_fanout_files(5)
            .into_iter()
            .map(|(path, count)| FileCount {
                path: path.display().to_string(),
                count,
            })
            .collect(),
    };

    Ok(serde_json::to_string_pretty(&stats)?)
}
