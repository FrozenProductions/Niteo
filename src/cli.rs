use clap::{Args, Parser, Subcommand, ValueEnum};
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

    /// Print every violation instead of grouped, capped output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Scan only changed TypeScript files (skips prompt).
    #[arg(long, global = true)]
    pub git: bool,

    /// Report output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Write the report to a file instead of stdout.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate the initial config file.
    Init,
    /// Scan the project for structural issues.
    Lint,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}
