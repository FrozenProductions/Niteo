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
    git_selection: Option<crate::git::GitSelection>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let scan_scope = scope_override
        .map(|scope| crate::analysis::resolve_scope_path(&project_config.root, scope))
        .transpose()?;

    let tsconfig = crate::tsconfig::discover_and_parse(workspace)?;

    let files = if let Some(selection) = git_selection.as_ref() {
        crate::analysis::resolve_changed_files(workspace, selection)?
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
        OutputFormat::Text => graph.format_dot(),
        OutputFormat::Json => render_json(&graph)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'graph' command"),
        OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'graph' command"),
        OutputFormat::Markdown => {
            bail!("Markdown format is not supported for the 'graph' command")
        }
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

fn render_json(graph: &import_graph::ImportGraph) -> Result<String> {
    #[derive(Serialize)]
    struct GraphData {
        nodes: Vec<NodeData>,
        edges: Vec<EdgeData>,
    }

    #[derive(Serialize)]
    struct NodeData {
        path: String,
        is_barrel: bool,
        is_test: bool,
    }

    #[derive(Serialize)]
    struct EdgeData {
        source: String,
        target: String,
        specifier: String,
        kind: String,
    }

    let nodes: Vec<NodeData> = graph
        .iter_files()
        .map(|(path, node)| NodeData {
            path: path.display().to_string(),
            is_barrel: node.is_barrel,
            is_test: node.is_test,
        })
        .collect();

    let edges: Vec<EdgeData> = graph
        .edges()
        .iter()
        .filter_map(|edge| {
            edge.resolved_target.as_ref().map(|target| EdgeData {
                source: edge.source_file.display().to_string(),
                target: target.display().to_string(),
                specifier: edge.specifier.clone(),
                kind: format!("{:?}", edge.kind),
            })
        })
        .collect();

    let graph_data = GraphData { nodes, edges };

    Ok(serde_json::to_string_pretty(&graph_data)?)
}
