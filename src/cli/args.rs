use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "vacuum",
    about = "Enumerate artifacts and emit deterministic JSONL manifests",
    long_about = None
)]
#[command(args_conflicts_with_subcommands = true)]
#[command(disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Root directories to scan
    #[arg(value_name = "ROOT")]
    pub roots: Vec<PathBuf>,

    /// Include only files matching this glob (repeatable)
    #[arg(long, action = ArgAction::Append, value_name = "GLOB")]
    pub include: Vec<String>,

    /// Exclude files matching this glob (repeatable)
    #[arg(long, action = ArgAction::Append, value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Do not follow symlinks
    #[arg(long)]
    pub no_follow: bool,

    /// Suppress witness ledger recording
    #[arg(long)]
    pub no_witness: bool,

    /// Print operator manifest (operator.v0 JSON) and exit
    #[arg(long)]
    pub describe: bool,

    /// Print output schema and exit
    #[arg(long)]
    pub schema: bool,

    /// Show scan progress on stderr
    #[arg(long)]
    pub progress: bool,

    /// Print version and exit
    #[arg(long)]
    pub version: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Witness {
        #[command(subcommand)]
        action: WitnessAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum WitnessAction {
    /// Search witness ledger records
    Query {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Include records on or after this ISO-8601 timestamp
        #[arg(long)]
        since: Option<String>,
        /// Include records before this ISO-8601 timestamp
        #[arg(long)]
        until: Option<String>,
        /// Filter by outcome (e.g. SCAN_COMPLETE, REFUSAL)
        #[arg(long)]
        outcome: Option<String>,
        /// Filter by input content hash
        #[arg(long = "input-hash")]
        input_hash: Option<String>,
        /// Maximum number of records to return
        #[arg(long)]
        limit: Option<usize>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show the most recent witness ledger record
    Last {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Count matching witness ledger records
    Count {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Include records on or after this ISO-8601 timestamp
        #[arg(long)]
        since: Option<String>,
        /// Include records before this ISO-8601 timestamp
        #[arg(long)]
        until: Option<String>,
        /// Filter by outcome (e.g. SCAN_COMPLETE, REFUSAL)
        #[arg(long)]
        outcome: Option<String>,
        /// Filter by input content hash
        #[arg(long = "input-hash")]
        input_hash: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn parse() -> Result<Cli, clap::Error> {
    Cli::try_parse()
}
