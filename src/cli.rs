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

    /// Enable caching of analysis results.
    #[arg(long, global = true)]
    pub cache: bool,

    /// Disable caching.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Clear the cache before running.
    #[arg(long, global = true)]
    pub clear_cache: bool,

    /// Minimum severity that causes lint to fail.
    #[arg(long, global = true, value_enum, default_value_t = FailOn::Any)]
    pub fail_on: FailOn,

    /// Fail when nested niteo.toml files are found inside the scan scope.
    #[arg(long, global = true)]
    pub deny_child_configs: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create niteo.toml.
    Init {
        /// Use a named preset instead of the full default config.
        #[arg(long, value_enum)]
        preset: Option<PresetName>,
    },
    /// Manage the current violation baseline.
    Baseline {
        #[command(subcommand)]
        command: BaselineCommand,
    },
    /// Scan for structural issues.
    Lint,
    /// List rules and severities.
    Rules {
        /// Use a named preset to show what would be configured.
        #[arg(long, value_enum)]
        preset: Option<PresetName>,
    },
    /// Explain a rule.
    Explain { rule: String },
    /// Show project statistics.
    Stats,
    /// Visualize import graph.
    Graph,
    /// Validate and inspect configuration.
    #[command(name = "config")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum BaselineCommand {
    /// Write current violations to the baseline file.
    Create,
    /// Remove stale entries from the baseline file.
    Prune,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Validate the config file and report diagnostics.
    Check,
    /// Print the resolved config.
    Print,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PresetName {
    #[value(name = "balanced")]
    Balanced,
    #[value(name = "strict")]
    Strict,
    #[value(name = "migration")]
    Migration,
    #[value(name = "react")]
    React,
    #[value(name = "library")]
    Library,
    #[value(name = "no-barrels")]
    NoBarrels,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FailOn {
    Error,
    Warn,
    Any,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
    Ndjson,
}
