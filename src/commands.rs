use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub mod baseline;
pub mod config;
pub mod graph;
pub mod lint;
pub mod rules;
pub mod stats;

pub fn write_report(
    workspace: &Path,
    output_path: Option<PathBuf>,
    rendered_report: &str,
) -> Result<()> {
    let Some(output_path) = output_path else {
        println!("{rendered_report}");
        return Ok(());
    };

    let resolved_output_path = crate::analysis::resolve_path(workspace, output_path);
    if let Some(parent) = resolved_output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(&resolved_output_path, rendered_report)
        .with_context(|| format!("failed to write {}", resolved_output_path.display()))?;

    Ok(())
}
