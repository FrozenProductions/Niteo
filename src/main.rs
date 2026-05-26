mod app;
mod cli;
mod config;
mod discovery;
mod git;
mod ignore;
mod jsx;
mod report;
mod rule_documentation;
mod rules;
mod syntax;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
