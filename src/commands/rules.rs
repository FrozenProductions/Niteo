use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::cli::{OutputFormat, PresetName};
use crate::commands::write_report;
use crate::config;
use crate::rule_documentation;

pub fn list(
    workspace: &Path,
    root_override: Option<PathBuf>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let rows = rule_documentation::configured_rules(&project_config);

    let rendered = match output_format {
        OutputFormat::Text => {
            let name_width = rows
                .iter()
                .map(|row| row.name.len())
                .max()
                .unwrap_or("rule".len());
            let cat_width = rows
                .iter()
                .map(|row| row.category.len())
                .max()
                .unwrap_or("category".len());

            let mut output = String::new();
            output.push_str(&format!(
                "{:<name_width$}  {:<cat_width$}  severity\n",
                "rule", "category"
            ));
            output.push_str(&format!(
                "{:-<name_width$}  {:-<cat_width$}  --------\n",
                "", ""
            ));
            for row in &rows {
                output.push_str(&format!(
                    "{:<name_width$}  {:<cat_width$}  {}\n",
                    row.name,
                    row.category,
                    row.severity.as_str()
                ));
            }
            output
        }
        OutputFormat::Json => rule_documentation::render_rules_json(&rows)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'rules' command"),
        OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'rules' command"),
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

pub fn list_with_preset(
    workspace: &Path,
    preset_name: PresetName,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<()> {
    let name = match preset_name {
        PresetName::Balanced => "balanced",
        PresetName::Strict => "strict",
        PresetName::Migration => "migration",
        PresetName::React => "react",
        PresetName::Library => "library",
        PresetName::NoBarrels => "no-barrels",
    };

    let preset = config::presets::PresetName::from_str(name)
        .ok_or_else(|| anyhow::anyhow!("unknown preset: {name}"))?;
    let source = config::presets::default_config_for_preset(preset);
    let raw: config::raw::RawConfig = toml::from_str(source)?;
    let project_config = raw.into_project_config(std::env::current_dir()?);
    let rows = rule_documentation::configured_rules(&project_config);

    let rendered = match output_format {
        OutputFormat::Text => {
            let name_width = rows
                .iter()
                .map(|row| row.name.len())
                .max()
                .unwrap_or("rule".len());
            let cat_width = rows
                .iter()
                .map(|row| row.category.len())
                .max()
                .unwrap_or("category".len());

            let mut output = String::new();
            output.push_str(&format!(
                "{:<name_width$}  {:<cat_width$}  severity\n",
                "rule", "category"
            ));
            output.push_str(&format!(
                "{:-<name_width$}  {:-<cat_width$}  --------\n",
                "", ""
            ));
            for row in &rows {
                output.push_str(&format!(
                    "{:<name_width$}  {:<cat_width$}  {}\n",
                    row.name,
                    row.category,
                    row.severity.as_str()
                ));
            }
            output
        }
        OutputFormat::Json => rule_documentation::render_rules_json(&rows)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'rules' command"),
        OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'rules' command"),
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}

pub fn explain(
    workspace: &Path,
    root_override: Option<PathBuf>,
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
    rule_name: &str,
) -> Result<()> {
    let project_config = config::ProjectConfig::resolve(workspace, root_override)?;
    let explanation = rule_documentation::explain_rule(rule_name, &project_config)?;

    let rendered = match output_format {
        OutputFormat::Text => rule_documentation::render_explanation_text(&explanation),
        OutputFormat::Json => rule_documentation::render_explanation_json(&explanation)?,
        OutputFormat::Sarif => bail!("SARIF format is not supported for the 'explain' command"),
        OutputFormat::Ndjson => bail!("NDJSON format is not supported for the 'explain' command"),
    };

    write_report(workspace, output_path, &rendered)?;

    Ok(())
}
