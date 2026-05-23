mod app;
mod cli;
mod config;
mod discovery;
mod report;
mod rules;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
