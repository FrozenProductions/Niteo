use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;

use crate::config::validation::ConfigDiagnosticSeverity;

pub fn check(workspace: &Path) -> Result<ExitCode> {
    let source = read_config_source(workspace)?;
    let report = crate::config::validation::validate_config_source(&source);

    println!("{}", report.render_text());

    if report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == ConfigDiagnosticSeverity::Error)
    {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

pub fn print(workspace: &Path) -> Result<ExitCode> {
    let source = read_config_source(workspace)?;
    println!("{source}");
    Ok(ExitCode::SUCCESS)
}

fn read_config_source(workspace: &Path) -> Result<String> {
    let config_path = workspace.join(crate::config::defaults::CONFIG_FILE_NAME);
    if config_path.exists() {
        Ok(std::fs::read_to_string(&config_path)?)
    } else {
        Ok(crate::config::defaults::DEFAULT_CONFIG_SOURCE.to_owned())
    }
}
