mod app;
mod baseline;
mod cli;
mod config;
mod discovery;
mod git;
mod ignore;
mod import_graph;
mod jsx;
mod report;
mod rule_documentation;
mod rules;
mod syntax;

use anyhow::Result;
use std::process::ExitCode;

fn main() -> Result<ExitCode> {
    app::run()
}
