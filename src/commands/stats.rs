use anyhow::{Result, bail};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::write_report;
use crate::config;
use crate::discovery;
use crate::import_graph;

pub fn show(
    workspace: &Path,
    root_override: Option<PathBuf>,
    scope_override: Option<PathBuf>,
    git_flag: bool,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope =
        scope_override.map(|scope| crate::analysis::resolve_path(&project_config.root, scope));

    let files = if git_flag {
        crate::analysis::resolve_changed_files(workspace)?
    } else {
        discovery::discover_files(
            &project_config.root,
            scan_scope.as_deref(),
            &project_config.gitignore,
        )?
    };

    let tests_config = project_config.structure.tests.clone();
    let tsconfig = crate::tsconfig::discover_and_parse(workspace)?;
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
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
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
        most_imported: Vec<FileCount>,
        highest_fanout: Vec<FileCount>,
    }

    #[derive(Serialize)]
    struct FileCount {
        path: String,
        count: usize,
    }

    let stats = Stats {
        files: graph.file_count(),
        import_edges: graph.edge_count(),
        unresolved_local_imports: graph.unresolved_count(),
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
