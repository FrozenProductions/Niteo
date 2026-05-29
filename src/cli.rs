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
    /// Project root to scan.
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    /// Limit scanning to this path.
    #[arg(long, global = true)]
    pub scope: Option<PathBuf>,

    /// Show every violation.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Scan changed TypeScript files only.
    #[arg(long, global = true)]
    pub git: bool,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Write output to a file.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Baseline file for suppressing existing violations.
    #[arg(long, global = true, default_value = "niteo-baseline.json")]
    pub baseline: PathBuf,

    /// Report suppressed violations and stale ignore directives.
    #[arg(long, global = true)]
    pub report_suppressions: bool,

    /// Re-run lint on file changes.
    #[arg(long, global = true)]
    pub watch: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create niteo.toml.
    Init,
    /// Manage the current violation baseline.
    Baseline {
        #[command(subcommand)]
        command: BaselineCommand,
    },
    /// Scan for structural issues.
    Lint,
    /// List rules and severities.
    Rules,
    /// Explain a rule.
    Explain { rule: String },
    /// Show project statistics.
    Stats,
    /// Visualize import graph.
    Graph,
}

#[derive(Debug, Subcommand)]
pub enum BaselineCommand {
    /// Write current violations to the baseline file.
    Create,
    /// Remove stale entries from the baseline file.
    Prune,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}
