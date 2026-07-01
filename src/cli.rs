use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::str::FromStr;

use crate::config::FailureThreshold;
use crate::git::GitSelection;

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

    /// Scan changed TypeScript files only. Optionally pass a revision range like `main..HEAD`.
    #[arg(
        long,
        global = true,
        value_name = "RANGE",
        num_args = 0..=1,
        default_missing_value = "",
        conflicts_with_all = ["git_staged", "git_unstaged"],
    )]
    pub git: Option<String>,

    /// Scan only staged TypeScript changes (index vs HEAD).
    #[arg(long, global = true, conflicts_with = "git_unstaged")]
    pub git_staged: bool,

    /// Scan only unstaged TypeScript changes (working tree vs index).
    #[arg(long, global = true)]
    pub git_unstaged: bool,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Write output to a file.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Baseline file for suppressing existing violations.
    #[arg(long, global = true)]
    pub baseline: Option<PathBuf>,

    /// Report suppressed violations and stale ignore directives.
    #[arg(long, global = true)]
    pub report_suppressions: bool,

    /// Re-run lint on file changes.
    #[arg(long, global = true)]
    pub watch: bool,

    /// Watch debounce duration in milliseconds.
    #[arg(long, global = true, default_value_t = 300)]
    pub watch_debounce_ms: u64,

    /// Enable caching of analysis results.
    #[arg(long, global = true)]
    pub cache: bool,

    /// Disable caching.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Clear the cache before running.
    #[arg(long, global = true)]
    pub clear_cache: bool,

    /// Minimum severity that causes lint to fail. Defaults to `any`, or to the
    /// `[fail-on].default` value from config when present.
    #[arg(long, global = true, value_enum)]
    pub fail_on: Option<FailOn>,

    /// Override the failure threshold for one rule. Repeatable.
    /// Format: `--fail-on-rule <rule>=<severity>` where severity is `error`, `warn`, or `any`.
    #[arg(long, global = true, value_name = "RULE=SEVERITY")]
    pub fail_on_rule: Vec<FailOnOverride>,

    /// Override the failure threshold for one rule category. Repeatable.
    /// Format: `--fail-on-category <category>=<severity>` where category is one of
    /// `typescript`, `hygiene`, `exports`, `files`, `domain`, or `imports`.
    #[arg(long, global = true, value_name = "CATEGORY=SEVERITY")]
    pub fail_on_category: Vec<FailOnOverride>,

    /// Fail when nested niteo.toml files are found inside the scan scope.
    #[arg(long, global = true)]
    pub deny_child_configs: bool,
}

impl CliOptions {
    pub fn git_selection(&self) -> Option<GitSelection> {
        if self.git_staged {
            return Some(GitSelection::Staged);
        }
        if self.git_unstaged {
            return Some(GitSelection::Unstaged);
        }
        match self.git.as_deref() {
            None => None,
            Some("") => Some(GitSelection::WorkingTree),
            Some(range) => Some(GitSelection::Range(range.to_string())),
        }
    }
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
    Lint {
        /// Apply fixes after linting.
        #[arg(long)]
        fix: bool,
        /// Write a history entry for this lint run, even when disabled in config.
        #[arg(long)]
        history: bool,
    },
    /// Apply autofixes for rules that support them.
    Fix {
        /// Preview fixes without writing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// List rules and severities.
    Rules {
        /// Use a named preset to show what would be configured.
        #[arg(long, value_enum)]
        preset: Option<PresetName>,
    },
    /// Explain a rule.
    Explain { rule: String },
    /// Show project statistics.
    Stats {
        /// Show health score history.
        #[arg(long)]
        history: bool,
    },
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
    Markdown,
}

#[derive(Debug, Clone)]
pub struct FailOnOverride {
    pub target: String,
    pub threshold: FailOn,
}

impl FromStr for FailOnOverride {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (target, threshold) = value
            .split_once('=')
            .ok_or_else(|| format!("expected <target>=<severity>, got '{value}'"))?;

        let threshold = threshold
            .parse::<FailOn>()
            .map_err(|error| format!("invalid severity '{threshold}': {error}"))?;

        Ok(Self {
            target: target.to_string(),
            threshold,
        })
    }
}

impl FromStr for FailOn {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "any" => Ok(Self::Any),
            _ => Err(format!(
                "unknown fail-on threshold '{value}'; use 'error', 'warn', or 'any'"
            )),
        }
    }
}

impl From<FailOn> for FailureThreshold {
    fn from(fail_on: FailOn) -> Self {
        match fail_on {
            FailOn::Error => FailureThreshold::Error,
            FailOn::Warn => FailureThreshold::Warn,
            FailOn::Any => FailureThreshold::Any,
        }
    }
}
