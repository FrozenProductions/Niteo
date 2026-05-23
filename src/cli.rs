use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "niteo",
    version,
    about = "Structural linter for TypeScript codebases"
)]
pub struct Cli {
    #[command(flatten)]
    pub options: CliOptions,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Args)]
pub struct CliOptions {
    /// Override the project root to scan.
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    /// Restrict the scan to a path prefix.
    #[arg(long, global = true)]
    pub scope: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate the initial config file.
    Init,
    /// Scan the project for structural issues.
    Lint,
}
