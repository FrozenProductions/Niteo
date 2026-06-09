mod analysis;
mod app;
mod baseline;
mod cache;
mod cli;
mod commands;
mod config;
mod directory_inventory;
mod discovery;
mod git;
mod ignore;
mod import_graph;
mod jsx;
mod report;
mod rule_adapters;
mod rule_documentation;
mod rules;
mod rules_runner;
mod syntax;
mod tsconfig;
mod watch;
mod workspace;

use anyhow::Result;
use std::process::ExitCode;

fn main() -> Result<ExitCode> {
    app::run()
}
