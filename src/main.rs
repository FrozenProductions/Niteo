mod app;
mod baseline;
mod cli;
mod config;
mod directory_inventory;
mod discovery;
mod git;
mod ignore;
mod import_graph;
mod jsx;
mod report;
mod rule_documentation;
mod rules;
mod syntax;
mod tsconfig;
mod watch;

use anyhow::Result;
use std::process::ExitCode;

fn main() -> Result<ExitCode> {
    app::run()
}
