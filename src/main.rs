mod app;
mod cli;
mod config;
mod discovery;
mod git;
mod ignore;
mod jsx;
mod report;
mod rules;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
